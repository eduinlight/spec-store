pub mod documents;
pub mod projects;
pub mod sessions;
pub mod specs;
pub mod tasks;
pub mod transfer;
pub mod users;

use std::path::Path;
use std::sync::{Arc, Mutex};

use rusqlite::{Connection, OptionalExtension, Transaction, params};

use crate::db;
use crate::model::{Caller, StoreError, StoreResult};
use crate::store::projects::access_in;

#[derive(Clone)]
pub struct Store {
  conn: Arc<Mutex<Connection>>,
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
  pub fn new(conn: Connection) -> Self {
    Store {
      conn: Arc::new(Mutex::new(conn)),
    }
  }

  pub fn open(path: impl AsRef<Path>) -> StoreResult<Self> {
    Ok(Store::new(db::open(path)?))
  }

  pub fn in_memory() -> StoreResult<Self> {
    Ok(Store::new(db::open_in_memory()?))
  }

  pub fn with_conn<T>(&self, f: impl FnOnce(&Connection) -> StoreResult<T>) -> StoreResult<T> {
    let conn = self
      .conn
      .lock()
      .unwrap_or_else(|poisoned| poisoned.into_inner());
    f(&conn)
  }

  pub fn with_tx<T>(&self, f: impl FnOnce(&Transaction) -> StoreResult<T>) -> StoreResult<T> {
    let mut conn = self
      .conn
      .lock()
      .unwrap_or_else(|poisoned| poisoned.into_inner());
    let tx = conn.transaction()?;
    let value = f(&tx)?;
    tx.commit()?;
    Ok(value)
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
      let mut stmt = conn.prepare(
        "SELECT a.id, u.handle, p.slug, a.action, a.target, a.created_at
         FROM audit_log a
         JOIN users u ON u.id = a.actor_id
         LEFT JOIN projects p ON p.id = a.project_id
         WHERE (?1 IS NULL OR a.project_id = ?1)
         ORDER BY a.id DESC
         LIMIT ?2",
      )?;
      let rows = stmt.query_map(params![project_id, limit as i64], |row| {
        Ok(AuditEntry {
          id: row.get(0)?,
          actor: row.get(1)?,
          project: row.get(2)?,
          action: row.get(3)?,
          target: row.get(4)?,
          created_at: row.get(5)?,
        })
      })?;
      rows
        .collect::<Result<Vec<_>, _>>()
        .map_err(StoreError::from)
    })
  }
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
      let mut stmt = conn.prepare(
        "SELECT a.id, u.handle, p.slug, a.action, a.target, a.created_at
         FROM audit_log a
         JOIN users u ON u.id = a.actor_id
         LEFT JOIN projects p ON p.id = a.project_id
         WHERE (?1 IS NULL OR a.project_id = ?1)
           AND (?2 IS NULL OR a.id < ?2)
         ORDER BY a.id DESC
         LIMIT ?3",
      )?;
      let rows = stmt.query_map(params![project_id, before, limit as i64 + 1], |row| {
        Ok(AuditEntry {
          id: row.get(0)?,
          actor: row.get(1)?,
          project: row.get(2)?,
          action: row.get(3)?,
          target: row.get(4)?,
          created_at: row.get(5)?,
        })
      })?;
      let mut entries = rows.collect::<Result<Vec<_>, _>>()?;
      let has_more = entries.len() > limit;
      entries.truncate(limit);
      Ok(AuditPage { entries, has_more })
    })
  }
}

pub fn audit(
  tx: &Transaction,
  actor_id: i64,
  project_id: Option<i64>,
  action: &str,
  target: &str,
) -> StoreResult<()> {
  tx.execute(
    "INSERT INTO audit_log (actor_id, project_id, action, target) VALUES (?1, ?2, ?3, ?4)",
    params![actor_id, project_id, action, target],
  )?;
  Ok(())
}

pub fn project_id_by_slug(conn: &Connection, slug: &str) -> StoreResult<Option<i64>> {
  Ok(
    conn
      .query_row("SELECT id FROM projects WHERE slug = ?1", [slug], |r| {
        r.get(0)
      })
      .optional()?,
  )
}
