use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use diesel::connection::SimpleConnection;
use diesel::dsl::sql;
use diesel::prelude::*;
use diesel::sql_types::{BigInt, Double, Nullable, Text};
use spec_store_core::store::Store;

static COUNTER: AtomicUsize = AtomicUsize::new(0);

#[allow(dead_code)]
pub fn temp_db_path(label: &str) -> PathBuf {
  let n = COUNTER.fetch_add(1, Ordering::SeqCst);
  let dir = std::env::temp_dir().join("spec-store-tests");
  std::fs::create_dir_all(&dir).expect("temp dir");
  dir.join(format!("{label}-{}-{n}.db", std::process::id()))
}

#[allow(dead_code)]
pub fn temp_dir(label: &str) -> PathBuf {
  let n = COUNTER.fetch_add(1, Ordering::SeqCst);
  let dir = std::env::temp_dir()
    .join("spec-store-tests")
    .join(format!("{label}-{}-{n}", std::process::id()));
  if dir.exists() {
    std::fs::remove_dir_all(&dir).expect("clear temp dir");
  }
  std::fs::create_dir_all(&dir).expect("temp dir");
  dir
}

#[derive(QueryableByName)]
struct TextRow {
  #[diesel(sql_type = Text)]
  value: String,
}

fn scalar_sql<ST: diesel::expression::TypedExpressionType>(
  query: &str,
) -> diesel::expression::SqlLiteral<ST> {
  sql::<ST>(&format!("({query})"))
}

#[allow(dead_code)]
pub fn int(store: &Store, query: &str) -> i64 {
  store
    .with_conn(|c| Ok(diesel::select(scalar_sql::<BigInt>(query)).get_result(c)?))
    .unwrap()
}

#[allow(dead_code)]
pub fn real(store: &Store, query: &str) -> f64 {
  store
    .with_conn(|c| Ok(diesel::select(scalar_sql::<Double>(query)).get_result(c)?))
    .unwrap()
}

#[allow(dead_code)]
pub fn text(store: &Store, query: &str) -> String {
  store
    .with_conn(|c| Ok(diesel::select(scalar_sql::<Text>(query)).get_result(c)?))
    .unwrap()
}

#[allow(dead_code)]
pub fn optional_text(store: &Store, query: &str) -> Option<String> {
  store
    .with_conn(|c| Ok(diesel::select(scalar_sql::<Nullable<Text>>(query)).get_result(c)?))
    .unwrap()
}

#[allow(dead_code)]
pub fn texts(store: &Store, query: &str) -> Vec<String> {
  store
    .with_conn(|c| {
      let rows: Vec<TextRow> = diesel::sql_query(query).load(c)?;
      Ok(rows.into_iter().map(|row| row.value).collect())
    })
    .unwrap()
}

#[allow(dead_code)]
pub fn exec(store: &Store, statements: &str) {
  store.with_tx(|c| Ok(c.batch_execute(statements)?)).unwrap()
}

#[allow(dead_code)]
pub fn insert(store: &Store, statement: &str) -> i64 {
  store
    .with_tx(|c| {
      c.batch_execute(statement)?;
      Ok(diesel::select(sql::<BigInt>("last_insert_rowid()")).get_result(c)?)
    })
    .unwrap()
}

#[derive(QueryableByName)]
struct ForeignKeys {
  #[diesel(sql_type = BigInt)]
  foreign_keys: i64,
}

#[derive(QueryableByName)]
struct BusyTimeout {
  #[diesel(sql_type = BigInt)]
  timeout: i64,
}

#[derive(QueryableByName)]
struct JournalMode {
  #[diesel(sql_type = Text)]
  journal_mode: String,
}

#[allow(dead_code)]
pub fn foreign_keys(conn: &mut SqliteConnection) -> i64 {
  diesel::sql_query("PRAGMA foreign_keys")
    .get_result::<ForeignKeys>(conn)
    .unwrap()
    .foreign_keys
}

#[allow(dead_code)]
pub fn busy_timeout(conn: &mut SqliteConnection) -> i64 {
  diesel::sql_query("PRAGMA busy_timeout")
    .get_result::<BusyTimeout>(conn)
    .unwrap()
    .timeout
}

#[allow(dead_code)]
pub fn journal_mode(conn: &mut SqliteConnection) -> String {
  diesel::sql_query("PRAGMA journal_mode")
    .get_result::<JournalMode>(conn)
    .unwrap()
    .journal_mode
    .to_lowercase()
}

#[allow(dead_code)]
pub fn table_names(conn: &mut SqliteConnection) -> Vec<String> {
  let rows: Vec<TextRow> =
    diesel::sql_query("SELECT name AS value FROM sqlite_master WHERE type = 'table' ORDER BY name")
      .load(conn)
      .unwrap();
  rows.into_iter().map(|row| row.value).collect()
}

#[allow(dead_code)]
pub fn diesel_versions(conn: &mut SqliteConnection) -> Vec<String> {
  let rows: Vec<TextRow> =
    diesel::sql_query("SELECT version AS value FROM __diesel_schema_migrations ORDER BY version")
      .load(conn)
      .unwrap();
  rows.into_iter().map(|row| row.value).collect()
}

pub const PREVIOUS_RUNNER_DDL: &str = "CREATE TABLE IF NOT EXISTS schema_migrations (
  version INTEGER PRIMARY KEY,
  applied_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
)";

pub const PREVIOUS_MIGRATIONS: [(i64, &str, &str); 2] = [
  (
    1,
    include_str!("../../migrations/0001_init/up.sql"),
    "2026-10-17T08:00:00.000Z",
  ),
  (
    2,
    include_str!("../../migrations/0002_web_ui/up.sql"),
    "2026-10-18T09:30:00.000Z",
  ),
];

