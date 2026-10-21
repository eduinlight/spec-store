use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use diesel::connection::SimpleConnection;
use diesel::dsl::sql;
use diesel::prelude::*;
use diesel::sql_types::{BigInt, Double, Nullable, Text};
use spec_store::store::Store;

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
