use rusqlite::{Connection, OptionalExtension, params};

use crate::model::{Caller, Role, StoreError, StoreResult, validate_slug};
use crate::store::users::{require_admin, user_id_by_handle};
use crate::store::{Store, audit, project_id_by_slug};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Project {
  pub id: i64,
  pub slug: String,
  pub name: String,
  pub repository_url: Option<String>,
  pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Member {
  pub handle: String,
  pub role: Role,
}

pub fn validate_repository_url(url: &str) -> StoreResult<()> {
  let rest = url
    .strip_prefix("https://")
    .or_else(|| url.strip_prefix("http://"));
  let host = rest.and_then(|r| r.split('/').next()).unwrap_or_default();
  if host.is_empty() || url.chars().any(|c| c.is_whitespace() || c.is_control()) {
    return Err(StoreError::Validation(
      "repository URL must be an absolute http or https URL with a host".into(),
    ));
  }
  Ok(())
}

pub fn project_in(conn: &Connection, project_id: i64) -> StoreResult<Project> {
  Ok(conn.query_row(
    "SELECT id, slug, name, repository_url, created_at FROM projects WHERE id = ?1",
    [project_id],
    |r| {
      Ok(Project {
        id: r.get(0)?,
        slug: r.get(1)?,
        name: r.get(2)?,
        repository_url: r.get(3)?,
        created_at: r.get(4)?,
      })
    },
  )?)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectSummary {
  pub slug: String,
  pub name: String,
  pub role: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Access {
  pub project_id: i64,
  pub role: Option<Role>,
  pub is_admin: bool,
}

impl Access {
  pub fn can_write(&self) -> bool {
    self.role.is_some_and(Role::can_write)
  }

  pub fn is_owner(&self) -> bool {
    self.role == Some(Role::Owner)
  }

  pub fn can_manage_members(&self) -> bool {
    self.is_admin || self.is_owner()
  }
}

pub fn role_of(conn: &Connection, project_id: i64, user_id: i64) -> StoreResult<Option<Role>> {
  let role: Option<String> = conn
    .query_row(
      "SELECT role FROM memberships WHERE project_id = ?1 AND user_id = ?2",
      params![project_id, user_id],
      |r| r.get(0),
    )
    .optional()?;
  role.map(|r| r.parse()).transpose()
}

pub fn access_in(conn: &Connection, caller: &Caller, slug: &str) -> StoreResult<Access> {
  match project_id_by_slug(conn, slug)? {
    None if caller.is_admin => Err(StoreError::NotFound(format!("project {slug:?}"))),
    None => Err(StoreError::Forbidden),
    Some(project_id) => {
      let role = role_of(conn, project_id, caller.user_id)?;
      if role.is_none() && !caller.is_admin {
        return Err(StoreError::Forbidden);
      }
      Ok(Access {
        project_id,
        role,
        is_admin: caller.is_admin,
      })
    }
  }
}

impl Store {
  pub fn create_project(&self, actor: &Caller, slug: &str, name: &str) -> StoreResult<Project> {
    require_admin(Some(actor))?;
    validate_slug(slug)?;
    self.with_tx(|tx| {
      if project_id_by_slug(tx, slug)?.is_some() {
        return Err(StoreError::Validation(format!(
          "project slug {slug:?} already exists"
        )));
      }
      tx.execute(
        "INSERT INTO projects (slug, name) VALUES (?1, ?2)",
        params![slug, name],
      )?;
      let id = tx.last_insert_rowid();
      audit(tx, actor.user_id, Some(id), "project.create", slug)?;
      let created_at: String =
        tx.query_row("SELECT created_at FROM projects WHERE id = ?1", [id], |r| {
          r.get(0)
        })?;
      Ok(Project {
        id,
        slug: slug.to_string(),
        name: name.to_string(),
        repository_url: None,
        created_at,
      })
    })
  }

  pub fn set_member(
    &self,
    actor: &Caller,
    project_slug: &str,
    handle: &str,
    role: Role,
  ) -> StoreResult<()> {
    self.with_tx(|tx| {
      let access = access_in(tx, actor, project_slug)?;
      if !access.can_manage_members() {
        return Err(StoreError::Forbidden);
      }
      let user_id = user_id_by_handle(tx, handle)?;
      tx.execute(
        "INSERT INTO memberships (project_id, user_id, role) VALUES (?1, ?2, ?3)
         ON CONFLICT(project_id, user_id) DO UPDATE SET role = excluded.role",
        params![access.project_id, user_id, role.as_str()],
      )?;
      audit(
        tx,
        actor.user_id,
        Some(access.project_id),
        "member.set",
        &format!("{handle}:{role}"),
      )
    })
  }

  pub fn remove_member(&self, actor: &Caller, project_slug: &str, handle: &str) -> StoreResult<()> {
    self.with_tx(|tx| {
      let access = access_in(tx, actor, project_slug)?;
      if !access.can_manage_members() {
        return Err(StoreError::Forbidden);
      }
      let user_id = user_id_by_handle(tx, handle)?;
      let removed = tx.execute(
        "DELETE FROM memberships WHERE project_id = ?1 AND user_id = ?2",
        params![access.project_id, user_id],
      )?;
      if removed == 0 {
        return Err(StoreError::NotFound(format!(
          "membership of {handle:?} in project {project_slug:?}"
        )));
      }
      audit(
        tx,
        actor.user_id,
        Some(access.project_id),
        "member.remove",
        handle,
      )
    })
  }

  pub fn list_projects(&self, caller: &Caller) -> StoreResult<Vec<ProjectSummary>> {
    self.with_conn(|conn| {
      let sql = if caller.is_admin {
        "SELECT p.slug, p.name, COALESCE(m.role, 'admin')
         FROM projects p
         LEFT JOIN memberships m ON m.project_id = p.id AND m.user_id = ?1
         ORDER BY p.slug"
      } else {
        "SELECT p.slug, p.name, m.role
         FROM projects p
         JOIN memberships m ON m.project_id = p.id AND m.user_id = ?1
         ORDER BY p.slug"
      };
      let mut stmt = conn.prepare(sql)?;
      let rows = stmt.query_map([caller.user_id], |r| {
        Ok(ProjectSummary {
          slug: r.get(0)?,
          name: r.get(1)?,
          role: r.get(2)?,
        })
      })?;
      rows
        .collect::<Result<Vec<_>, _>>()
        .map_err(StoreError::from)
    })
  }

  pub fn access(&self, caller: &Caller, slug: &str) -> StoreResult<Access> {
    self.with_conn(|conn| access_in(conn, caller, slug))
  }

  pub fn project(&self, caller: &Caller, slug: &str) -> StoreResult<Project> {
    self.with_conn(|conn| {
      let access = access_in(conn, caller, slug)?;
      project_in(conn, access.project_id)
    })
  }

  pub fn list_members(&self, caller: &Caller, slug: &str) -> StoreResult<Vec<Member>> {
    self.with_conn(|conn| {
      let access = access_in(conn, caller, slug)?;
      let mut stmt = conn.prepare(
        "SELECT u.handle, m.role FROM memberships m JOIN users u ON u.id = m.user_id
         WHERE m.project_id = ?1
         ORDER BY CASE m.role WHEN 'owner' THEN 0 WHEN 'editor' THEN 1 ELSE 2 END, u.handle",
      )?;
      let rows = stmt.query_map([access.project_id], |r| {
        Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
      })?;
      rows
        .map(|row| {
          let (handle, role) = row?;
          Ok(Member {
            handle,
            role: role.parse()?,
          })
        })
        .collect()
    })
  }

  pub fn set_repository_url(
    &self,
    actor: &Caller,
    slug: &str,
    url: Option<&str>,
  ) -> StoreResult<()> {
    if let Some(url) = url {
      validate_repository_url(url)?;
    }
    self.with_tx(|tx| {
      let access = access_in(tx, actor, slug)?;
      if !access.can_manage_members() {
        return Err(StoreError::Forbidden);
      }
      tx.execute(
        "UPDATE projects SET repository_url = ?2 WHERE id = ?1",
        params![access.project_id, url],
      )?;
      let target = match url {
        Some(_) => slug.to_string(),
        None => format!("{slug} cleared"),
      };
      audit(
        tx,
        actor.user_id,
        Some(access.project_id),
        "project.repo",
        &target,
      )
    })
  }
}
