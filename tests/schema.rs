mod common;

use diesel::prelude::*;
use diesel::sql_types::{Integer, Text};
use diesel::sqlite::Sqlite;
use spec_store::{db, schema};

#[derive(QueryableByName)]
struct TableColumn {
  #[diesel(sql_type = Text)]
  name: String,
  #[diesel(sql_type = Text)]
  #[diesel(column_name = type_)]
  declared_type: String,
  #[diesel(sql_type = Integer)]
  notnull: i32,
  #[diesel(sql_type = Integer)]
  pk: i32,
}

fn table_info(conn: &mut SqliteConnection, table: &str) -> Vec<TableColumn> {
  diesel::sql_query(format!(
    "SELECT name, type AS type_, \"notnull\", pk FROM pragma_table_info('{table}') ORDER BY cid"
  ))
  .load(conn)
  .unwrap()
}

fn declared_columns<Q>(query: Q) -> Vec<String>
where
  Q: diesel::query_builder::QueryFragment<Sqlite>,
{
  let sql = diesel::debug_query::<Sqlite, _>(&query).to_string();
  let select = sql
    .strip_prefix("SELECT ")
    .and_then(|rest| rest.split(" FROM ").next())
    .unwrap();
  select
    .split(", ")
    .map(|column| {
      column
        .rsplit('.')
        .next()
        .unwrap()
        .trim_matches('`')
        .to_string()
    })
    .collect()
}

fn assert_matches(conn: &mut SqliteConnection, table: &str, declared: Vec<String>) {
  let actual: Vec<String> = table_info(conn, table)
    .into_iter()
    .map(|c| c.name)
    .collect();
  assert_eq!(declared, actual, "columns of {table}");
}

#[test]
fn every_declared_table_matches_the_migrated_schema() {
  let path = common::temp_db_path("schema");
  drop(db::open(&path).expect("open"));
  let mut conn = SqliteConnection::establish(path.to_str().unwrap()).unwrap();
  assert_matches(
    &mut conn,
    "users",
    declared_columns(schema::users::table.select(schema::users::all_columns)),
  );
  assert_matches(
    &mut conn,
    "api_tokens",
    declared_columns(schema::api_tokens::table.select(schema::api_tokens::all_columns)),
  );
  assert_matches(
    &mut conn,
    "projects",
    declared_columns(schema::projects::table.select(schema::projects::all_columns)),
  );
  assert_matches(
    &mut conn,
    "memberships",
    declared_columns(schema::memberships::table.select(schema::memberships::all_columns)),
  );
  assert_matches(
    &mut conn,
    "specs",
    declared_columns(schema::specs::table.select(schema::specs::all_columns)),
  );
  assert_matches(
    &mut conn,
    "documents",
    declared_columns(schema::documents::table.select(schema::documents::all_columns)),
  );
  assert_matches(
    &mut conn,
    "revisions",
    declared_columns(schema::revisions::table.select(schema::revisions::all_columns)),
  );
  assert_matches(
    &mut conn,
    "audit_log",
    declared_columns(schema::audit_log::table.select(schema::audit_log::all_columns)),
  );
  assert_matches(
    &mut conn,
    "sessions",
    declared_columns(schema::sessions::table.select(schema::sessions::all_columns)),
  );
}

#[test]
fn integer_columns_are_big_ints_and_nullability_matches() {
  let path = common::temp_db_path("schema-types");
  drop(db::open(&path).expect("open"));
  let mut conn = SqliteConnection::establish(path.to_str().unwrap()).unwrap();
  let nullable: [(&str, &[&str]); 9] = [
    ("users", &[]),
    ("api_tokens", &["last_used_at", "revoked_at"]),
    ("projects", &["repository_url"]),
    ("memberships", &[]),
    ("specs", &[]),
    ("documents", &["spec_id", "current_revision_id"]),
    ("revisions", &["version"]),
    ("audit_log", &["project_id"]),
    ("sessions", &["ended_at"]),
  ];
  for (table, nullable_columns) in nullable {
    for column in table_info(&mut conn, table) {
      assert!(
        column.declared_type == "INTEGER" || column.declared_type == "TEXT",
        "{table}.{} has type {}",
        column.name,
        column.declared_type
      );
      let may_be_null = column.notnull == 0 && column.pk == 0;
      assert_eq!(
        may_be_null,
        nullable_columns.contains(&column.name.as_str()),
        "nullability of {table}.{}",
        column.name
      );
    }
  }
}
