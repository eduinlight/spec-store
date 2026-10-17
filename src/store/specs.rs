use rusqlite::{Connection, OptionalExtension, params};

use crate::model::{
  Caller, DocumentKind, SpecStatus, StoreError, StoreResult, allowed_targets, transition_allowed,
  validate_slug,
};
use crate::store::projects::access_in;
use crate::store::{Store, audit};

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreatedSpec {
  pub number: i64,
  pub slug: String,
  pub branch: String,
  pub status: SpecStatus,
}

impl Store {
  pub fn create_spec(
    &self,
    caller: &Caller,
    project: &str,
    slug: &str,
  ) -> StoreResult<CreatedSpec> {
    self.with_tx(|tx| {
      let access = access_in(tx, caller, project)?;
      if !access.can_write() {
        return Err(StoreError::Forbidden);
      }
      validate_slug(slug)?;
      let duplicate: i64 = tx.query_row(
        "SELECT count(*) FROM specs WHERE project_id = ?1 AND slug = ?2",
        params![access.project_id, slug],
        |r| r.get(0),
      )?;
      if duplicate > 0 {
        return Err(StoreError::Validation(format!(
          "spec slug {slug:?} already exists in project {project:?}"
        )));
      }
      let highest: Option<i64> = tx.query_row(
        "SELECT max(number) FROM specs WHERE project_id = ?1",
        [access.project_id],
        |r| r.get(0),
      )?;
      let number = highest.unwrap_or(0) + 1;
      let branch = branch_name(number, slug);
      tx.execute(
        "INSERT INTO specs (project_id, number, slug, status, branch) VALUES (?1, ?2, ?3, 'draft', ?4)",
        params![access.project_id, number, slug, branch],
      )?;
      audit(tx, caller.user_id, Some(access.project_id), "spec.create", &branch)?;
      Ok(CreatedSpec {
        number,
        slug: slug.to_string(),
        branch,
        status: SpecStatus::Draft,
      })
    })
  }

  pub fn set_spec_status(
    &self,
    caller: &Caller,
    project: &str,
    number: i64,
    to: SpecStatus,
  ) -> StoreResult<SpecStatus> {
    self.with_tx(|tx| {
      let access = access_in(tx, caller, project)?;
      let role = access
        .role
        .filter(|r| r.can_write())
        .ok_or(StoreError::Forbidden)?;
      let spec = spec_by_number(tx, access.project_id, project, number)?;
      if !transition_allowed(spec.status, to, role) {
        let targets: Vec<&str> = allowed_targets(spec.status, role)
          .iter()
          .map(|s| s.as_str())
          .collect();
        let allowed = if targets.is_empty() {
          "none".to_string()
        } else {
          targets.join(", ")
        };
        return Err(StoreError::Validation(format!(
          "cannot change spec {number:03} from {} to {to}; allowed targets for a {role}: {allowed}",
          spec.status
        )));
      }
      tx.execute(
        "UPDATE specs SET status = ?1 WHERE id = ?2",
        params![to.as_str(), spec.id],
      )?;
      audit(
        tx,
        caller.user_id,
        Some(access.project_id),
        "spec.status",
        &format!("{number:03}:{}→{to}", spec.status),
      )?;
      Ok(to)
    })
  }
}
