use diesel::connection::SimpleConnection;
use diesel::prelude::*;
use sdd_vault_testkit::db::database_exists;
use sdd_vault_testkit::{TestDb, with_database};

fn create_notes(url: &str) {
  let mut conn = PgConnection::establish(url).unwrap();
  conn
    .batch_execute(
      "CREATE TABLE notes (id bigint GENERATED ALWAYS AS IDENTITY, body text NOT NULL)",
    )
    .unwrap();
}

#[derive(QueryableByName)]
struct Count {
  #[diesel(sql_type = diesel::sql_types::BigInt)]
  count: i64,
}

#[test]
fn clone_has_the_template_schema_and_is_dropped() {
  let name;
  {
    let db = TestDb::new(create_notes);
    name = db.name.clone();
    let mut conn = db.connect();
    conn
      .batch_execute("INSERT INTO notes (body) VALUES ('a\r\nb')")
      .unwrap();
    let row: Count = diesel::sql_query("SELECT count(*) AS count FROM notes")
      .get_result(&mut conn)
      .unwrap();
    assert_eq!(row.count, 1);
    assert!(database_exists(&name));
  }
  assert!(!database_exists(&name));
}

#[test]
fn clones_are_isolated() {
  let first = TestDb::new(create_notes);
  let second = TestDb::new(create_notes);
  first
    .connect()
    .batch_execute("INSERT INTO notes (body) VALUES ('x')")
    .unwrap();
  let row: Count = diesel::sql_query("SELECT count(*) AS count FROM notes")
    .get_result(&mut second.connect())
    .unwrap();
  assert_eq!(row.count, 0);
}

#[test]
fn empty_database_has_no_tables() {
  let db = TestDb::empty();
  let row: Count = diesel::sql_query(
    "SELECT count(*) AS count FROM information_schema.tables WHERE table_schema = 'public'",
  )
  .get_result(&mut db.connect())
  .unwrap();
  assert_eq!(row.count, 0);
}

#[test]
fn database_url_is_rewritten() {
  assert_eq!(
    with_database("postgres://u:p@h:5432/platform?sslmode=disable", "other"),
    "postgres://u:p@h:5432/other?sslmode=disable"
  );
  assert_eq!(
    with_database("postgres://u:p@h/platform", "x"),
    "postgres://u:p@h/x"
  );
}
