mod common;

use spec_store::model::{Caller, DocumentKind, Role, SpecStatus, StoreError};
use spec_store::store::Store;

struct Fixture {
  store: Store,
  admin: Caller,
  editor: Caller,
  outsider: Caller,
}

fn fixture() -> Fixture {
  let store = Store::open(common::temp_db_path("store-specs")).unwrap();
  store.create_user(None, "root", false).unwrap();
  let admin = store.caller_by_handle("root").unwrap();
  for h in ["editor", "outsider"] {
    store.create_user(Some(&admin), h, false).unwrap();
  }
  store.create_project(&admin, "demo", "Demo").unwrap();
  store
    .set_member(&admin, "demo", "editor", Role::Editor)
    .unwrap();
  Fixture {
    editor: store.caller_by_handle("editor").unwrap(),
    outsider: store.caller_by_handle("outsider").unwrap(),
    admin,
    store,
  }
}

fn insert_spec(store: &Store, number: i64, slug: &str, status: &str) -> i64 {
  store
    .with_tx(|tx| {
      tx.execute(
        "INSERT INTO specs (project_id, number, slug, status, branch)
         VALUES ((SELECT id FROM projects WHERE slug = 'demo'), ?1, ?2, ?3, printf('%03d-%s', ?1, ?2))",
        rusqlite::params![number, slug, status],
      )?;
      Ok(tx.last_insert_rowid())
    })
    .unwrap()
}

fn insert_document(store: &Store, spec_id: i64, kind: &str) {
  store
    .with_tx(|tx| {
      tx.execute(
        "INSERT INTO documents (project_id, spec_id, kind)
         VALUES ((SELECT id FROM projects WHERE slug = 'demo'), ?1, ?2)",
        rusqlite::params![spec_id, kind],
      )?;
      Ok(())
    })
    .unwrap();
}

#[test]
fn list_specs_with_kinds_ordered() {
  let f = fixture();
  let second = insert_spec(&f.store, 2, "second", "clarified");
  let first = insert_spec(&f.store, 1, "first", "approved");
  insert_document(&f.store, first, "spec");
  insert_document(&f.store, first, "tasks");
  insert_document(&f.store, second, "spec");

  let specs = f.store.list_specs(&f.editor, "demo").unwrap();
  assert_eq!(specs.len(), 2);
  assert_eq!(specs[0].number, 1);
  assert_eq!(specs[0].slug, "first");
  assert_eq!(specs[0].status, SpecStatus::Approved);
  assert_eq!(specs[0].branch, "001-first");
  assert_eq!(
    specs[0].kinds,
    vec![DocumentKind::Spec, DocumentKind::Tasks]
  );
  assert_eq!(specs[1].number, 2);
  assert_eq!(specs[1].kinds, vec![DocumentKind::Spec]);

  assert!(f.store.list_specs(&f.admin, "demo").unwrap().len() == 2);
  assert!(matches!(
    f.store.list_specs(&f.outsider, "demo"),
    Err(StoreError::Forbidden)
  ));
  assert!(f.store.list_specs(&f.admin, "demo").is_ok());
}

#[test]
fn list_specs_is_empty_for_a_project_without_specs() {
  let f = fixture();
  assert!(f.store.list_specs(&f.editor, "demo").unwrap().is_empty());
}

#[test]
fn active_spec_skips_archived_and_reports_none() {
  let f = fixture();
  assert_eq!(f.store.active_spec(&f.editor, "demo").unwrap(), None);

  insert_spec(&f.store, 1, "first", "approved");
  insert_spec(&f.store, 2, "second", "draft");
  insert_spec(&f.store, 3, "third", "archived");

  let active = f.store.active_spec(&f.editor, "demo").unwrap().unwrap();
  assert_eq!(active.number, 2);
  assert_eq!(active.slug, "second");
  assert_eq!(active.status, SpecStatus::Draft);
  assert_eq!(active.branch, "002-second");

  assert!(matches!(
    f.store.active_spec(&f.outsider, "demo"),
    Err(StoreError::Forbidden)
  ));
}

fn full_fixture() -> (Fixture, Caller, Caller) {
  let f = fixture();
  f.store.create_user(Some(&f.admin), "owner", false).unwrap();
  f.store
    .create_user(Some(&f.admin), "reader", false)
    .unwrap();
  f.store
    .set_member(&f.admin, "demo", "owner", Role::Owner)
    .unwrap();
  f.store
    .set_member(&f.admin, "demo", "reader", Role::Reader)
    .unwrap();
  let owner = f.store.caller_by_handle("owner").unwrap();
  let reader = f.store.caller_by_handle("reader").unwrap();
  (f, owner, reader)
}

