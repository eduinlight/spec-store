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
  assert_eq!(specs[0].kinds, vec![DocumentKind::Spec, DocumentKind::Tasks]);
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
