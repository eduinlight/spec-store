#![allow(dead_code)]

use std::ops::Deref;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use diesel::connection::SimpleConnection;
use diesel::dsl::sql;
use diesel::prelude::*;
use diesel::sql_types::{BigInt, Double, Nullable, Text};
use spec_store_core::model::{Caller, OrgRole};
use spec_store_core::store::Store;
use spec_store_core::store::users::Identity;
use spec_store_testkit::TestDb;

static COUNTER: AtomicUsize = AtomicUsize::new(0);

pub const ORG: &str = "acme";

pub fn migrate(url: &str) {
  spec_store_core::db::open_with_size(url, 1).expect("migrate template");
}

pub struct TestStore {
  pub store: Store,
  pub db: TestDb,
}

impl Deref for TestStore {
  type Target = Store;

  fn deref(&self) -> &Store {
    &self.store
  }
}

pub fn test_db() -> TestDb {
  TestDb::new(migrate)
}

pub fn test_store() -> TestStore {
  test_store_with_pool(4)
}

pub fn test_store_with_pool(size: u32) -> TestStore {
  let db = test_db();
  let store = Store::open_with_size(&db.url, size).expect("open store");
  TestStore { store, db }
}

pub fn identity(handle: &str) -> Identity {
  Identity {
    subject: format!("subject-{handle}"),
    handle: handle.to_string(),
    email: format!("{handle}@example.com"),
    name: handle.to_string(),
  }
}

pub fn person(store: &Store, handle: &str) -> Caller {
  store.caller_for(&identity(handle), false).expect("person")
}

pub fn operator(store: &Store, handle: &str) -> Caller {
  store.caller_for(&identity(handle), true).expect("operator")
}

pub fn project(slug: &str) -> String {
  format!("{ORG}/{slug}")
}

pub fn organization(store: &Store, owner: &Caller, slug: &str) {
  store
    .create_organization(owner, slug, slug)
    .expect("create organization");
}

pub fn join(store: &Store, _owner: &Caller, org: &str, member: &Caller) {
  store
    .admit_organization_member(org, member.user_id, OrgRole::Member)
    .expect("admit organization member");
}

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

#[derive(QueryableByName)]
struct IdRow {
  #[diesel(sql_type = BigInt)]
  id: i64,
}

fn scalar_sql<ST: diesel::expression::TypedExpressionType>(
  query: &str,
) -> diesel::expression::SqlLiteral<ST> {
  sql::<ST>(&format!("({query})"))
}

pub fn int(store: &Store, query: &str) -> i64 {
  store
    .with_conn(|c| Ok(diesel::select(scalar_sql::<BigInt>(query)).get_result(c)?))
    .unwrap()
}

pub fn real(store: &Store, query: &str) -> f64 {
  store
    .with_conn(|c| Ok(diesel::select(scalar_sql::<Double>(query)).get_result(c)?))
    .unwrap()
}

pub fn text(store: &Store, query: &str) -> String {
  store
    .with_conn(|c| Ok(diesel::select(scalar_sql::<Text>(query)).get_result(c)?))
    .unwrap()
}

pub fn optional_text(store: &Store, query: &str) -> Option<String> {
  store
    .with_conn(|c| Ok(diesel::select(scalar_sql::<Nullable<Text>>(query)).get_result(c)?))
    .unwrap()
}

pub fn texts(store: &Store, query: &str) -> Vec<String> {
  store
    .with_conn(|c| {
      let rows: Vec<TextRow> = diesel::sql_query(query).load(c)?;
      Ok(rows.into_iter().map(|row| row.value).collect())
    })
    .unwrap()
}

pub fn exec(store: &Store, statements: &str) {
  store.with_tx(|c| Ok(c.batch_execute(statements)?)).unwrap()
}

pub fn insert(store: &Store, statement_returning_id: &str) -> i64 {
  store
    .with_tx(|c| {
      let row: IdRow = diesel::sql_query(statement_returning_id).get_result(c)?;
      Ok(row.id)
    })
    .unwrap()
}

pub fn table_names(conn: &mut PgConnection) -> Vec<String> {
  let rows: Vec<TextRow> = diesel::sql_query(
    "SELECT table_name::text AS value FROM information_schema.tables WHERE table_schema = 'public' ORDER BY table_name",
  )
  .load(conn)
  .unwrap();
  rows.into_iter().map(|row| row.value).collect()
}

pub fn diesel_versions(conn: &mut PgConnection) -> Vec<String> {
  let rows: Vec<TextRow> = diesel::sql_query(
    "SELECT version::text AS value FROM __diesel_schema_migrations ORDER BY version",
  )
  .load(conn)
  .unwrap();
  rows.into_iter().map(|row| row.value).collect()
}

pub fn column_names_of(conn: &mut PgConnection, table: &str) -> Vec<String> {
  let rows: Vec<TextRow> = diesel::sql_query(format!(
    "SELECT column_name::text AS value FROM information_schema.columns WHERE table_schema = 'public' AND table_name = '{table}' ORDER BY ordinal_position"
  ))
  .load(conn)
  .unwrap();
  rows.into_iter().map(|row| row.value).collect()
}
