mod common;

use spec_store::model::{Caller, DocumentKind, Role, StoreError};
use spec_store::store::Store;

struct Fixture {
  store: Store,
  owner: Caller,
  editor: Caller,
  reader: Caller,
  outsider: Caller,
}

fn fixture() -> Fixture {
  let store = Store::open(common::temp_db_path("store-documents")).unwrap();
  store.create_user(None, "root", false).unwrap();
  let admin = store.caller_by_handle("root").unwrap();
  for h in ["owner", "editor", "reader", "outsider"] {
    store.create_user(Some(&admin), h, false).unwrap();
  }
  store.create_project(&admin, "demo", "Demo").unwrap();
  store.set_member(&admin, "demo", "owner", Role::Owner).unwrap();
  store
    .set_member(&admin, "demo", "editor", Role::Editor)
    .unwrap();
  store
    .set_member(&admin, "demo", "reader", Role::Reader)
    .unwrap();
  store
    .with_tx(|tx| {
      tx.execute(
        "INSERT INTO specs (project_id, number, slug, status, branch)
         VALUES ((SELECT id FROM projects WHERE slug = 'demo'), 1, 'first', 'draft', '001-first')",
        [],
      )?;
      Ok(())
    })
    .unwrap();
  Fixture {
    owner: store.caller_by_handle("owner").unwrap(),
    editor: store.caller_by_handle("editor").unwrap(),
    reader: store.caller_by_handle("reader").unwrap(),
    outsider: store.caller_by_handle("outsider").unwrap(),
    store,
  }
}

#[test]
fn get_document_and_distinct_not_found() {
  let f = fixture();
  let id = f
    .store
    .put_document(&f.editor, "demo", 1, DocumentKind::Spec, "# Spec 001\n", "initial spec", None)
    .unwrap();

  let doc = f
    .store
    .get_document(&f.reader, "demo", 1, DocumentKind::Spec)
    .unwrap();
  assert_eq!(doc.body, "# Spec 001\n");
  assert_eq!(doc.revision.id, id);
  assert_eq!(doc.revision.author, "editor");
  assert_eq!(doc.revision.message, "initial spec");
  assert!(doc.revision.created_at.ends_with('Z'));

  match f
    .store
    .get_document(&f.reader, "demo", 2, DocumentKind::Spec)
    .unwrap_err()
  {
    StoreError::NotFound(what) => assert!(what.contains("spec 002"), "{what}"),
    other => panic!("expected not found, got {other:?}"),
  }
  match f
    .store
    .get_document(&f.reader, "demo", 1, DocumentKind::Plan)
    .unwrap_err()
  {
    StoreError::NotFound(what) => {
      assert!(what.contains("plan"), "{what}");
      assert!(!what.contains("spec 001 "), "{what} should blame the kind, not the spec");
    }
    other => panic!("expected not found, got {other:?}"),
  }
  assert!(matches!(
    f.store.get_document(&f.reader, "demo", 1, DocumentKind::Constitution),
    Err(StoreError::Validation(_))
  ));
  assert!(matches!(
    f.store.get_document(&f.outsider, "demo", 1, DocumentKind::Spec),
    Err(StoreError::Forbidden)
  ));
}

#[test]
fn get_constitution_and_not_found() {
  let f = fixture();
  match f.store.get_constitution(&f.reader, "demo").unwrap_err() {
    StoreError::NotFound(what) => assert!(what.contains("constitution"), "{what}"),
    other => panic!("expected not found, got {other:?}"),
  }

  let id = f
    .store
    .put_constitution(&f.owner, "demo", "# Constitution\n", "1.0", "ratified", None)
    .unwrap();
  let constitution = f.store.get_constitution(&f.reader, "demo").unwrap();
  assert_eq!(constitution.body, "# Constitution\n");
  assert_eq!(constitution.version, "1.0");
  assert_eq!(constitution.revision.id, id);
  assert_eq!(constitution.revision.author, "owner");
  assert_eq!(constitution.revision.version.as_deref(), Some("1.0"));
  assert!(matches!(
    f.store.get_constitution(&f.outsider, "demo"),
    Err(StoreError::Forbidden)
  ));
}

#[test]
fn put_creates_document_and_revision() {
  let f = fixture();
  let first = f
    .store
    .put_document(&f.editor, "demo", 1, DocumentKind::Tasks, "v1", "first", None)
    .unwrap();
  let second = f
    .store
    .put_document(&f.editor, "demo", 1, DocumentKind::Tasks, "v2", "second", Some(first))
    .unwrap();
  assert!(second > first);

  let doc = f
    .store
    .get_document(&f.editor, "demo", 1, DocumentKind::Tasks)
    .unwrap();
  assert_eq!(doc.body, "v2");
  assert_eq!(doc.revision.id, second);
  assert_eq!(doc.revision.message, "second");

  let (documents, revisions): (i64, i64) = f
    .store
    .with_conn(|c| {
      Ok((
        c.query_row("SELECT count(*) FROM documents", [], |r| r.get(0))?,
        c.query_row("SELECT count(*) FROM revisions", [], |r| r.get(0))?,
      ))
    })
    .unwrap();
  assert_eq!(documents, 1);
  assert_eq!(revisions, 2);

  let audit = f.store.list_audit(Some("demo"), 10).unwrap();
  assert!(audit.iter().any(|e| e.action == "document.put" && e.target.contains("tasks")));

  assert!(matches!(
    f.store.put_document(&f.editor, "demo", 2, DocumentKind::Spec, "x", "m", None),
    Err(StoreError::NotFound(_))
  ));
  assert!(matches!(
    f.store.put_document(&f.editor, "demo", 1, DocumentKind::Constitution, "x", "m", None),
    Err(StoreError::Validation(_))
  ));
}
