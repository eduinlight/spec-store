use rusqlite::{Connection, OptionalExtension, params};

use crate::model::{Caller, Role, StoreError, StoreResult, validate_slug};
use crate::store::users::{require_admin, user_id_by_handle};
use crate::store::{Store, audit, project_id_by_slug};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Project {
  pub id: i64,
  pub slug: String,
  pub name: String,
  pub created_at: String,
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
}
