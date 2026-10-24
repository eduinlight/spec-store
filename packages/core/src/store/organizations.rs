use diesel::pg::PgConnection;
use diesel::prelude::*;

use crate::model::{AuditResult, Caller, OrgRole, StoreError, StoreResult, Via, validate_slug};
use crate::schema::{iso_utc, memberships, organization_members, organizations, projects, users};
use crate::store::refusals::Attempt;
use crate::store::users::user_id_by_handle;
use crate::store::{NewAudit, Store, audit_in, insert_audit};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Organization {
  pub id: i64,
  pub slug: String,
  pub name: String,
  pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrganizationSummary {
  pub slug: String,
  pub name: String,
  pub role: Option<OrgRole>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrganizationMember {
  pub handle: String,
  pub name: String,
  pub role: OrgRole,
}

pub fn organization_id_by_slug(conn: &mut PgConnection, slug: &str) -> StoreResult<Option<i64>> {
  Ok(
    organizations::table
      .filter(organizations::slug.eq(slug))
      .select(organizations::id)
      .first(conn)
      .optional()?,
  )
}

pub fn org_role(
  conn: &mut PgConnection,
  organization_id: i64,
  user_id: i64,
) -> StoreResult<Option<OrgRole>> {
  let role: Option<String> = organization_members::table
    .filter(organization_members::organization_id.eq(organization_id))
    .filter(organization_members::user_id.eq(user_id))
    .select(organization_members::role)
    .first(conn)
    .optional()?;
  role.map(|r| r.parse()).transpose()
}

fn not_found(organization: &str) -> StoreError {
  StoreError::NotFound(format!("organization {organization:?}"))
}

pub fn readable_organization(
  conn: &mut PgConnection,
  caller: &Caller,
  organization: &str,
) -> StoreResult<(i64, Option<OrgRole>)> {
  let organization_id =
    organization_id_by_slug(conn, organization)?.ok_or_else(|| not_found(organization))?;
  let role = org_role(conn, organization_id, caller.user_id)?;
  if role.is_none() && !caller.is_operator {
    return Err(not_found(organization));
  }
  Ok((organization_id, role))
}

pub fn owned_organization(
  conn: &mut PgConnection,
  actor: &Caller,
  organization: &str,
) -> StoreResult<i64> {
  let (organization_id, role) = readable_organization(conn, actor, organization)?;
  match role {
    Some(OrgRole::Owner) => Ok(organization_id),
    _ => Err(StoreError::Forbidden),
  }
}

fn lock_organization(conn: &mut PgConnection, organization_id: i64) -> StoreResult<()> {
  organizations::table
    .filter(organizations::id.eq(organization_id))
    .select(organizations::id)
    .for_update()
    .first::<i64>(conn)?;
  Ok(())
}

fn owner_count(conn: &mut PgConnection, organization_id: i64) -> StoreResult<i64> {
  Ok(
    organization_members::table
      .filter(organization_members::organization_id.eq(organization_id))
      .filter(organization_members::role.eq(OrgRole::Owner.as_str()))
      .count()
      .get_result(conn)?,
  )
}

fn last_owner_refusal(organization: &str) -> StoreError {
  StoreError::Validation(format!(
    "this change would leave organization {organization:?} without an owner; make another member an owner first"
  ))
}

fn member_role(
  conn: &mut PgConnection,
  organization_id: i64,
  organization: &str,
  handle: &str,
) -> StoreResult<(i64, OrgRole)> {
  let missing = || {
    StoreError::NotFound(format!(
      "member {handle:?} of organization {organization:?}"
    ))
  };
  let user_id = user_id_by_handle(conn, handle).map_err(|_| missing())?;
  let role = org_role(conn, organization_id, user_id)?.ok_or_else(missing)?;
  Ok((user_id, role))
}

impl Store {
  pub fn create_organization(
    &self,
    actor: &Caller,
    slug: &str,
    name: &str,
  ) -> StoreResult<Organization> {
    validate_slug(slug)?;
    self.with_tx(|tx| {
      if organization_id_by_slug(tx, slug)?.is_some() {
        return Err(StoreError::Validation(format!(
          "organization slug {slug:?} already exists"
        )));
      }
      let (id, created_at): (i64, String) = diesel::insert_into(organizations::table)
        .values((organizations::slug.eq(slug), organizations::name.eq(name)))
        .returning((organizations::id, iso_utc(organizations::created_at)))
        .get_result(tx)?;
      diesel::insert_into(organization_members::table)
        .values((
          organization_members::organization_id.eq(id),
          organization_members::user_id.eq(actor.user_id),
          organization_members::role.eq(OrgRole::Owner.as_str()),
        ))
        .execute(tx)?;
      audit_in(tx, actor, Some(id), None, "organization.create", slug)?;
      Ok(Organization {
        id,
        slug: slug.to_string(),
        name: name.to_string(),
        created_at,
      })
    })
  }

  pub fn list_organizations(&self, caller: &Caller) -> StoreResult<Vec<OrganizationSummary>> {
    self.with_conn(|conn| {
      let rows: Vec<(String, String, Option<String>)> = organizations::table
        .left_join(
          organization_members::table.on(
            organization_members::organization_id
              .eq(organizations::id)
              .and(organization_members::user_id.eq(caller.user_id)),
          ),
        )
        .select((
          organizations::slug,
          organizations::name,
          organization_members::role.nullable(),
        ))
        .order(organizations::slug)
        .load(conn)?;
      rows
        .into_iter()
        .filter(|(_, _, role)| role.is_some() || caller.is_operator)
        .map(|(slug, name, role)| {
          Ok(OrganizationSummary {
            slug,
            name,
            role: role.map(|r| r.parse()).transpose()?,
          })
        })
        .collect()
    })
  }

  pub fn organization(&self, caller: &Caller, organization: &str) -> StoreResult<Organization> {
    self.with_conn(|conn| {
      let (organization_id, _) = readable_organization(conn, caller, organization)?;
      let (id, slug, name, created_at) = organizations::table
        .filter(organizations::id.eq(organization_id))
        .select((
          organizations::id,
          organizations::slug,
          organizations::name,
          iso_utc(organizations::created_at),
        ))
        .first::<(i64, String, String, String)>(conn)?;
      Ok(Organization {
        id,
        slug,
        name,
        created_at,
      })
    })
  }

  pub fn list_organization_members(
    &self,
    caller: &Caller,
    organization: &str,
  ) -> StoreResult<Vec<OrganizationMember>> {
    self.with_conn(|conn| {
      let (organization_id, _) = readable_organization(conn, caller, organization)?;
      let rows: Vec<(String, String, String)> = organization_members::table
        .inner_join(users::table)
        .filter(organization_members::organization_id.eq(organization_id))
        .select((users::handle, users::name, organization_members::role))
        .load(conn)?;
      let mut members = rows
        .into_iter()
        .map(|(handle, name, role)| {
          Ok(OrganizationMember {
            handle,
            name,
            role: role.parse()?,
          })
        })
        .collect::<StoreResult<Vec<_>>>()?;
      members.sort_by(|a, b| {
        (a.role != OrgRole::Owner)
          .cmp(&(b.role != OrgRole::Owner))
          .then_with(|| a.handle.cmp(&b.handle))
      });
      Ok(members)
    })
  }

  pub fn admit_organization_member(
    &self,
    organization: &str,
    user_id: i64,
    role: OrgRole,
  ) -> StoreResult<()> {
    self.with_tx(|tx| admit_member(tx, organization, user_id, role, &Via::default()))
  }

  pub fn set_organization_member(
    &self,
    actor: &Caller,
    organization: &str,
    handle: &str,
    role: OrgRole,
  ) -> StoreResult<()> {
    let attempt = Attempt::organization(
      "organization.member.set",
      organization,
      format!("{handle}:{role}"),
    );
    self.audited(actor, attempt, |tx| {
      let organization_id = owned_organization(tx, actor, organization)?;
      lock_organization(tx, organization_id)?;
      let (user_id, current) = member_role(tx, organization_id, organization, handle)?;
      if current == OrgRole::Owner
        && role != OrgRole::Owner
        && owner_count(tx, organization_id)? <= 1
      {
        return Err(last_owner_refusal(organization));
      }
      diesel::update(
        organization_members::table
          .filter(organization_members::organization_id.eq(organization_id))
          .filter(organization_members::user_id.eq(user_id)),
      )
      .set(organization_members::role.eq(role.as_str()))
      .execute(tx)?;
      audit_in(
        tx,
        actor,
        Some(organization_id),
        None,
        "organization.member.set",
        &format!("{handle}:{role}"),
      )
    })
  }

  pub fn remove_organization_member(
    &self,
    actor: &Caller,
    organization: &str,
    handle: &str,
  ) -> StoreResult<()> {
    let attempt = Attempt::organization("organization.member.remove", organization, handle);
    self.audited(actor, attempt, |tx| {
      let (organization_id, actor_role) = readable_organization(tx, actor, organization)?;
      let leaving = actor.handle == handle && actor_role.is_some();
      if actor_role != Some(OrgRole::Owner) && !leaving {
        return Err(StoreError::Forbidden);
      }
      lock_organization(tx, organization_id)?;
      let (user_id, role) = member_role(tx, organization_id, organization, handle)?;
      if role == OrgRole::Owner && owner_count(tx, organization_id)? <= 1 {
        return Err(last_owner_refusal(organization));
      }
      let organization_projects = projects::table
        .filter(projects::organization_id.eq(organization_id))
        .select(projects::id);
      diesel::delete(
        memberships::table
          .filter(memberships::user_id.eq(user_id))
          .filter(memberships::project_id.eq_any(organization_projects)),
      )
      .execute(tx)?;
      diesel::delete(
        organization_members::table
          .filter(organization_members::organization_id.eq(organization_id))
          .filter(organization_members::user_id.eq(user_id)),
      )
      .execute(tx)?;
      audit_in(
        tx,
        actor,
        Some(organization_id),
        None,
        "organization.member.remove",
        handle,
      )
    })
  }
}

pub fn admit_member(
  conn: &mut PgConnection,
  organization: &str,
  user_id: i64,
  role: OrgRole,
  via: &Via,
) -> StoreResult<()> {
  let organization_id =
    organization_id_by_slug(conn, organization)?.ok_or_else(|| not_found(organization))?;
  let inserted = diesel::insert_into(organization_members::table)
    .values((
      organization_members::organization_id.eq(organization_id),
      organization_members::user_id.eq(user_id),
      organization_members::role.eq(role.as_str()),
    ))
    .on_conflict_do_nothing()
    .execute(conn)?;
  if inserted > 0 {
    let handle: String = users::table
      .filter(users::id.eq(user_id))
      .select(users::handle)
      .first(conn)?;
    insert_audit(
      conn,
      NewAudit {
        actor_id: user_id,
        via,
        organization_id: Some(organization_id),
        project_id: None,
        action: "organization.member.admit",
        target: &format!("{handle}:{role}"),
        result: AuditResult::Ok,
        detail: None,
      },
    )?;
  }
  Ok(())
}
