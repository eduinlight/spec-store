pub mod markers;
pub mod readiness;
pub mod spec_list;
pub mod task_history;
pub mod traceability;

use std::collections::HashMap;

use diesel::pg::PgConnection;
use diesel::prelude::*;

use crate::model::{Caller, DocumentKind, StoreResult};
use crate::schema::{documents, iso_utc, revisions, users};
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RevisionRecord {
  pub id: i64,
  pub body: String,
  pub author: String,
  pub client: Option<String>,
  pub at: String,
}

/// Every revision of a document with its body and attribution, oldest first.
pub fn revision_records(
  conn: &mut PgConnection,
  document_id: i64,
) -> StoreResult<Vec<RevisionRecord>> {
  let rows: Vec<(i64, String, String, Option<String>, String)> = revisions::table
    .inner_join(users::table)
    .filter(revisions::document_id.eq(document_id))
    .select((
      revisions::id,
      revisions::body,
      users::handle,
      revisions::client,
      iso_utc(revisions::created_at),
    ))
    .order(revisions::id)
    .load(conn)?;
  Ok(
    rows
      .into_iter()
      .map(|(id, body, author, client, at)| RevisionRecord {
        id,
        body,
        author,
        client,
        at,
      })
      .collect(),
  )
}

/// Ids of the documents of a spec, by kind.
pub(crate) fn spec_document_ids(
  conn: &mut PgConnection,
  spec_id: i64,
) -> StoreResult<Vec<(DocumentKind, i64)>> {
  let rows: Vec<(String, i64)> = documents::table
    .filter(documents::spec_id.eq(spec_id))
    .select((documents::kind, documents::id))
    .load(conn)?;
  let mut found = rows
    .into_iter()
    .map(|(kind, id)| Ok((kind.parse()?, id)))
    .collect::<StoreResult<Vec<(DocumentKind, i64)>>>()?;
  found.sort_by_key(|(kind, _)| {
    DocumentKind::SPEC_KINDS
      .iter()
      .position(|k| k == kind)
      .unwrap_or(DocumentKind::SPEC_KINDS.len())
  });
  Ok(found)
}
