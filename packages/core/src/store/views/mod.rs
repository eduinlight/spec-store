pub mod readiness;
pub mod spec_list;
pub mod traceability;

use std::collections::HashMap;

use diesel::pg::PgConnection;
use diesel::prelude::*;

use crate::model::{Caller, DocumentKind, StoreResult};
use crate::schema::{documents, revisions};
use crate::store::projects::access_in;
use crate::store::specs::{SpecRow, spec_by_number};

pub(crate) type BodiesBySpec = HashMap<i64, HashMap<DocumentKind, String>>;

/// Current bodies of the spec documents, keyed by spec id and document kind.
pub(crate) fn current_bodies(
  conn: &mut PgConnection,
  spec_ids: &[i64],
) -> StoreResult<BodiesBySpec> {
  let rows: Vec<(i64, String, String)> = documents::table
    .inner_join(revisions::table.on(documents::current_revision_id.eq(revisions::id.nullable())))
    .filter(documents::spec_id.eq_any(spec_ids))
    .select((
      documents::spec_id.assume_not_null(),
      documents::kind,
      revisions::body,
    ))
    .load(conn)?;
  let mut by_spec: BodiesBySpec = HashMap::new();
  for (spec_id, kind, body) in rows {
    by_spec
      .entry(spec_id)
      .or_default()
      .insert(kind.parse()?, body);
  }
  Ok(by_spec)
}

pub(crate) struct SpecScope {
  pub project_id: i64,
  pub spec: SpecRow,
}

/// Checks read access to the project and resolves the spec; outsiders get not found.
pub(crate) fn spec_scope(
  conn: &mut PgConnection,
  caller: &Caller,
  project: &str,
  number: i64,
) -> StoreResult<SpecScope> {
  let access = access_in(conn, caller, project)?;
  let spec = spec_by_number(conn, access.project_id, project, number)?;
  Ok(SpecScope {
    project_id: access.project_id,
    spec,
  })
}
