use std::time::Duration;

use diesel::pg::{Pg, PgConnection};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::sql_types::BigInt;
use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};

use crate::model::{StoreError, StoreResult};

pub type DbPool = Pool<ConnectionManager<PgConnection>>;

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!();
pub const POOL_SIZE: u32 = 10;
pub const POOL_TIMEOUT: Duration = Duration::from_secs(5);
pub const MIGRATION_LOCK_KEY: i64 = 0x7370_6563_7374_6f72;

pub fn open(url: &str) -> StoreResult<DbPool> {
  open_with_size(url, POOL_SIZE)
}

pub fn open_with_size(url: &str, size: u32) -> StoreResult<DbPool> {
  let pool = Pool::builder()
    .max_size(size)
    .connection_timeout(POOL_TIMEOUT)
    .build(ConnectionManager::new(url))?;
  {
    let mut conn = pool.get()?;
    migrate(&mut conn)?;
  }
  Ok(pool)
}

pub fn migrate(conn: &mut PgConnection) -> StoreResult<()> {
  conn.transaction(|conn| {
    diesel::sql_query("SELECT pg_advisory_xact_lock($1)")
      .bind::<BigInt, _>(MIGRATION_LOCK_KEY)
      .execute(conn)
      .map_err(|e| StoreError::Migration {
        step: "lock".into(),
        detail: e.to_string(),
      })?;
    let pending = MigrationHarness::<Pg>::pending_migrations(conn, MIGRATIONS).map_err(|e| {
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
  })
}
