mod common;

use diesel::pg::Pg;
use diesel::prelude::*;
use spec_store_core::schema;

fn declared_columns<Q>(query: Q) -> Vec<String>
where
  Q: diesel::query_builder::QueryFragment<Pg>,
{
  let sql = diesel::debug_query::<Pg, _>(&query).to_string();
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
        .trim_matches('"')
        .to_string()
    })
    .collect()
}

#[test]
fn every_declared_table_matches_the_migrated_schema() {
  let store = common::test_store();
  store
    .with_conn(|conn| {
      let checks: Vec<(&str, Vec<String>)> = vec![
        (
          "users",
          declared_columns(schema::users::table.select(schema::users::all_columns)),
        ),
        (
          "organizations",
          declared_columns(schema::organizations::table.select(schema::organizations::all_columns)),
        ),
        (
          "organization_members",
          declared_columns(
            schema::organization_members::table.select(schema::organization_members::all_columns),
          ),
        ),
        (
          "invitations",
          declared_columns(schema::invitations::table.select(schema::invitations::all_columns)),
        ),
        (
          "projects",
          declared_columns(schema::projects::table.select(schema::projects::all_columns)),
        ),
        (
          "memberships",
          declared_columns(schema::memberships::table.select(schema::memberships::all_columns)),
        ),
        (
          "specs",
          declared_columns(schema::specs::table.select(schema::specs::all_columns)),
        ),
        (
          "documents",
          declared_columns(schema::documents::table.select(schema::documents::all_columns)),
        ),
        (
          "revisions",
          declared_columns(schema::revisions::table.select(schema::revisions::all_columns)),
        ),
        (
          "audit_log",
          declared_columns(schema::audit_log::table.select(schema::audit_log::all_columns)),
        ),
        (
          "browser_sessions",
          declared_columns(
            schema::browser_sessions::table.select(schema::browser_sessions::all_columns),
          ),
        ),
        (
          "login_attempts",
          declared_columns(
            schema::login_attempts::table.select(schema::login_attempts::all_columns),
          ),
        ),
        (
          "email_outbox",
          declared_columns(schema::email_outbox::table.select(schema::email_outbox::all_columns)),
        ),
      ];
      for (table, declared) in checks {
        assert_eq!(
          declared,
          common::column_names_of(conn, table),
          "columns of {table}"
        );
      }
      Ok(())
    })
    .unwrap();
}

#[test]
fn iso_utc_formats_with_milliseconds_and_z() {
  let store = common::test_store();
  let formatted = common::text(
    &store,
    "SELECT iso_utc(timestamptz '2026-10-22 13:04:05.678+02')",
  );
  assert_eq!(formatted, "2026-10-22T11:04:05.678Z");
}
