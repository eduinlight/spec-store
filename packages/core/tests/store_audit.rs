mod common;

use diesel::connection::SimpleConnection;
use sdd_vault_core::model::{Role, StoreError};
use sdd_vault_core::store::{Store, audit};

fn insert_user(store: &Store, handle: &str) -> i64 {
  common::person(store, handle).user_id
}

fn insert_project(store: &Store, slug: &str) -> i64 {
  let org = common::int(
    store,
    "SELECT count(*) FROM organizations WHERE slug = 'acme'",
  );
  if org == 0 {
    common::exec(
      store,
      "INSERT INTO organizations (slug, name) VALUES ('acme', 'Acme')",
    );
  }
  common::insert(
    store,
    &format!(
      "INSERT INTO projects (organization_id, slug, name) SELECT id, '{slug}', '{slug}' FROM organizations WHERE slug = 'acme' RETURNING id"
    ),
  )
}

fn count_users(store: &Store) -> i64 {
  common::int(store, "SELECT count(*) FROM users")
}

#[test]
fn with_tx_commits_on_ok() {
  let store = common::test_store();
  insert_user(&store, "alice");
  assert_eq!(count_users(&store), 1);
}

#[test]
fn with_tx_rolls_back_on_error() {
  let store = common::test_store();
  let result: Result<(), StoreError> = store.with_tx(|tx| {
    tx.batch_execute(
      "INSERT INTO users (identity_subject, handle, email, name) VALUES ('s', 'bob', 'b@x', 'Bob')",
    )?;
    Err(StoreError::Validation("abort".into()))
  });
  assert!(matches!(result, Err(StoreError::Validation(_))));
  assert_eq!(count_users(&store), 0);
}

#[test]
fn audit_records_actor_project_action_target_and_time() {
  let store = common::test_store();
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
  assert_eq!(entries[1].project.as_deref(), Some("acme/demo"));
  assert_eq!(entries[1].organization.as_deref(), Some("acme"));
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
  let store = common::test_store();
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

  let demo_entries = store.list_audit(Some("acme/demo"), 10).unwrap();
  assert_eq!(demo_entries.len(), 3);
  assert!(
    demo_entries
      .iter()
      .all(|e| e.project.as_deref() == Some("acme/demo"))
  );

  let limited = store.list_audit(None, 2).unwrap();
  assert_eq!(limited.len(), 2);
  assert_eq!(limited[0].target, "x");

  let unknown = store.list_audit(Some("acme/missing"), 10).unwrap();
  assert!(unknown.is_empty());
}

#[test]
fn list_audit_page_is_authorized_and_paged() {
  let store = common::test_store();
  let root = common::person(&store, "root");
  common::organization(&store, &root, common::ORG);
  let owner = common::person(&store, "owner");
  let editor = common::person(&store, "editor");
  let operator = common::operator(&store, "ops");
  common::join(&store, &root, common::ORG, &owner);
  common::join(&store, &root, common::ORG, &editor);
  store.create_project(&root, "acme/demo", "Demo").unwrap();
  store.create_project(&root, "acme/other", "Other").unwrap();
  store
    .set_member(&root, "acme/demo", "owner", Role::Owner)
    .unwrap();
  store
    .set_member(&root, "acme/demo", "editor", Role::Editor)
    .unwrap();
  for i in 0..5 {
    store
      .create_spec(&editor, "acme/demo", &format!("spec-{i}"))
      .unwrap();
  }

  let first = store
    .list_audit_page(&owner, Some("acme/demo"), None, 2)
    .unwrap();
  assert_eq!(first.entries.len(), 2);
  assert!(first.has_more);
  assert!(first.entries[0].id > first.entries[1].id, "newest first");
  assert_eq!(first.entries[0].action, "spec.create");
  assert_eq!(first.entries[0].target, "005-spec-4");
  assert!(
    first
      .entries
      .iter()
      .all(|e| e.project.as_deref() == Some("acme/demo"))
  );

  let mut seen = first.entries.len();
  let mut cursor = first.entries.last().map(|e| e.id);
  let mut has_more = first.has_more;
  while has_more {
    let page = store
      .list_audit_page(&owner, Some("acme/demo"), cursor, 2)
      .unwrap();
    assert!(!page.entries.is_empty());
    assert!(page.entries.iter().all(|e| Some(e.id) < cursor));
    seen += page.entries.len();
    cursor = page.entries.last().map(|e| e.id);
    has_more = page.has_more;
  }
  let total = store.list_audit(Some("acme/demo"), 100).unwrap().len();
  assert_eq!(seen, total, "pagination walks every entry exactly once");
  assert!(
    total >= 7,
    "project.create, two member.set, five spec.create"
  );

  assert!(matches!(
    store.list_audit_page(&owner, None, None, 10),
    Err(StoreError::Forbidden)
  ));
  assert!(matches!(
    store.list_audit_page(&owner, Some("acme/other"), None, 10),
    Err(StoreError::NotFound(_))
  ));
  assert!(matches!(
    store.list_audit_page(&editor, Some("acme/demo"), None, 10),
    Err(StoreError::Forbidden)
  ));

  let global = store.list_audit_page(&operator, None, None, 100).unwrap();
  assert!(!global.has_more);
  assert!(
    global.entries.iter().any(|e| e.project.is_none()),
    "organization entries have no project"
  );
  assert!(
    global
      .entries
      .iter()
      .any(|e| e.project.as_deref() == Some("acme/demo"))
  );
  assert!(
    global
      .entries
      .iter()
      .any(|e| e.project.as_deref() == Some("acme/other"))
  );
  let other = store
    .list_audit_page(&operator, Some("acme/other"), None, 100)
    .unwrap();
  assert!(!other.entries.is_empty());
  assert!(
    other
      .entries
      .iter()
      .all(|e| e.project.as_deref() == Some("acme/other"))
  );
  assert!(matches!(
    store.list_audit_page(&operator, Some("acme/missing"), None, 10),
    Err(StoreError::NotFound(_))
  ));
  assert!(matches!(
    store.list_audit_page(&owner, Some("acme/missing"), None, 10),
    Err(StoreError::NotFound(_))
  ));
}
