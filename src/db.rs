use std::path::Path;
use std::time::Duration;

use rusqlite::Connection;

use crate::model::StoreResult;

const MIGRATIONS: &[(i64, &str)] = &[
  (1, include_str!("migrations/0001_init.sql")),
  (2, include_str!("migrations/0002_web_ui.sql")),
];

pub fn open(path: impl AsRef<Path>) -> StoreResult<Connection> {
  let conn = Connection::open(path)?;
  let _journal_mode: String = conn.query_row("PRAGMA journal_mode = WAL", [], |r| r.get(0))?;
  configure(&conn)?;
  Ok(conn)
}

pub fn open_in_memory() -> StoreResult<Connection> {
  let conn = Connection::open_in_memory()?;
  configure(&conn)?;
  Ok(conn)
}

fn configure(conn: &Connection) -> StoreResult<()> {
  conn.busy_timeout(Duration::from_secs(5))?;
  conn.pragma_update(None, "foreign_keys", "ON")?;
  migrate(conn)
}

fn migrate(conn: &Connection) -> StoreResult<()> {
  conn.execute_batch(
    "CREATE TABLE IF NOT EXISTS schema_migrations (
      version INTEGER PRIMARY KEY,
      applied_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
    )",
  )?;
  for (version, sql) in MIGRATIONS {
    let applied: bool = conn.query_row(
      "SELECT count(*) FROM schema_migrations WHERE version = ?1",
      [version],
      |r| r.get::<_, i64>(0).map(|n| n > 0),
    )?;
    if applied {
      continue;
    }
    conn.execute_batch("BEGIN")?;
    let result = conn.execute_batch(sql).and_then(|_| {
      conn
        .execute(
          "INSERT INTO schema_migrations (version) VALUES (?1)",
          [version],
        )
        .map(|_| ())
    });
    match result {
      Ok(()) => conn.execute_batch("COMMIT")?,
      Err(e) => {
        conn.execute_batch("ROLLBACK")?;
        return Err(e.into());
      }
    }
  }
  Ok(())
}
