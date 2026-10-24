pub mod audit;
pub mod documents;
pub mod invitations;
pub mod organizations;
pub mod outbox;
pub mod projects;
pub mod sessions;
pub mod specs;
pub mod tasks;
pub mod transfer;
pub mod users;

use diesel::pg::PgConnection;
use diesel::prelude::*;

use crate::address::ProjectRef;
use crate::db::{self, DbPool};
use crate::model::{AuditResult, Caller, StoreError, StoreResult, Via};
use crate::schema::{self, iso_utc};
use crate::store::projects::access_in;

pub const PING_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(2);

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
  pub organization: Option<String>,
  pub project: Option<String>,
  pub action: String,
  pub target: String,
  pub created_at: String,
  pub client: Option<String>,
  pub origin: Option<String>,
  pub result: AuditResult,
  pub detail: Option<String>,
}

impl Store {
  pub fn new(pool: DbPool) -> Self {
    Store { pool }
  }

  pub fn open(url: &str) -> StoreResult<Self> {
    Ok(Store::new(db::open(url)?))
  }

  pub fn open_with_size(url: &str, size: u32) -> StoreResult<Self> {
    Ok(Store::new(db::open_with_size(url, size)?))
  }

  pub fn with_conn<T>(
    &self,
    f: impl FnOnce(&mut PgConnection) -> StoreResult<T>,
  ) -> StoreResult<T> {
    let mut conn = self.pool.get()?;
    f(&mut conn)
  }

  pub fn with_tx<T>(&self, f: impl FnOnce(&mut PgConnection) -> StoreResult<T>) -> StoreResult<T> {
    let mut pooled = self.pool.get()?;
    let conn: &mut PgConnection = &mut pooled;
    conn.transaction(f)
  }

  pub fn ping(&self) -> StoreResult<()> {
    let mut conn = self.pool.get_timeout(PING_TIMEOUT)?;
    diesel::sql_query("SELECT 1").execute(&mut conn)?;
    Ok(())
  }

  pub fn list_audit(&self, project: Option<&str>, limit: usize) -> StoreResult<Vec<AuditEntry>> {
    self.with_conn(|conn| {
      let project_id = match project {
        Some(address) => match project_id_by_address(conn, &ProjectRef::parse(address)?)? {
          Some(id) => Some(id),
          None => return Ok(Vec::new()),
        },
        None => None,
      };
      audit_entries(conn, project_id, None, limit as i64)
    })
  }
}

type AuditRow = (
  i64,
  String,
  Option<String>,
  Option<String>,
  String,
  String,
  String,
  Option<String>,
  Option<String>,
  String,
  Option<String>,
);

pub(crate) fn audit_entries(
  conn: &mut PgConnection,
  project_id: Option<i64>,
  before: Option<i64>,
  limit: i64,
) -> StoreResult<Vec<AuditEntry>> {
  filtered_audit_entries(conn, None, project_id, before, limit)
}

pub(crate) fn filtered_audit_entries(
  conn: &mut PgConnection,
  organization_id: Option<i64>,
  project_id: Option<i64>,
  before: Option<i64>,
  limit: i64,
) -> StoreResult<Vec<AuditEntry>> {
  let mut query = schema::audit_log::table
    .inner_join(schema::users::table)
    .left_join(schema::organizations::table)
    .left_join(schema::projects::table)
    .select((
      schema::audit_log::id,
      schema::users::handle,
      schema::organizations::slug.nullable(),
      schema::projects::slug.nullable(),
      schema::audit_log::action,
      schema::audit_log::target,
      iso_utc(schema::audit_log::created_at),
      schema::audit_log::client,
      schema::audit_log::origin,
      schema::audit_log::result,
      schema::audit_log::detail,
    ))
    .order(schema::audit_log::id.desc())
    .limit(limit)
    .into_boxed();
  if let Some(organization_id) = organization_id {
    query = query.filter(schema::audit_log::organization_id.eq(organization_id));
  }
  if let Some(project_id) = project_id {
    query = query.filter(schema::audit_log::project_id.eq(project_id));
  }
  if let Some(before) = before {
    query = query.filter(schema::audit_log::id.lt(before));
  }
  let rows: Vec<AuditRow> = query.load(conn)?;
  rows
    .into_iter()
    .map(
      |(
        id,
        actor,
        organization,
        project,
        action,
        target,
        created_at,
        client,
        origin,
        result,
        detail,
      )| {
        Ok(AuditEntry {
          id,
          project: match (&organization, project) {
            (Some(org), Some(project)) => Some(format!("{org}/{project}")),
            _ => None,
          },
          organization,
          actor,
          action,
          target,
          created_at,
          client,
          origin,
          result: result.parse()?,
          detail,
        })
      },
    )
    .collect()
}

