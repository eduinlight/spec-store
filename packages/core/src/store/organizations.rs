use diesel::pg::PgConnection;
use diesel::prelude::*;

use crate::model::{Caller, OrgRole, StoreError, StoreResult, validate_slug};
use crate::schema::{iso_utc, organization_members, organizations};
use crate::store::users::user_id_by_handle;
use crate::store::{Store, audit_in};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Organization {
  pub id: i64,
  pub slug: String,
  pub name: String,
  pub created_at: String,
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
      audit_in(
        tx,
        actor.user_id,
        Some(id),
        None,
        "organization.create",
        slug,
      )?;
      Ok(Organization {
        id,
        slug: slug.to_string(),
        name: name.to_string(),
        created_at,
      })
    })
  }

  pub fn set_organization_member(
    &self,
    actor: &Caller,
    organization: &str,
    handle: &str,
    role: OrgRole,
  ) -> StoreResult<()> {
    self.with_tx(|tx| {
      let organization_id = owned_organization(tx, actor, organization)?;
      let user_id = user_id_by_handle(tx, handle)?;
      diesel::insert_into(organization_members::table)
        .values((
          organization_members::organization_id.eq(organization_id),
          organization_members::user_id.eq(user_id),
          organization_members::role.eq(role.as_str()),
        ))
        .on_conflict((
          organization_members::organization_id,
          organization_members::user_id,
        ))
        .do_update()
        .set(organization_members::role.eq(role.as_str()))
        .execute(tx)?;
      audit_in(
        tx,
        actor.user_id,
        Some(organization_id),
        None,
        "organization.member.set",
        &format!("{handle}:{role}"),
      )
    })
  }
}

pub fn owned_organization(
  conn: &mut PgConnection,
  actor: &Caller,
  organization: &str,
) -> StoreResult<i64> {
  let not_found = || StoreError::NotFound(format!("organization {organization:?}"));
  let organization_id = organization_id_by_slug(conn, organization)?.ok_or_else(not_found)?;
  match org_role(conn, organization_id, actor.user_id)? {
    Some(OrgRole::Owner) => Ok(organization_id),
    Some(OrgRole::Member) => Err(StoreError::Forbidden),
    None if actor.is_operator => Err(StoreError::Forbidden),
    None => Err(not_found()),
  }
}
