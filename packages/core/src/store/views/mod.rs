pub mod active;
pub mod activity;
pub mod markers;
pub mod overview;
pub mod readiness;
pub mod spec_list;
pub mod task_history;
pub mod traceability;

use std::collections::HashMap;

use diesel::pg::PgConnection;
use diesel::prelude::*;

use crate::markdown::markers::scan_markers;
use crate::model::{Caller, DocumentKind, SpecStatus, StoreResult};
use crate::schema::{documents, iso_utc, revisions, users};
use crate::store::projects::access_in;
use crate::store::specs::{SpecRow, spec_by_number, spec_rows_in};
use crate::store::tasks::SpecMarker;

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

pub(crate) struct LiveSpec {
  pub row: SpecRow,
  pub bodies: HashMap<DocumentKind, String>,
}

impl LiveSpec {
  pub fn body(&self, kind: DocumentKind) -> Option<&str> {
    self.bodies.get(&kind).map(String::as_str)
  }

  pub fn markers(&self) -> Vec<SpecMarker> {
    DocumentKind::SPEC_KINDS
      .into_iter()
      .filter_map(|kind| self.bodies.get(&kind).map(|body| (kind, body)))
      .flat_map(|(kind, body)| {
        scan_markers(body).into_iter().map(move |m| SpecMarker {
          document_kind: kind,
          kind: m.kind,
          heading: m.heading,
          text: m.text,
          line: m.line,
        })
      })
      .collect()
  }
}

/// Non-archived specs of a project with their current documents, by number ascending; the last one is the active spec.
pub(crate) fn live_specs(conn: &mut PgConnection, project_id: i64) -> StoreResult<Vec<LiveSpec>> {
  let rows: Vec<SpecRow> = spec_rows_in(conn, project_id)?
    .into_iter()
    .filter(|row| row.status != SpecStatus::Archived)
    .collect();
  let ids: Vec<i64> = rows.iter().map(|row| row.id).collect();
  let mut bodies = current_bodies(conn, &ids)?;
  Ok(
    rows
      .into_iter()
      .map(|row| LiveSpec {
        bodies: bodies.remove(&row.id).unwrap_or_default(),
        row,
      })
      .collect(),
  )
}