impl Store {
  pub fn list_audit_page(
    &self,
    caller: &Caller,
    project: Option<&str>,
    before: Option<i64>,
    limit: usize,
  ) -> StoreResult<AuditPage> {
    self.with_conn(|conn| {
      let project_id = match project {
        Some(address) => {
          let access = access_in(conn, caller, address)?;
          if !(access.is_owner() || access.is_operator) {
            return Err(StoreError::Forbidden);
          }
          Some(access.project_id)
        }
        None if caller.is_operator => None,
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
  conn: &mut PgConnection,
  actor: &Caller,
  project_id: Option<i64>,
  action: &str,
  target: &str,
) -> StoreResult<()> {
  let organization_id: Option<i64> = match project_id {
    Some(id) => Some(
      schema::projects::table
        .filter(schema::projects::id.eq(id))
        .select(schema::projects::organization_id)
        .first(conn)?,
    ),
    None => None,
  };
  audit_in(conn, actor, organization_id, project_id, action, target)
}

pub fn audit_in(
  conn: &mut PgConnection,
  actor: &Caller,
  organization_id: Option<i64>,
  project_id: Option<i64>,
  action: &str,
  target: &str,
) -> StoreResult<()> {
  insert_audit(
    conn,
    NewAudit {
      actor_id: actor.user_id,
      via: &actor.via,
      organization_id,
      project_id,
      action,
      target,
      result: AuditResult::Ok,
      detail: None,
    },
  )
}

pub(crate) struct NewAudit<'a> {
  pub actor_id: i64,
  pub via: &'a Via,
  pub organization_id: Option<i64>,
  pub project_id: Option<i64>,
  pub action: &'a str,
  pub target: &'a str,
  pub result: AuditResult,
  pub detail: Option<&'a str>,
}

pub(crate) fn insert_audit(conn: &mut PgConnection, entry: NewAudit<'_>) -> StoreResult<()> {
  diesel::insert_into(schema::audit_log::table)
    .values((
      schema::audit_log::actor_id.eq(entry.actor_id),
      schema::audit_log::organization_id.eq(entry.organization_id),
      schema::audit_log::project_id.eq(entry.project_id),
      schema::audit_log::action.eq(entry.action),
      schema::audit_log::target.eq(entry.target),
      schema::audit_log::client.eq(entry.via.client_name()),
      schema::audit_log::origin.eq(entry.via.origin.as_deref()),
      schema::audit_log::result.eq(entry.result.as_str()),
      schema::audit_log::detail.eq(entry.detail),
    ))
    .execute(conn)?;
  Ok(())
}

pub fn project_id_by_address(
  conn: &mut PgConnection,
  address: &ProjectRef,
) -> StoreResult<Option<i64>> {
  Ok(
    schema::projects::table
      .inner_join(schema::organizations::table)
      .filter(schema::organizations::slug.eq(&address.organization))
      .filter(schema::projects::slug.eq(&address.project))
      .select(schema::projects::id)
      .first(conn)
      .optional()?,
  )
}
