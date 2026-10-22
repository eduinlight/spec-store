mod common;

use spec_store::model::StoreError;
use spec_store::store::Store;

const RELEASE_TWO_TABLES: [&str; 10] = [
  "schema_migrations",
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

const RELEASE_ONE_TABLES: [&str; 9] = [
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

fn tables_of(path: &std::path::Path) -> Vec<String> {
  common::table_names(&mut common::raw_connection(path))
}

fn history(path: &std::path::Path) -> Vec<String> {
  let mut conn = common::raw_connection(path);
  let columns = vec!["version".to_string(), "run_on".to_string()];
  common::dump_table(&mut conn, "__diesel_schema_migrations", &columns)
}

#[test]
fn previous_release_opens_unchanged() {
  let path = common::previous_release_db("upgrade-unchanged", 2);
  let before = common::dump_database(&mut common::raw_connection(&path), &RELEASE_TWO_TABLES);
  drop(Store::open(&path).expect("open previous release"));
  let after = common::dump_database(&mut common::raw_connection(&path), &RELEASE_TWO_TABLES);
  assert_eq!(before, after);
  assert_eq!(
    common::diesel_versions(&mut common::raw_connection(&path)),
    vec!["0001", "0002"]
  );
  let store = Store::open(&path).unwrap();
  let alice = store.caller_by_handle("alice").unwrap();
  let constitution = store.get_constitution(&alice, "demo").unwrap();
  assert_eq!(constitution.body, "# Constitution\r\n");
  assert_eq!(constitution.version, "1.0");
}

#[test]
fn carry_over_keeps_apply_times() {
  let path = common::previous_release_db("upgrade-times", 2);
  drop(Store::open(&path).expect("open"));
  assert_eq!(
    history(&path),
    vec![
      "'0001'|'2026-10-17T08:00:00.000Z'",
      "'0002'|'2026-10-18T09:30:00.000Z'"
    ]
  );
}

#[test]
fn release_with_only_first_migration_gets_second() {
  let path = common::previous_release_db("upgrade-partial", 1);
  let mut conn = common::raw_connection(&path);
  let columns: Vec<(String, Vec<String>)> = RELEASE_ONE_TABLES
    .iter()
    .map(|t| (t.to_string(), common::column_names_of(&mut conn, t)))
    .collect();
  let dump = |conn: &mut diesel::SqliteConnection| -> Vec<Vec<String>> {
    columns
      .iter()
      .map(|(table, cols)| common::dump_table(conn, table, cols))
      .collect()
  };
  let before = dump(&mut conn);
  drop(conn);
  drop(Store::open(&path).expect("open"));
  let mut conn = common::raw_connection(&path);
  assert_eq!(dump(&mut conn), before);
  let rows = history(&path);
  assert_eq!(rows.len(), 2, "{rows:?}");
  assert_eq!(rows[0], "'0001'|'2026-10-17T08:00:00.000Z'");
  assert!(rows[1].starts_with("'0002'|"), "{rows:?}");
  assert!(tables_of(&path).iter().any(|t| t == "sessions"));
  assert!(
    common::column_names_of(&mut conn, "projects")
      .iter()
      .any(|c| c == "repository_url")
  );
}

#[test]
fn previous_history_table_is_kept() {
  let path = common::previous_release_db("upgrade-kept", 2);
  let columns = vec!["version".to_string(), "applied_at".to_string()];
  let before = common::dump_table(
    &mut common::raw_connection(&path),
    "schema_migrations",
    &columns,
  );
  drop(Store::open(&path).expect("open"));
  let after = common::dump_table(
    &mut common::raw_connection(&path),
    "schema_migrations",
    &columns,
  );
  assert_eq!(before.len(), 2);
  assert_eq!(before, after);
}

#[test]
fn second_open_changes_no_history() {
  let path = common::previous_release_db("upgrade-twice", 2);
  drop(Store::open(&path).expect("first open"));
  let first = history(&path);
  drop(Store::open(&path).expect("second open"));
  assert_eq!(history(&path), first);
}

#[test]
fn failing_migration_leaves_database_untouched() {
  let path = common::previous_release_db("upgrade-failing", 1);
  {
    use diesel::connection::SimpleConnection;
    common::raw_connection(&path)
      .batch_execute("CREATE TABLE sessions (blocker INTEGER)")
      .unwrap();
  }
  let mut tables = RELEASE_ONE_TABLES.to_vec();
  tables.push("sessions");
  let before = common::dump_database(&mut common::raw_connection(&path), &tables);
  let tables_before = tables_of(&path);
  let result = Store::open(&path);
  match result {
    Err(StoreError::Migration { step, detail }) => {
      assert_eq!(step, "0002");
      assert!(detail.contains("sessions"), "{detail}");
    }
    Err(other) => panic!("expected a migration error, got {other:?}"),
    Ok(_) => panic!("expected a migration error, got a store"),
  }
  assert_eq!(
    common::dump_database(&mut common::raw_connection(&path), &tables),
    before
  );
  assert_eq!(tables_of(&path), tables_before);
}

#[test]
fn empty_previous_history_is_refused() {
  let path = common::previous_release_db("upgrade-empty-history", 2);
  {
    use diesel::connection::SimpleConnection;
    common::raw_connection(&path)
      .batch_execute("DELETE FROM schema_migrations")
      .unwrap();
  }
  let tables_before = tables_of(&path);
  match Store::open(&path) {
    Err(StoreError::Migration { step, detail }) => {
      assert_eq!(step, "carry-over");
      assert!(detail.contains("schema_migrations"), "{detail}");
    }
    Err(other) => panic!("expected a migration error, got {other:?}"),
    Ok(_) => panic!("expected a migration error, got a store"),
  }
  assert_eq!(tables_of(&path), tables_before);
}

#[test]
fn unknown_previous_version_is_ignored() {
  let path = common::previous_release_db("upgrade-unknown", 2);
  {
    use diesel::connection::SimpleConnection;
    common::raw_connection(&path)
      .batch_execute(
        "INSERT INTO schema_migrations (version, applied_at) VALUES (3, '2026-10-19T00:00:00.000Z')",
      )
      .unwrap();
  }
  drop(Store::open(&path).expect("open"));
  assert_eq!(
    common::diesel_versions(&mut common::raw_connection(&path)),
    vec!["0001", "0002"]
  );
}
