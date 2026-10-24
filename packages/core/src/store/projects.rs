use std::collections::HashMap;

use diesel::dsl::sql;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use diesel::sql_types::{Nullable, Timestamptz};
use diesel::upsert::excluded;

use crate::address::ProjectRef;
use crate::model::{Caller, DocumentKind, OrgRole, Role, StoreError, StoreResult, validate_slug};
use crate::schema::{
  audit_log, iso_utc, memberships, organization_members, organizations, projects, specs, users,
};
use crate::store::documents::{append_revision, current_revision, document_id, ensure_document};
use crate::store::organizations::{org_role, organization_id_by_slug, owned_organization};
use crate::store::refusals::Attempt;
use crate::store::specs::spec_rows_in;
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
  pub archived: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectMember {
  pub handle: String,
  pub name: String,
  pub role: Role,
  pub guest: bool,
  pub joined_at: String,
  pub last_active_at: Option<String>,
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
  let (id, organization, slug, name, repository_url, created_at, archived) = projects::table
    .inner_join(organizations::table)
    .filter(projects::id.eq(project_id))
    .select((
      projects::id,
      organizations::slug,
      projects::slug,
      projects::name,
      projects::repository_url,
      iso_utc(projects::created_at),
      projects::archived_at.is_not_null(),
    ))
    .first::<(i64, String, String, String, Option<String>, String, bool)>(conn)?;
  Ok(Project {
    id,
    address: ProjectRef::new(&organization, &slug),
    slug,
    name,
    repository_url,
    created_at,
    archived,
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
  pub archived: bool,
}

impl Access {
  pub fn can_write(&self) -> bool {
    !self.archived && self.role.is_some_and(Role::can_write)
  }

  pub fn is_owner(&self) -> bool {
    self.role == Some(Role::Owner)
  }

  pub fn can_administer(&self) -> bool {
    !self.archived && self.is_owner()
  }

  pub fn can_manage_members(&self) -> bool {
    self.can_administer()
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
  let found: Option<(i64, i64, bool)> = projects::table
    .inner_join(organizations::table)
    .filter(organizations::slug.eq(&address.organization))
    .filter(projects::slug.eq(&address.project))
    .select((
      projects::id,
      projects::organization_id,
      projects::archived_at.is_not_null(),
    ))
    .first(conn)
    .optional()?;
  let (project_id, organization_id, archived) = found.ok_or_else(not_found)?;
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
    archived,
  })
}

impl Store {
  pub fn create_project(&self, actor: &Caller, project: &str, name: &str) -> StoreResult<Project> {
    let address = ProjectRef::parse(project)?;
    validate_slug(&address.project)?;
    let organization = address.organization.clone();
    let attempt = Attempt::organization("project.create", &organization, project);
    self.audited(actor, attempt, |tx| {
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
      audit(tx, actor, Some(id), "project.create", &address.to_string())?;
      Ok(Project {
        id,
        slug: address.project.clone(),
        address,
        name: name.to_string(),
        repository_url: None,
        created_at,
        archived: false,
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
    let attempt = Attempt::project("member.set", project_slug, format!("{handle}:{role}"));
    self.audited(actor, attempt, |tx| {
      let access = access_in(tx, actor, project_slug)?;
      if !access.can_manage_members() {
        return Err(StoreError::Forbidden);
      }
      lock_project(tx, access.project_id)?;
      let user_id = user_id_by_handle(tx, handle)?;
      let current = role_of(tx, access.project_id, user_id)?;
      if current.is_none() && org_role(tx, access.organization_id, user_id)?.is_none() {
        return Err(StoreError::Validation(format!(
          "{handle:?} is not a member of the organization; invite them to the project instead"
        )));
      }
      if current == Some(Role::Owner)
        && role != Role::Owner
        && owner_count(tx, access.project_id)? <= 1
      {
        return Err(last_owner_refusal(project_slug));
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
        actor,
        Some(access.project_id),
        "member.set",
        &format!("{handle}:{role}"),
      )
    })
  }

  pub fn remove_member(&self, actor: &Caller, project_slug: &str, handle: &str) -> StoreResult<()> {
    let attempt = Attempt::project("member.remove", project_slug, handle);
    self.audited(actor, attempt, |tx| {
      let access = access_in(tx, actor, project_slug)?;
      if !access.can_manage_members() {
        return Err(StoreError::Forbidden);
      }
      lock_project(tx, access.project_id)?;
      let user_id = user_id_by_handle(tx, handle)?;
      if role_of(tx, access.project_id, user_id)? == Some(Role::Owner)
        && owner_count(tx, access.project_id)? <= 1
      {
        return Err(last_owner_refusal(project_slug));
      }
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
      audit(tx, actor, Some(access.project_id), "member.remove", handle)
    })
  }

  pub fn copy_project(
    &self,
    actor: &Caller,
    source: &str,
    slug: &str,
    name: &str,
    repository_url: Option<&str>,
  ) -> StoreResult<Project> {
    let source_address = ProjectRef::parse(source)?;
    validate_slug(slug)?;
    if let Some(url) = repository_url {
      validate_repository_url(url)?;
    }
    let address = ProjectRef::new(&source_address.organization, slug);
    let target = format!("{source}→{address}");
    let attempt =
      Attempt::organization("project.copy", &source_address.organization, target.clone());
    self.audited(actor, attempt, |tx| {
      let organization_id = owned_organization(tx, actor, &source_address.organization)?;
      let origin = access_in(tx, actor, source)?;
      let duplicate: i64 = projects::table
        .filter(projects::organization_id.eq(organization_id))
        .filter(projects::slug.eq(slug))
        .count()
        .get_result(tx)?;
      if duplicate > 0 {
        return Err(StoreError::Validation(format!(
          "project slug {slug:?} already exists in organization {:?}",
          source_address.organization
        )));
      }
      let (id, created_at): (i64, String) = diesel::insert_into(projects::table)
        .values((
          projects::organization_id.eq(organization_id),
          projects::slug.eq(slug),
          projects::name.eq(name),
          projects::repository_url.eq(repository_url),
        ))
        .returning((projects::id, iso_utc(projects::created_at)))
        .get_result(tx)?;
      let message = format!("Copied from {source}");
      copy_documents(tx, actor, origin.project_id, id, &message)?;
      audit(tx, actor, Some(id), "project.copy", &target)?;
      Ok(Project {
        id,
        slug: slug.to_string(),
        address,
        name: name.to_string(),
        repository_url: repository_url.map(str::to_string),
        created_at,
        archived: false,
      })
    })
  }

  pub fn archive_project(&self, actor: &Caller, project: &str) -> StoreResult<()> {
    self.set_archived(actor, project, true)
  }

  pub fn restore_project(&self, actor: &Caller, project: &str) -> StoreResult<()> {
    self.set_archived(actor, project, false)
  }

  fn set_archived(&self, actor: &Caller, project: &str, archive: bool) -> StoreResult<()> {
    let action = if archive {
      "project.archive"
    } else {
      "project.restore"
    };
    let attempt = Attempt::project(action, project, project);
    self.audited(actor, attempt, |tx| {
      let access = access_in(tx, actor, project)?;
      if !access.is_owner() {
        return Err(StoreError::Forbidden);
      }
      if access.archived == archive {
        return Err(StoreError::Validation(if archive {
          format!("project {project:?} is already archived")
        } else {
          format!("project {project:?} is not archived")
        }));
      }
      let target = projects::table.filter(projects::id.eq(access.project_id));
      if archive {
        diesel::update(target)
          .set(projects::archived_at.eq(diesel::dsl::now))
          .execute(tx)?;
      } else {
        diesel::update(target)
          .set(projects::archived_at.eq(sql::<Nullable<Timestamptz>>("NULL")))
          .execute(tx)?;
      }
      audit(tx, actor, Some(access.project_id), action, project)
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

  pub fn project_members(&self, caller: &Caller, slug: &str) -> StoreResult<Vec<ProjectMember>> {
    self.with_conn(|conn| {
      let access = access_in(conn, caller, slug)?;
      let rows: Vec<(i64, String, String, String, String, bool)> = memberships::table
        .inner_join(users::table)
        .left_join(
          organization_members::table.on(
            organization_members::organization_id
              .eq(access.organization_id)
              .and(organization_members::user_id.eq(memberships::user_id)),
          ),
        )
        .filter(memberships::project_id.eq(access.project_id))
        .select((
          users::id,
          users::handle,
          users::name,
          memberships::role,
          iso_utc(memberships::created_at),
          organization_members::user_id.nullable().is_null(),
        ))
        .load(conn)?;
      let mut last_active = last_active_in(conn, access.project_id)?;
      let mut members = rows
        .into_iter()
        .map(|(user_id, handle, name, role, joined_at, guest)| {
          Ok(ProjectMember {
            handle,
            name,
            role: role.parse()?,
            guest,
            joined_at,
            last_active_at: last_active.remove(&user_id),
          })
        })
        .collect::<StoreResult<Vec<_>>>()?;
      members.sort_by(|a, b| {
        (a.role != Role::Owner)
          .cmp(&(b.role != Role::Owner))
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
    let attempt = Attempt::project("project.repo", slug, slug);
    self.audited(actor, attempt, |tx| {
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
      audit(tx, actor, Some(access.project_id), "project.repo", &target)
    })
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VisibleProject {
  pub id: i64,
  pub address: ProjectRef,
  pub name: String,
  pub role: Option<Role>,
  pub archived: bool,
}

type VisibleRow = (
  i64,
  String,
  String,
  String,
  Option<String>,
  Option<String>,
  bool,
);

pub fn visible_projects(
  conn: &mut PgConnection,
  caller: &Caller,
) -> StoreResult<Vec<VisibleProject>> {
  visible_projects_in(conn, caller, None)
}

pub fn visible_projects_in(
  conn: &mut PgConnection,
  caller: &Caller,
  organization: Option<&str>,
) -> StoreResult<Vec<VisibleProject>> {
  let mut query = projects::table
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
      projects::archived_at.is_not_null(),
    ))
    .order((organizations::slug, projects::slug))
    .into_boxed();
  if let Some(organization) = organization {
    query = query.filter(organizations::slug.eq(organization));
  }
  let rows: Vec<VisibleRow> = query.load(conn)?;
  let mut visible = Vec::new();
  for (id, organization, slug, name, project_role, org_role, archived) in rows {
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
      archived,
    });
  }
  Ok(visible)
}

pub(crate) fn lock_project(conn: &mut PgConnection, project_id: i64) -> StoreResult<()> {
  projects::table
    .filter(projects::id.eq(project_id))
    .select(projects::id)
    .for_update()
    .first::<i64>(conn)?;
  Ok(())
}

pub(crate) fn owner_count(conn: &mut PgConnection, project_id: i64) -> StoreResult<i64> {
  Ok(
    memberships::table
      .filter(memberships::project_id.eq(project_id))
      .filter(memberships::role.eq(Role::Owner.as_str()))
      .count()
      .get_result(conn)?,
  )
}

pub(crate) fn last_owner_refusal(project: &str) -> StoreError {
  StoreError::Validation(format!(
    "this change would leave project {project:?} without an owner; make another member an owner first"
  ))
}

fn last_active_in(conn: &mut PgConnection, project_id: i64) -> StoreResult<HashMap<i64, String>> {
  let rows: Vec<(i64, String)> = audit_log::table
    .filter(audit_log::project_id.eq(project_id))
    .filter(audit_log::result.eq("ok"))
    .distinct_on(audit_log::actor_id)
    .order((audit_log::actor_id, audit_log::id.desc()))
    .select((audit_log::actor_id, iso_utc(audit_log::created_at)))
    .load(conn)?;
  Ok(rows.into_iter().collect())
}

fn copy_document(
  conn: &mut PgConnection,
  actor: &Caller,
  from: (i64, Option<i64>),
  to: (i64, Option<i64>),
  kind: DocumentKind,
  message: &str,
) -> StoreResult<()> {
  let Some(source_id) = document_id(conn, from.0, from.1, kind)? else {
    return Ok(());
  };
  let Some((body, meta)) = current_revision(conn, source_id)? else {
    return Ok(());
  };
  let target_id = ensure_document(conn, to.0, to.1, kind)?;
  append_revision(
    conn,
    target_id,
    &body,
    meta.version.as_deref(),
    actor,
    message,
  )?;
  Ok(())
}

fn copy_documents(
  conn: &mut PgConnection,
  actor: &Caller,
  from_project: i64,
  to_project: i64,
  message: &str,
) -> StoreResult<()> {
  copy_document(
    conn,
    actor,
    (from_project, None),
    (to_project, None),
    DocumentKind::Constitution,
    message,
  )?;
  for spec in spec_rows_in(conn, from_project)? {
    let new_spec_id: i64 = diesel::insert_into(specs::table)
      .values((
        specs::project_id.eq(to_project),
        specs::number.eq(spec.number),
        specs::slug.eq(&spec.slug),
        specs::status.eq(spec.status.as_str()),
        specs::branch.eq(&spec.branch),
      ))
      .returning(specs::id)
      .get_result(conn)?;
    for kind in DocumentKind::SPEC_KINDS {
      copy_document(
        conn,
        actor,
        (from_project, Some(spec.id)),
        (to_project, Some(new_spec_id)),
        kind,
        message,
      )?;
    }
  }
  Ok(())
}
