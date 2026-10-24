mod common;

use sdd_vault_core::db;
use sdd_vault_core::store::Store;

const EXPECTED_TABLES: [&str; 14] = [
  "__diesel_schema_migrations",
  "audit_log",
  "browser_sessions",
  "documents",
  "email_outbox",
  "invitations",
  "login_attempts",
  "memberships",
  "organization_members",
  "organizations",
  "projects",
  "revisions",
  "specs",
  "users",
];

#[test]
fn open_creates_every_table() {
  let test_db = sdd_vault_testkit::TestDb::empty();
  let pool = db::open(&test_db.url).expect("open");
  let names = common::table_names(&mut pool.get().unwrap());
  assert_eq!(names, EXPECTED_TABLES);
}

#[test]
fn reopening_applies_nothing_new() {
  let test_db = sdd_vault_testkit::TestDb::empty();
  {
    let _first = db::open(&test_db.url).expect("first open");
  }
  let pool = db::open(&test_db.url).expect("second open");
  assert_eq!(
    common::diesel_versions(&mut pool.get().unwrap()),
    vec!["0001", "0002", "0003", "0004"]
  );
}

#[test]
fn store_keeps_its_data_across_opens() {
  let test_db = sdd_vault_testkit::TestDb::empty();
  {
    let store = Store::open(&test_db.url).expect("first open");
    common::person(&store, "root");
  }
  let store = Store::open(&test_db.url).expect("second open");
  assert_eq!(
    store.caller_by_handle("root", false).unwrap().handle,
    "root"
  );
}

#[test]
fn ping_answers_on_a_live_database() {
  let store = common::test_store();
  store.ping().expect("ping");
}

#[test]
fn unreachable_database_is_a_pool_error() {
  let error = Store::open("postgres://nobody:nothing@127.0.0.1:1/none")
    .err()
    .expect("fails");
  assert!(
    matches!(error, sdd_vault_core::model::StoreError::Pool(_)),
    "{error:?}"
  );
}
