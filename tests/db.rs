mod common;

use diesel::prelude::*;
use diesel::sql_types::Text;
use spec_store::db;
use spec_store::store::Store;

const EXPECTED_TABLES: [&str; 10] = [
  "__diesel_schema_migrations",
  "users",
  "api_tokens",
  "projects",
  "memberships",
  "specs",
  "documents",
  "revisions",
  "audit_log",
  "sessions",
];

#[test]
fn open_creates_every_table() {
  let path = common::temp_db_path("db-tables");
  let pool = db::open(&path).expect("open");
  let names = common::table_names(&mut pool.get().unwrap());
  for table in EXPECTED_TABLES {
    assert!(names.iter().any(|n| n == table), "missing table {table}");
  }
}

#[test]
fn open_enables_wal_and_foreign_keys() {
  let path = common::temp_db_path("db-pragmas");
  let pool = db::open(&path).expect("open");
  let mut conn = pool.get().unwrap();
  assert_eq!(common::journal_mode(&mut conn), "wal");
  assert_eq!(common::foreign_keys(&mut conn), 1);
}

#[test]
fn open_records_diesel_history_only() {
  let path = common::temp_db_path("db-idempotent");
  {
    let _first = db::open(&path).expect("first open");
  }
  let pool = db::open(&path).expect("second open");
  let mut conn = pool.get().unwrap();
  assert_eq!(common::diesel_versions(&mut conn), vec!["0001", "0002"]);
  assert!(
    !common::table_names(&mut conn)
      .iter()
      .any(|n| n == "schema_migrations"),
    "a new database has no previous history table"
  );
}

#[derive(QueryableByName)]
struct ColumnName {
  #[diesel(sql_type = Text)]
  name: String,
}

fn column_names(conn: &mut SqliteConnection, table: &str) -> Vec<String> {
  diesel::sql_query(format!("SELECT name FROM pragma_table_info('{table}')"))
    .load::<ColumnName>(conn)
    .unwrap()
    .into_iter()
    .map(|c| c.name)
    .collect()
}

#[test]
fn second_migration_adds_sessions_and_repository_url() {
  let pool = db::open_in_memory().expect("memory");
  let mut conn = pool.get().unwrap();
  let sessions = column_names(&mut conn, "sessions");
  for column in [
    "id",
    "secret_hash",
    "token_id",
    "user_id",
    "created_at",
    "expires_at",
    "ended_at",
  ] {
    assert!(
      sessions.iter().any(|c| c == column),
      "sessions lacks {column}: {sessions:?}"
    );
  }
  let projects = column_names(&mut conn, "projects");
  assert!(
    projects.iter().any(|c| c == "repository_url"),
    "{projects:?}"
  );
}

#[test]
fn open_in_memory_runs_migrations_too() {
  let pool = db::open_in_memory().expect("memory");
  let mut conn = pool.get().unwrap();
  assert!(
    common::table_names(&mut conn)
      .iter()
      .any(|n| n == "revisions")
  );
  assert_eq!(common::diesel_versions(&mut conn), vec!["0001", "0002"]);
}

#[test]
fn memory_store_has_connection_settings() {
  let store = Store::in_memory().expect("memory");
  let (foreign_keys, busy_timeout) = store
    .with_conn(|conn| Ok((common::foreign_keys(conn), common::busy_timeout(conn))))
    .unwrap();
  assert_eq!(foreign_keys, 1);
  assert_eq!(busy_timeout, 5000);
}

#[test]
fn file_store_keeps_its_data_across_opens() {
  let path = common::temp_db_path("db-reopen");
  {
    let store = Store::open(&path).expect("first open");
    store.create_user(None, "root", true).unwrap();
  }
  let store = Store::open(&path).expect("second open");
  assert!(store.caller_by_handle("root").unwrap().is_admin);
}

#[test]
fn new_file_and_memory_stores_record_both_migrations() {
  let path = common::temp_db_path("db-new-store");
  for store in [
    Store::open(&path).expect("file"),
    Store::in_memory().expect("memory"),
  ] {
    let (versions, tables) = store
      .with_conn(|conn| Ok((common::diesel_versions(conn), common::table_names(conn))))
      .unwrap();
    assert_eq!(versions, vec!["0001", "0002"]);
    assert!(
      !tables.iter().any(|t| t == "schema_migrations"),
      "{tables:?}"
    );
    for table in EXPECTED_TABLES {
      assert!(tables.iter().any(|t| t == table), "missing {table}");
    }
  }
}
