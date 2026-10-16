use rusqlite::{Connection, OptionalExtension, params};

use crate::model::{Caller, DocumentKind, SpecStatus, StoreError, StoreResult};
use crate::store::Store;
use crate::store::projects::access_in;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpecSummary {
  pub number: i64,
  pub slug: String,
  pub status: SpecStatus,
  pub branch: String,
  pub kinds: Vec<DocumentKind>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpecRow {
  pub id: i64,
  pub number: i64,
  pub slug: String,
  pub status: SpecStatus,
  pub branch: String,
}

pub fn branch_name(number: i64, slug: &str) -> String {
  format!("{number:03}-{slug}")
}

fn row_to_spec(r: &rusqlite::Row<'_>) -> rusqlite::Result<(i64, i64, String, String, String)> {
  Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
}

fn to_spec_row(raw: (i64, i64, String, String, String)) -> StoreResult<SpecRow> {
  let (id, number, slug, status, branch) = raw;
  Ok(SpecRow {
    id,
    number,
    slug,
    status: status.parse()?,
    branch,
  })
}

pub fn spec_by_number(
  conn: &Connection,
  project_id: i64,
  project_slug: &str,
  number: i64,
) -> StoreResult<SpecRow> {
  let raw = conn
    .query_row(
      "SELECT id, number, slug, status, branch FROM specs WHERE project_id = ?1 AND number = ?2",
      params![project_id, number],
      row_to_spec,
    )
    .optional()?;
  match raw {
    Some(raw) => to_spec_row(raw),
    None => Err(StoreError::NotFound(format!(
      "spec {number:03} in project {project_slug:?}"
    ))),
  }
}

pub fn kinds_of(conn: &Connection, spec_id: i64) -> StoreResult<Vec<DocumentKind>> {
  let mut stmt = conn.prepare(
    "SELECT kind FROM documents WHERE spec_id = ?1
     ORDER BY CASE kind
       WHEN 'spec' THEN 1 WHEN 'plan' THEN 2 WHEN 'tasks' THEN 3
       WHEN 'research' THEN 4 WHEN 'validation' THEN 5 ELSE 6 END",
  )?;
  let kinds = stmt
    .query_map([spec_id], |r| r.get::<_, String>(0))?
    .collect::<Result<Vec<_>, _>>()?;
  kinds.iter().map(|k| k.parse()).collect()
}

fn summarize(conn: &Connection, row: SpecRow) -> StoreResult<SpecSummary> {
  Ok(SpecSummary {
    kinds: kinds_of(conn, row.id)?,
    number: row.number,
    slug: row.slug,
    status: row.status,
    branch: row.branch,
  })
}

impl Store {
  pub fn list_specs(&self, caller: &Caller, project: &str) -> StoreResult<Vec<SpecSummary>> {
    self.with_conn(|conn| {
      let access = access_in(conn, caller, project)?;
      let mut stmt = conn.prepare(
        "SELECT id, number, slug, status, branch FROM specs WHERE project_id = ?1 ORDER BY number",
      )?;
      let rows = stmt
        .query_map([access.project_id], row_to_spec)?
        .collect::<Result<Vec<_>, _>>()?;
      rows
        .into_iter()
        .map(|raw| summarize(conn, to_spec_row(raw)?))
        .collect()
    })
  }

  pub fn active_spec(&self, caller: &Caller, project: &str) -> StoreResult<Option<SpecSummary>> {
    self.with_conn(|conn| {
      let access = access_in(conn, caller, project)?;
      let raw = conn
        .query_row(
          "SELECT id, number, slug, status, branch FROM specs
           WHERE project_id = ?1 AND status != 'archived' ORDER BY number DESC LIMIT 1",
          [access.project_id],
          row_to_spec,
        )
        .optional()?;
      raw
        .map(|raw| summarize(conn, to_spec_row(raw)?))
        .transpose()
    })
  }
}
