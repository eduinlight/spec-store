use std::path::Path;
use std::time::Duration;

use diesel::connection::SimpleConnection;
use diesel::dsl::sql;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, CustomizeConnection, Pool};
use diesel::sql_types::{BigInt, Text};
use diesel::sqlite::{Sqlite, SqliteConnection};
use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};

use crate::model::{StoreError, StoreResult};

pub type DbPool = Pool<ConnectionManager<SqliteConnection>>;

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!();
pub const FILE_POOL_SIZE: u32 = 8;
pub const POOL_TIMEOUT: Duration = Duration::from_secs(5);
pub const BUSY_TIMEOUT_MS: u64 = 5000;

const DIESEL_HISTORY_DDL: &str = "CREATE TABLE IF NOT EXISTS __diesel_schema_migrations (
       version VARCHAR(50) PRIMARY KEY NOT NULL,
       run_on TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);";

const CARRY_OVER: &str = "INSERT OR IGNORE INTO __diesel_schema_migrations (version, run_on)
  SELECT printf('%04d', version), applied_at FROM schema_migrations
  WHERE version IN (1, 2)
  ORDER BY version
  RETURNING version";

const CARRY_OVER_STEP: &str = "carry-over";

#[derive(QueryableByName)]
struct CarriedVersion {
  #[diesel(sql_type = Text)]
  version: String,
}

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
  conn.immediate_transaction(|conn| {
    carry_over_history(conn).map_err(|e| match e {
      StoreError::Database(error) => carry_over_error(error.to_string()),
      other => other,
    })?;
    run_pending_migrations(conn)
  })
}

fn carry_over_error(detail: String) -> StoreError {
  StoreError::Migration {
    step: CARRY_OVER_STEP.into(),
    detail,
  }
}

fn table_exists(conn: &mut SqliteConnection, table: &str) -> StoreResult<bool> {
  let count: i64 = diesel::select(sql::<BigInt>(&format!(
    "(SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = '{table}')"
  )))
  .get_result(conn)?;
  Ok(count > 0)
}

fn carry_over_history(conn: &mut SqliteConnection) -> StoreResult<()> {
  conn.batch_execute(DIESEL_HISTORY_DDL)?;
  if table_exists(conn, "schema_migrations")? {
    let carried: Vec<CarriedVersion> = diesel::sql_query(CARRY_OVER).load(conn)?;
    for row in carried {
      tracing::info!(version = %row.version, "carried over migration from schema_migrations");
    }
  }
  let recorded: i64 = diesel::select(sql::<BigInt>(
    "(SELECT count(*) FROM __diesel_schema_migrations)",
  ))
  .get_result(conn)?;
  if recorded == 0 && table_exists(conn, "users")? {
    return Err(carry_over_error(
      "schema_migrations is empty or missing but the schema already exists".into(),
    ));
  }
  Ok(())
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