const RELEASE_ONE_ROWS: &str = "
INSERT INTO users (id, handle, is_admin, created_at) VALUES (1, 'root', 1, '2026-10-17T08:01:00.000Z');
INSERT INTO users (id, handle, is_admin, created_at) VALUES (2, 'alice', 0, '2026-10-17T08:02:00.000Z');
INSERT INTO api_tokens (id, user_id, token_hash, label, created_at, last_used_at, revoked_at)
  VALUES (1, 2, 'hash-one', 'laptop', '2026-10-17T08:03:00.000Z', '2026-10-17T08:04:00.000Z', NULL);
INSERT INTO api_tokens (id, user_id, token_hash, label, created_at, last_used_at, revoked_at)
  VALUES (2, 2, 'hash-two', 'old', '2026-10-17T08:03:30.000Z', NULL, '2026-10-17T08:05:00.000Z');
INSERT INTO projects (id, slug, name, created_at) VALUES (1, 'demo', 'Demo', '2026-10-17T08:06:00.000Z');
INSERT INTO memberships (project_id, user_id, role) VALUES (1, 2, 'owner');
INSERT INTO specs (id, project_id, number, slug, status, branch, created_at)
  VALUES (1, 1, 1, 'first', 'approved', '001-first', '2026-10-17T08:07:00.000Z');
INSERT INTO documents (id, project_id, spec_id, kind, current_revision_id) VALUES (1, 1, NULL, 'constitution', NULL);
INSERT INTO documents (id, project_id, spec_id, kind, current_revision_id) VALUES (2, 1, 1, 'spec', NULL);
INSERT INTO revisions (id, document_id, body, version, author_id, message, created_at)
  VALUES (1, 1, '# Constitution' || char(13) || char(10), '1.0', 2, 'init', '2026-10-17T08:08:00.000Z');
INSERT INTO revisions (id, document_id, body, version, author_id, message, created_at)
  VALUES (2, 2, '# Spec 001 — First', NULL, 2, 'draft', '2026-10-17T08:09:00.000Z');
INSERT INTO revisions (id, document_id, body, version, author_id, message, created_at)
  VALUES (3, 2, '# Spec 001 — First' || char(10) || 'Status: approved', NULL, 2, 'approve', '2026-10-17T08:10:00.000Z');
UPDATE documents SET current_revision_id = 1 WHERE id = 1;
UPDATE documents SET current_revision_id = 3 WHERE id = 2;
INSERT INTO audit_log (id, actor_id, project_id, action, target, created_at)
  VALUES (1, 1, NULL, 'user.create', 'alice', '2026-10-17T08:02:00.000Z');
INSERT INTO audit_log (id, actor_id, project_id, action, target, created_at)
  VALUES (2, 2, 1, 'document.put', '001/spec@3', '2026-10-17T08:10:00.000Z');
";

const RELEASE_TWO_ROWS: &str = "
UPDATE projects SET repository_url = 'https://example.com/demo' WHERE id = 1;
INSERT INTO sessions (id, secret_hash, token_id, user_id, created_at, expires_at, ended_at)
  VALUES (1, 'session-hash', 1, 2, '2026-10-18T10:00:00.000Z', '2026-11-17T10:00:00.000Z', NULL);
";

#[allow(dead_code)]
pub fn raw_connection(path: &std::path::Path) -> SqliteConnection {
  SqliteConnection::establish(path.to_str().unwrap()).unwrap()
}

#[allow(dead_code)]
pub fn previous_release_db(label: &str, migrations: usize) -> PathBuf {
  let path = temp_db_path(label);
  let mut conn = raw_connection(&path);
  conn.batch_execute(PREVIOUS_RUNNER_DDL).unwrap();
  for (version, sql, applied_at) in PREVIOUS_MIGRATIONS.iter().take(migrations) {
    conn.batch_execute(sql).unwrap();
    conn
      .batch_execute(&format!(
        "INSERT INTO schema_migrations (version, applied_at) VALUES ({version}, '{applied_at}')"
      ))
      .unwrap();
  }
  conn.batch_execute(RELEASE_ONE_ROWS).unwrap();
  if migrations >= 2 {
    conn.batch_execute(RELEASE_TWO_ROWS).unwrap();
  }
  path
}

#[allow(dead_code)]
pub fn column_names_of(conn: &mut SqliteConnection, table: &str) -> Vec<String> {
  let rows: Vec<TextRow> = diesel::sql_query(format!(
    "SELECT name AS value FROM pragma_table_info('{table}') ORDER BY cid"
  ))
  .load(conn)
  .unwrap();
  rows.into_iter().map(|row| row.value).collect()
}

#[allow(dead_code)]
pub fn dump_table(conn: &mut SqliteConnection, table: &str, columns: &[String]) -> Vec<String> {
  let row = columns
    .iter()
    .map(|c| format!("quote({c})"))
    .collect::<Vec<_>>()
    .join(" || '|' || ");
  let rows: Vec<TextRow> =
    diesel::sql_query(format!("SELECT {row} AS value FROM {table} ORDER BY rowid"))
      .load(conn)
      .unwrap();
  rows.into_iter().map(|row| row.value).collect()
}

#[allow(dead_code)]
pub fn dump_database(conn: &mut SqliteConnection, tables: &[&str]) -> Vec<(String, Vec<String>)> {
  tables
    .iter()
    .map(|table| {
      let columns = column_names_of(conn, table);
      (table.to_string(), dump_table(conn, table, &columns))
    })
    .collect()
}
