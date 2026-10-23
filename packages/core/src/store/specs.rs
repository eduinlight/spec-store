use diesel::pg::PgConnection;
use diesel::prelude::*;

use crate::model::{
  Caller, DocumentKind, SpecStatus, StoreError, StoreResult, allowed_targets, transition_allowed,
  validate_slug,
};
use crate::schema::{documents, projects, specs};
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

type RawSpecRow = (i64, i64, String, String, String);

type SpecColumns = (
  specs::id,
  specs::number,
  specs::slug,
  specs::status,
  specs::branch,
);

const SPEC_COLUMNS: SpecColumns = (
  specs::id,
  specs::number,
  specs::slug,
  specs::status,
  specs::branch,
);

fn to_spec_row(raw: RawSpecRow) -> StoreResult<SpecRow> {
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
  conn: &mut PgConnection,
  project_id: i64,
  project_slug: &str,
  number: i64,
) -> StoreResult<SpecRow> {
  let raw: Option<RawSpecRow> = specs::table
    .filter(specs::project_id.eq(project_id))
    .filter(specs::number.eq(number))
    .select(SPEC_COLUMNS)
    .first(conn)
    .optional()?;
  match raw {
    Some(raw) => to_spec_row(raw),
    None => Err(StoreError::NotFound(format!(
      "spec {number:03} in project {project_slug:?}"
    ))),
  }
}

pub fn kinds_of(conn: &mut PgConnection, spec_id: i64) -> StoreResult<Vec<DocumentKind>> {
  let raw: Vec<String> = documents::table
    .filter(documents::spec_id.eq(spec_id))
    .select(documents::kind)
    .load(conn)?;
  let mut kinds = raw
    .iter()
    .map(|k| k.parse())
    .collect::<StoreResult<Vec<DocumentKind>>>()?;
  kinds.sort_by_key(|kind| {
    DocumentKind::SPEC_KINDS
      .iter()
      .position(|k| k == kind)
      .unwrap_or(DocumentKind::SPEC_KINDS.len())
  });
  Ok(kinds)
}

fn summarize(conn: &mut PgConnection, row: SpecRow) -> StoreResult<SpecSummary> {
  Ok(SpecSummary {
    kinds: kinds_of(conn, row.id)?,
    number: row.number,
    slug: row.slug,
    status: row.status,
    branch: row.branch,
  })
}

pub fn list_specs_in(conn: &mut PgConnection, project_id: i64) -> StoreResult<Vec<SpecSummary>> {
  spec_rows_in(conn, project_id)?
    .into_iter()
    .map(|row| summarize(conn, row))
    .collect()
}

pub fn spec_rows_in(conn: &mut PgConnection, project_id: i64) -> StoreResult<Vec<SpecRow>> {
  let rows: Vec<RawSpecRow> = specs::table
    .filter(specs::project_id.eq(project_id))
    .select(SPEC_COLUMNS)
    .order(specs::number)
    .load(conn)?;
  rows.into_iter().map(to_spec_row).collect()
}

pub fn active_spec_in(conn: &mut PgConnection, project_id: i64) -> StoreResult<Option<SpecRow>> {
  let raw: Option<RawSpecRow> = specs::table
    .filter(specs::project_id.eq(project_id))
    .filter(specs::status.ne("archived"))
    .select(SPEC_COLUMNS)
    .order(specs::number.desc())
    .first(conn)
    .optional()?;
  raw.map(to_spec_row).transpose()
}

impl Store {
  pub fn list_specs(&self, caller: &Caller, project: &str) -> StoreResult<Vec<SpecSummary>> {
    self.with_conn(|conn| {
      let access = access_in(conn, caller, project)?;
      list_specs_in(conn, access.project_id)
    })
  }

  pub fn active_spec(&self, caller: &Caller, project: &str) -> StoreResult<Option<SpecSummary>> {
    self.with_conn(|conn| {
      let access = access_in(conn, caller, project)?;
      active_spec_in(conn, access.project_id)?
        .map(|row| summarize(conn, row))
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
      projects::table
        .filter(projects::id.eq(access.project_id))
        .select(projects::id)
        .for_update()
        .first::<i64>(tx)?;
      let duplicate: i64 = specs::table
        .filter(specs::project_id.eq(access.project_id))
        .filter(specs::slug.eq(slug))
        .count()
        .get_result(tx)?;
      if duplicate > 0 {
        return Err(StoreError::Validation(format!(
          "spec slug {slug:?} already exists in project {project:?}"
        )));
      }
      let highest: Option<i64> = specs::table
        .filter(specs::project_id.eq(access.project_id))
        .select(diesel::dsl::max(specs::number))
        .first(tx)?;
      let number = highest.unwrap_or(0) + 1;
      let branch = branch_name(number, slug);
      diesel::insert_into(specs::table)
        .values((
          specs::project_id.eq(access.project_id),
          specs::number.eq(number),
          specs::slug.eq(slug),
          specs::status.eq(SpecStatus::Draft.as_str()),
          specs::branch.eq(&branch),
        ))
        .execute(tx)?;
      audit(
        tx,
        caller.user_id,
        Some(access.project_id),
        "spec.create",
        &branch,
      )?;
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
      diesel::update(specs::table.filter(specs::id.eq(spec.id)))
        .set(specs::status.eq(to.as_str()))
        .execute(tx)?;
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
