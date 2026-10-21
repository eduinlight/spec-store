use std::path::Path;
use std::time::Duration;

use diesel::connection::SimpleConnection;
use diesel::r2d2::{ConnectionManager, CustomizeConnection, Pool};
use diesel::sqlite::{Sqlite, SqliteConnection};
use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};

use crate::model::{StoreError, StoreResult};

pub type DbPool = Pool<ConnectionManager<SqliteConnection>>;

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!();
pub const FILE_POOL_SIZE: u32 = 8;
pub const POOL_TIMEOUT: Duration = Duration::from_secs(5);
pub const BUSY_TIMEOUT_MS: u64 = 5000;

#[derive(Debug, Clone, Copy)]
struct ConnectionSettings;

impl CustomizeConnection<SqliteConnection, diesel::r2d2::Error> for ConnectionSettings {
  fn on_acquire(&self, conn: &mut SqliteConnection) -> Result<(), diesel::r2d2::Error> {
    apply_connection_settings(conn).map_err(diesel::r2d2::Error::QueryError)
  }
}

fn apply_connection_settings(conn: &mut SqliteConnection) -> diesel::QueryResult<()> {
  conn.batch_execute(&format!(
    "PRAGMA busy_timeout = {BUSY_TIMEOUT_MS}; PRAGMA foreign_keys = ON;"
  ))
}

pub fn open(path: impl AsRef<Path>) -> StoreResult<DbPool> {
  let url = path.as_ref().to_str().ok_or_else(|| {
    StoreError::Validation(format!(
      "database path {} is not valid UTF-8",
      path.as_ref().display()
    ))
  })?;
  let pool = Pool::builder()
    .max_size(FILE_POOL_SIZE)
    .connection_timeout(POOL_TIMEOUT)
    .connection_customizer(Box::new(ConnectionSettings))
    .build(ConnectionManager::new(url))?;
  {
    let mut conn = pool.get()?;
    conn.batch_execute("PRAGMA journal_mode = WAL")?;
    migrate(&mut conn)?;
  }
  Ok(pool)
}

pub fn open_in_memory() -> StoreResult<DbPool> {
  let pool = Pool::builder()
    .max_size(1)
    .min_idle(Some(1))
    .idle_timeout(None)
    .max_lifetime(None)
    .connection_timeout(POOL_TIMEOUT)
    .connection_customizer(Box::new(ConnectionSettings))
    .build(ConnectionManager::new(":memory:"))?;
  {
    let mut conn = pool.get()?;
    migrate(&mut conn)?;
  }
  Ok(pool)
}

pub fn migrate(conn: &mut SqliteConnection) -> StoreResult<()> {
  conn.immediate_transaction(run_pending_migrations)
}

fn run_pending_migrations(conn: &mut SqliteConnection) -> StoreResult<()> {
  let pending = MigrationHarness::<Sqlite>::pending_migrations(conn, MIGRATIONS).map_err(|e| {
    StoreError::Migration {
      step: "pending".into(),
      detail: e.to_string(),
    }
  })?;
  for migration in pending {
    let step = migration.name().version().to_string();
    conn
      .run_migration(&*migration)
      .map_err(|e| StoreError::Migration {
        step,
        detail: e.to_string(),
      })?;
  }
  Ok(())
}
