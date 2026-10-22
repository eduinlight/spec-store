use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use diesel::upsert::excluded;

use crate::model::{Caller, Role, StoreError, StoreResult, validate_slug};
use crate::schema::{memberships, projects, users};
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

pub fn project_in(conn: &mut SqliteConnection, project_id: i64) -> StoreResult<Project> {
  let (id, slug, name, repository_url, created_at) = projects::table
    .filter(projects::id.eq(project_id))
    .select((
      projects::id,
      projects::slug,
      projects::name,
      projects::repository_url,
      projects::created_at,
    ))
    .first::<(i64, String, String, Option<String>, String)>(conn)?;
  Ok(Project {
    id,
    slug,
    name,
    repository_url,
    created_at,
  })
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

pub fn role_of(
  conn: &mut SqliteConnection,
  project_id: i64,
  user_id: i64,
) -> StoreResult<Option<Role>> {
  let role: Option<String> = memberships::table
    .filter(memberships::project_id.eq(project_id))
    .filter(memberships::user_id.eq(user_id))
    .select(memberships::role)
    .first(conn)
    .optional()?;
  role.map(|r| r.parse()).transpose()
}

pub fn access_in(conn: &mut SqliteConnection, caller: &Caller, slug: &str) -> StoreResult<Access> {
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
      let (id, created_at): (i64, String) = diesel::insert_into(projects::table)
        .values((projects::slug.eq(slug), projects::name.eq(name)))
        .returning((projects::id, projects::created_at))
        .get_result(tx)?;
      audit(tx, actor.user_id, Some(id), "project.create", slug)?;
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
      diesel::insert_into(memberships::table)
        .values((
          memberships::project_id.eq(access.project_id),
          memberships::user_id.eq(user_id),
          memberships::role.eq(role.as_str()),
        ))
        .on_conflict((memberships::project_id, memberships::user_id))
        .do_update()
        .set(memberships::role.eq(excluded(memberships::role)))
        .execute(tx)?;
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
      let removed = diesel::delete(
        memberships::table
          .filter(memberships::project_id.eq(access.project_id))
          .filter(memberships::user_id.eq(user_id)),
      )
      .execute(tx)?;
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
      let joined = projects::table.left_join(
        memberships::table.on(
          memberships::project_id
            .eq(projects::id)
            .and(memberships::user_id.eq(caller.user_id)),
        ),
      );
      let rows: Vec<(String, String, Option<String>)> = if caller.is_admin {
        joined
          .select((projects::slug, projects::name, memberships::role.nullable()))
          .order(projects::slug)
          .load(conn)?
      } else {
        joined
          .filter(memberships::role.nullable().is_not_null())
          .select((projects::slug, projects::name, memberships::role.nullable()))
          .order(projects::slug)
          .load(conn)?
      };
      Ok(
        rows
          .into_iter()
          .map(|(slug, name, role)| ProjectSummary {
            slug,
            name,
            role: role.unwrap_or_else(|| "admin".to_string()),
          })
          .collect(),
      )
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
      let rows: Vec<(String, String)> = memberships::table
        .inner_join(users::table)
        .filter(memberships::project_id.eq(access.project_id))
        .select((users::handle, memberships::role))
        .load(conn)?;
      let mut members = rows
        .into_iter()
        .map(|(handle, role)| {
          Ok(Member {
            handle,
            role: role.parse()?,
          })
        })
        .collect::<StoreResult<Vec<_>>>()?;
      members.sort_by(|a, b| {
        role_rank(a.role)
          .cmp(&role_rank(b.role))
          .then_with(|| a.handle.cmp(&b.handle))
      });
      Ok(members)
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
      diesel::update(projects::table.filter(projects::id.eq(access.project_id)))
        .set(projects::repository_url.eq(url))
        .execute(tx)?;
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

fn role_rank(role: Role) -> u8 {
  match role {
    Role::Owner => 0,
    Role::Editor => 1,
    Role::Reader => 2,
  }
}
