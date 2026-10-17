mod common;

use spec_store::db;

const EXPECTED_TABLES: [&str; 9] = [
  "schema_migrations",
  "users",
  "api_tokens",
  "projects",
  "memberships",
  "specs",
  "documents",
  "revisions",
  "audit_log",
];

fn table_names(conn: &rusqlite::Connection) -> Vec<String> {
  let mut stmt = conn
    .prepare("SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name")
    .unwrap();
  stmt
    .query_map([], |row| row.get::<_, String>(0))
    .unwrap()
    .map(Result::unwrap)
    .collect()
}

#[test]
fn open_creates_every_table() {
  let path = common::temp_db_path("db-tables");
  let conn = db::open(&path).expect("open");
  let names = table_names(&conn);
  for table in EXPECTED_TABLES {
    assert!(names.iter().any(|n| n == table), "missing table {table}");
  }
}

#[test]
fn open_enables_wal_and_foreign_keys() {
  let path = common::temp_db_path("db-pragmas");
  let conn = db::open(&path).expect("open");
  let journal: String = conn
    .query_row("PRAGMA journal_mode", [], |r| r.get(0))
    .unwrap();
  assert_eq!(journal.to_lowercase(), "wal");
  let fk: i64 = conn
    .query_row("PRAGMA foreign_keys", [], |r| r.get(0))
    .unwrap();
  assert_eq!(fk, 1);
}

#[test]
fn open_is_idempotent() {
  let path = common::temp_db_path("db-idempotent");
  {
    let _first = db::open(&path).expect("first open");
  }
  let conn = db::open(&path).expect("second open");
  let applied: i64 = conn
    .query_row("SELECT count(*) FROM schema_migrations", [], |r| r.get(0))
    .unwrap();
  assert_eq!(applied, 1);
  let version: i64 = conn
    .query_row("SELECT max(version) FROM schema_migrations", [], |r| {
      r.get(0)
    })
    .unwrap();
  assert_eq!(version, 1);
}

#[test]
fn open_in_memory_runs_migrations_too() {
  let conn = db::open_in_memory().expect("memory");
  assert!(table_names(&conn).iter().any(|n| n == "revisions"));
}