#[test]
fn create_spec_assigns_next_number_and_branch() {
  let (f, owner, _) = full_fixture();
  let alpha = f.store.create_spec(&f.editor, "demo", "alpha").unwrap();
  assert_eq!(alpha.number, 1);
  assert_eq!(alpha.slug, "alpha");
  assert_eq!(alpha.branch, "001-alpha");
  assert_eq!(alpha.status, SpecStatus::Draft);

  let beta = f.store.create_spec(&owner, "demo", "beta-two").unwrap();
  assert_eq!(beta.number, 2);
  assert_eq!(beta.branch, "002-beta-two");

  f.store
    .set_spec_status(&owner, "demo", 2, SpecStatus::Archived)
    .unwrap();
  let gamma = f.store.create_spec(&f.editor, "demo", "gamma").unwrap();
  assert_eq!(gamma.number, 3, "archived specs keep their number");

  let listed = f.store.list_specs(&f.editor, "demo").unwrap();
  assert_eq!(listed.len(), 3);
  assert!(
    listed.iter().all(|s| s.kinds.is_empty()),
    "creation writes no document"
  );
  let audit = f.store.list_audit(Some("demo"), 10).unwrap();
  assert!(
    audit
      .iter()
      .any(|e| e.action == "spec.create" && e.target == "003-gamma")
  );
}

#[test]
fn rejects_bad_or_duplicate_slug() {
  let (f, _, reader) = full_fixture();
  f.store.create_spec(&f.editor, "demo", "alpha").unwrap();
  match f.store.create_spec(&f.editor, "demo", "alpha").unwrap_err() {
    StoreError::Validation(msg) => assert!(msg.contains("alpha"), "{msg}"),
    other => panic!("expected validation, got {other:?}"),
  }
  for bad in ["Alpha", "a_b", "one-two-three-four-five", ""] {
    assert!(
      matches!(
        f.store.create_spec(&f.editor, "demo", bad),
        Err(StoreError::Validation(_))
      ),
      "{bad:?}"
    );
  }
  assert!(matches!(
    f.store.create_spec(&reader, "demo", "beta"),
    Err(StoreError::Forbidden)
  ));
  assert!(matches!(
    f.store.create_spec(&f.outsider, "demo", "beta"),
    Err(StoreError::Forbidden)
  ));
  assert!(matches!(
    f.store.create_spec(&f.admin, "demo", "beta"),
    Err(StoreError::Forbidden)
  ));
  assert_eq!(
    f.store.list_specs(&f.editor, "demo").unwrap().len(),
    1,
    "no number consumed"
  );
}

#[test]
fn status_transitions_table() {
  let (f, owner, reader) = full_fixture();
  f.store.create_spec(&f.editor, "demo", "alpha").unwrap();
  let status = |n: usize| f.store.list_specs(&f.editor, "demo").unwrap()[n].status;

  match f
    .store
    .set_spec_status(&f.editor, "demo", 1, SpecStatus::Approved)
    .unwrap_err()
  {
    StoreError::Validation(msg) => assert!(msg.contains("clarified"), "{msg}"),
    other => panic!("expected validation, got {other:?}"),
  }
  f.store
    .set_spec_status(&f.editor, "demo", 1, SpecStatus::Clarified)
    .unwrap();
  assert_eq!(status(0), SpecStatus::Clarified);
  f.store
    .set_spec_status(&f.editor, "demo", 1, SpecStatus::Approved)
    .unwrap();
  f.store
    .set_spec_status(&f.editor, "demo", 1, SpecStatus::Draft)
    .unwrap();
  assert_eq!(status(0), SpecStatus::Draft);

  assert!(matches!(
    f.store
      .set_spec_status(&f.editor, "demo", 1, SpecStatus::Archived),
    Err(StoreError::Validation(_))
  ));
  assert!(matches!(
    f.store
      .set_spec_status(&reader, "demo", 1, SpecStatus::Clarified),
    Err(StoreError::Forbidden)
  ));
  assert!(matches!(
    f.store
      .set_spec_status(&f.editor, "demo", 9, SpecStatus::Clarified),
    Err(StoreError::NotFound(_))
  ));

  f.store
    .set_spec_status(&owner, "demo", 1, SpecStatus::Archived)
    .unwrap();
  assert_eq!(status(0), SpecStatus::Archived);
  assert!(matches!(
    f.store
      .set_spec_status(&owner, "demo", 1, SpecStatus::Draft),
    Err(StoreError::Validation(_))
  ));
  assert_eq!(f.store.active_spec(&f.editor, "demo").unwrap(), None);
  let audit = f.store.list_audit(Some("demo"), 20).unwrap();
  assert!(
    audit
      .iter()
      .any(|e| e.action == "spec.status" && e.target.contains("archived"))
  );
}
