use diesel::prelude::*;

use crate::model::{Caller, OrgRole, StoreError, StoreResult};
use crate::schema::projects;
use crate::store::organizations::{organization_id_by_slug, readable_organization};
use crate::store::{AuditPage, Store, filtered_audit_entries};

fn page(entries: Vec<crate::store::AuditEntry>, limit: usize) -> AuditPage {
  let mut entries = entries;
  let has_more = entries.len() > limit;
  entries.truncate(limit);
  AuditPage { entries, has_more }
}

impl Store {
  pub fn list_organization_audit(
    &self,
    caller: &Caller,
    organization: &str,
    project: Option<&str>,
    before: Option<i64>,
    limit: usize,
  ) -> StoreResult<AuditPage> {
    self.with_conn(|conn| {
      let (organization_id, role) = readable_organization(conn, caller, organization)?;
      if role != Some(OrgRole::Owner) && !caller.is_operator {
        return Err(StoreError::Forbidden);
      }
      let project_id = match project {
        Some(slug) => Some(
          projects::table
            .filter(projects::organization_id.eq(organization_id))
            .filter(projects::slug.eq(slug))
            .select(projects::id)
            .first::<i64>(conn)
            .optional()?
            .ok_or_else(|| StoreError::NotFound(format!("project \"{organization}/{slug}\"")))?,
        ),
        None => None,
      };
      let entries = filtered_audit_entries(
        conn,
        Some(organization_id),
        project_id,
        before,
        limit as i64 + 1,
      )?;
      Ok(page(entries, limit))
    })
  }

  pub fn list_global_audit(
    &self,
    caller: &Caller,
    organization: Option<&str>,
    before: Option<i64>,
    limit: usize,
  ) -> StoreResult<AuditPage> {
    if !caller.is_operator {
      return Err(StoreError::Forbidden);
    }
    self.with_conn(|conn| {
      let organization_id = match organization {
        Some(slug) => Some(
          organization_id_by_slug(conn, slug)?
            .ok_or_else(|| StoreError::NotFound(format!("organization {slug:?}")))?,
        ),
        None => None,
      };
      let entries = filtered_audit_entries(conn, organization_id, None, before, limit as i64 + 1)?;
      Ok(page(entries, limit))
    })
  }
}
