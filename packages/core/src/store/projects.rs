use diesel::pg::PgConnection;
use diesel::prelude::*;
use diesel::upsert::excluded;

use crate::address::ProjectRef;
use crate::model::{Caller, OrgRole, Role, StoreError, StoreResult, validate_slug};
use crate::schema::{iso_utc, memberships, organization_members, organizations, projects, users};
use crate::store::organizations::{org_role, organization_id_by_slug};
use crate::store::users::user_id_by_handle;
use crate::store::{Store, audit};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Project {
  pub id: i64,
  pub address: ProjectRef,
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

pub fn project_in(conn: &mut PgConnection, project_id: i64) -> StoreResult<Project> {
  let (id, organization, slug, name, repository_url, created_at) = projects::table
    .inner_join(organizations::table)
    .filter(projects::id.eq(project_id))
    .select((
      projects::id,
      organizations::slug,
      projects::slug,
      projects::name,
      projects::repository_url,
      iso_utc(projects::created_at),
    ))
    .first::<(i64, String, String, String, Option<String>, String)>(conn)?;
  Ok(Project {
    id,
    address: ProjectRef::new(&organization, &slug),
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
  pub organization_id: i64,
  pub role: Option<Role>,
  pub is_operator: bool,
}

impl Access {
  pub fn can_write(&self) -> bool {
    self.role.is_some_and(Role::can_write)
  }

  pub fn is_owner(&self) -> bool {
    self.role == Some(Role::Owner)
  }

  pub fn can_manage_members(&self) -> bool {
    self.is_owner()
  }
}

pub fn role_of(
  conn: &mut PgConnection,
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

pub fn access_in(conn: &mut PgConnection, caller: &Caller, project: &str) -> StoreResult<Access> {
  let address = ProjectRef::parse(project)?;
  let not_found = || StoreError::NotFound(format!("project {project:?}"));
  let found: Option<(i64, i64)> = projects::table
    .inner_join(organizations::table)
    .filter(organizations::slug.eq(&address.organization))
    .filter(projects::slug.eq(&address.project))
    .select((projects::id, projects::organization_id))
    .first(conn)
    .optional()?;
  let (project_id, organization_id) = found.ok_or_else(not_found)?;
  let role = match org_role(conn, organization_id, caller.user_id)? {
    Some(OrgRole::Owner) => Some(Role::Owner),
    _ => role_of(conn, project_id, caller.user_id)?,
  };
  if role.is_none() && !caller.is_operator {
    return Err(not_found());
  }
  Ok(Access {
    project_id,
    organization_id,
    role,
    is_operator: caller.is_operator,
  })
}

impl Store {
  pub fn create_project(&self, actor: &Caller, project: &str, name: &str) -> StoreResult<Project> {
    let address = ProjectRef::parse(project)?;
    validate_slug(&address.project)?;
    self.with_tx(|tx| {
      let organization_id = organization_id_by_slug(tx, &address.organization)?
        .ok_or_else(|| StoreError::NotFound(format!("organization {:?}", address.organization)))?;
      match org_role(tx, organization_id, actor.user_id)? {
        Some(OrgRole::Owner) => {}
        Some(OrgRole::Member) => return Err(StoreError::Forbidden),
        None if actor.is_operator => return Err(StoreError::Forbidden),
        None => {
          return Err(StoreError::NotFound(format!(
            "organization {:?}",
            address.organization
          )));
        }
      }
      let duplicate: i64 = projects::table
        .filter(projects::organization_id.eq(organization_id))
        .filter(projects::slug.eq(&address.project))
        .count()
        .get_result(tx)?;
      if duplicate > 0 {
        return Err(StoreError::Validation(format!(
          "project slug {:?} already exists in organization {:?}",
          address.project, address.organization
        )));
      }
      let (id, created_at): (i64, String) = diesel::insert_into(projects::table)
        .values((
          projects::organization_id.eq(organization_id),
          projects::slug.eq(&address.project),
          projects::name.eq(name),
        ))
        .returning((projects::id, iso_utc(projects::created_at)))
        .get_result(tx)?;
      audit(
        tx,
        actor.user_id,
        Some(id),
        "project.create",
        &address.to_string(),
      )?;
      Ok(Project {
        id,
        slug: address.project.clone(),
        address,
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
      if org_role(tx, access.organization_id, user_id)?.is_none() {
        return Err(StoreError::Validation(format!(
          "{handle:?} is not a member of the organization; invite them first"
        )));
      }
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
      Ok(
        visible_projects(conn, caller)?
          .into_iter()
          .map(|p| ProjectSummary {
            slug: p.address.to_string(),
            name: p.name,
            role: p
              .role
              .map(|r| r.as_str().to_string())
              .unwrap_or_else(|| "operator".to_string()),
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VisibleProject {
  pub id: i64,
  pub address: ProjectRef,
  pub name: String,
  pub role: Option<Role>,
}

type VisibleRow = (i64, String, String, String, Option<String>, Option<String>);

pub fn visible_projects(
  conn: &mut PgConnection,
  caller: &Caller,
) -> StoreResult<Vec<VisibleProject>> {
  let rows: Vec<VisibleRow> = projects::table
    .inner_join(organizations::table)
    .left_join(
      memberships::table.on(
        memberships::project_id
          .eq(projects::id)
          .and(memberships::user_id.eq(caller.user_id)),
      ),
    )
    .left_join(
      organization_members::table.on(
        organization_members::organization_id
          .eq(projects::organization_id)
          .and(organization_members::user_id.eq(caller.user_id)),
      ),
    )
    .select((
      projects::id,
      organizations::slug,
      projects::slug,
      projects::name,
      memberships::role.nullable(),
      organization_members::role.nullable(),
    ))
    .order((organizations::slug, projects::slug))
    .load(conn)?;
  let mut visible = Vec::new();
  for (id, organization, slug, name, project_role, org_role) in rows {
    let role = if org_role.as_deref() == Some(OrgRole::Owner.as_str()) {
      Some(Role::Owner)
    } else {
      project_role.map(|r| r.parse()).transpose()?
    };
    if role.is_none() && !caller.is_operator {
      continue;
    }
    visible.push(VisibleProject {
      id,
      address: ProjectRef::new(&organization, &slug),
      name,
      role,
    });
  }
  Ok(visible)
}

fn role_rank(role: Role) -> u8 {
  match role {
    Role::Owner => 0,
    Role::Editor => 1,
    Role::Reader => 2,
  }
}
