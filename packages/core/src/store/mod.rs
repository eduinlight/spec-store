pub mod documents;
pub mod projects;
pub mod specs;
pub mod tasks;
pub mod transfer;
pub mod users;

use std::path::Path;

use diesel::dsl::sql;
use diesel::expression::SqlLiteral;
use diesel::prelude::*;
use diesel::sql_types::Text;
use diesel::sqlite::SqliteConnection;

use crate::db::{self, DbPool};
use crate::model::{Caller, StoreError, StoreResult};
use crate::schema;
use crate::store::projects::access_in;

#[derive(Clone)]
pub struct Store {
  pool: DbPool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AuditPage {
  pub entries: Vec<AuditEntry>,
  pub has_more: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditEntry {
  pub id: i64,
  pub actor: String,
  pub project: Option<String>,
  pub action: String,
  pub target: String,
  pub created_at: String,
}

impl Store {
  pub fn new(pool: DbPool) -> Self {
    Store { pool }
  }

  pub fn open(path: impl AsRef<Path>) -> StoreResult<Self> {
    Ok(Store::new(db::open(path)?))
  }

  pub fn in_memory() -> StoreResult<Self> {
    Ok(Store::new(db::open_in_memory()?))
  }

  pub fn with_conn<T>(
    &self,
    f: impl FnOnce(&mut SqliteConnection) -> StoreResult<T>,
  ) -> StoreResult<T> {
    let mut conn = self.pool.get()?;
    f(&mut conn)
  }

  pub fn with_tx<T>(
    &self,
    f: impl FnOnce(&mut SqliteConnection) -> StoreResult<T>,
  ) -> StoreResult<T> {
    let mut conn = self.pool.get()?;
    conn.immediate_transaction(f)
  }

  pub fn list_audit(
    &self,
    project_slug: Option<&str>,
    limit: usize,
  ) -> StoreResult<Vec<AuditEntry>> {
    self.with_conn(|conn| {
      let project_id = match project_slug {
        Some(slug) => match project_id_by_slug(conn, slug)? {
          Some(id) => Some(id),
          None => return Ok(Vec::new()),
        },
        None => None,
      };
      audit_entries(conn, project_id, None, limit as i64)
    })
  }
}

type AuditRow = (i64, String, Option<String>, String, String, String);

fn audit_entries(
  conn: &mut SqliteConnection,
  project_id: Option<i64>,
  before: Option<i64>,
  limit: i64,
) -> StoreResult<Vec<AuditEntry>> {
  let mut query = schema::audit_log::table
    .inner_join(schema::users::table)
    .left_join(schema::projects::table)
    .select((
      schema::audit_log::id,
      schema::users::handle,
      schema::projects::slug.nullable(),
      schema::audit_log::action,
      schema::audit_log::target,
      schema::audit_log::created_at,
    ))
    .order(schema::audit_log::id.desc())
    .limit(limit)
    .into_boxed();
  if let Some(project_id) = project_id {
    query = query.filter(schema::audit_log::project_id.eq(project_id));
  }
  if let Some(before) = before {
    query = query.filter(schema::audit_log::id.lt(before));
  }
  let rows: Vec<AuditRow> = query.load(conn)?;
  Ok(
    rows
      .into_iter()
      .map(
        |(id, actor, project, action, target, created_at)| AuditEntry {
          id,
          actor,
          project,
          action,
          target,
          created_at,
        },
      )
      .collect(),
  )
}

impl Store {
  pub fn list_audit_page(
    &self,
    caller: &Caller,
    project_slug: Option<&str>,
    before: Option<i64>,
    limit: usize,
  ) -> StoreResult<AuditPage> {
    self.with_conn(|conn| {
      let project_id = match project_slug {
        Some(slug) => {
          let access = access_in(conn, caller, slug)?;
          if !access.can_manage_members() {
            return Err(StoreError::Forbidden);
          }
          Some(access.project_id)
        }
        None if caller.is_admin => None,
        None => return Err(StoreError::Forbidden),
      };
      let mut entries = audit_entries(conn, project_id, before, limit as i64 + 1)?;
      let has_more = entries.len() > limit;
      entries.truncate(limit);
      Ok(AuditPage { entries, has_more })
    })
  }
}

pub fn audit(
  conn: &mut SqliteConnection,
  actor_id: i64,
  project_id: Option<i64>,
  action: &str,
  target: &str,
) -> StoreResult<()> {
  diesel::insert_into(schema::audit_log::table)
    .values((
      schema::audit_log::actor_id.eq(actor_id),
      schema::audit_log::project_id.eq(project_id),
      schema::audit_log::action.eq(action),
      schema::audit_log::target.eq(target),
    ))
    .execute(conn)?;
  Ok(())
}

pub fn project_id_by_slug(conn: &mut SqliteConnection, slug: &str) -> StoreResult<Option<i64>> {
  Ok(
    schema::projects::table
      .filter(schema::projects::slug.eq(slug))
      .select(schema::projects::id)
      .first(conn)
      .optional()?,
  )
}

pub(crate) fn now_utc() -> SqlLiteral<Text> {
  sql::<Text>("strftime('%Y-%m-%dT%H:%M:%fZ', 'now')")
}
