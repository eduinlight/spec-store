mod common;

use spec_store::model::StoreError;
use spec_store::store::{Store, audit};

fn store() -> Store {
  Store::open(common::temp_db_path("store-audit")).expect("open store")
}

fn insert_user(store: &Store, handle: &str) -> i64 {
  store
    .with_tx(|tx| {
      tx.execute(
        "INSERT INTO users (handle, is_admin) VALUES (?1, 1)",
        [handle],
      )?;
      Ok(tx.last_insert_rowid())
    })
    .unwrap()
}

fn insert_project(store: &Store, slug: &str) -> i64 {
  store
    .with_tx(|tx| {
      tx.execute("INSERT INTO projects (slug, name) VALUES (?1, ?1)", [slug])?;
      Ok(tx.last_insert_rowid())
    })
    .unwrap()
}

fn count_users(store: &Store) -> i64 {
  store
    .with_conn(|conn| Ok(conn.query_row("SELECT count(*) FROM users", [], |r| r.get(0))?))
    .unwrap()
}

#[test]
fn with_tx_commits_on_ok() {
  let store = store();
  insert_user(&store, "alice");
  assert_eq!(count_users(&store), 1);
}

#[test]
fn with_tx_rolls_back_on_error() {
  let store = store();
  let result: Result<(), StoreError> = store.with_tx(|tx| {
    tx.execute("INSERT INTO users (handle, is_admin) VALUES ('bob', 0)", [])?;
    Err(StoreError::Validation("abort".into()))
  });
  assert!(matches!(result, Err(StoreError::Validation(_))));
  assert_eq!(count_users(&store), 0);
}

#[test]
fn audit_records_actor_project_action_target_and_time() {
  let store = store();
  let alice = insert_user(&store, "alice");
  let project = insert_project(&store, "demo");
  store
    .with_tx(|tx| audit(tx, alice, Some(project), "project.create", "demo"))
    .unwrap();
  store
    .with_tx(|tx| audit(tx, alice, None, "user.create", "bob"))
    .unwrap();

  let entries = store.list_audit(None, 10).unwrap();
  assert_eq!(entries.len(), 2);
  assert_eq!(entries[0].action, "user.create");
  assert_eq!(entries[0].target, "bob");
  assert_eq!(entries[0].actor, "alice");
  assert_eq!(entries[0].project, None);
  assert_eq!(entries[1].action, "project.create");
  assert_eq!(entries[1].project.as_deref(), Some("demo"));
  assert!(
    entries[1].created_at.ends_with('Z'),
    "{}",
    entries[1].created_at
  );
  assert_eq!(
    entries[1].created_at.len(),
    "2026-10-17T00:00:00.000Z".len()
  );
}

#[test]
fn list_audit_filters_by_project_and_limit() {
  let store = store();
  let alice = insert_user(&store, "alice");
  let demo = insert_project(&store, "demo");
  let other = insert_project(&store, "other");
  for i in 0..3 {
    store
      .with_tx(|tx| audit(tx, alice, Some(demo), "doc.put", &format!("spec-{i}")))
      .unwrap();
  }
  store
    .with_tx(|tx| audit(tx, alice, Some(other), "doc.put", "x"))
    .unwrap();

  let demo_entries = store.list_audit(Some("demo"), 10).unwrap();
  assert_eq!(demo_entries.len(), 3);
  assert!(
    demo_entries
      .iter()
      .all(|e| e.project.as_deref() == Some("demo"))
  );

  let limited = store.list_audit(None, 2).unwrap();
  assert_eq!(limited.len(), 2);
  assert_eq!(limited[0].target, "x");

  let unknown = store.list_audit(Some("missing"), 10).unwrap();
  assert!(unknown.is_empty());
}
