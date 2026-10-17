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
  store
    .set_member(&admin, "demo", "owner", Role::Owner)
    .unwrap();
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
    .put_document(
      &f.editor,
      "demo",
      1,
      DocumentKind::Spec,
      "# Spec 001\n",
      "initial spec",
      None,
    )
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
      assert!(
        !what.contains("spec 001 "),
        "{what} should blame the kind, not the spec"
      );
    }
    other => panic!("expected not found, got {other:?}"),
  }
  assert!(matches!(
    f.store
      .get_document(&f.reader, "demo", 1, DocumentKind::Constitution),
    Err(StoreError::Validation(_))
  ));
  assert!(matches!(
    f.store
      .get_document(&f.outsider, "demo", 1, DocumentKind::Spec),
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
    .put_constitution(
      &f.owner,
      "demo",
      "# Constitution\n",
      "1.0",
      "ratified",
      None,
    )
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
    .put_document(
      &f.editor,
      "demo",
      1,
      DocumentKind::Tasks,
      "v1",
      "first",
      None,
    )
    .unwrap();
  let second = f
    .store
    .put_document(
      &f.editor,
      "demo",
      1,
      DocumentKind::Tasks,
      "v2",
      "second",
      Some(first),
    )
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
  assert!(
    audit
      .iter()
      .any(|e| e.action == "document.put" && e.target.contains("tasks"))
  );

  assert!(matches!(
    f.store
      .put_document(&f.editor, "demo", 2, DocumentKind::Spec, "x", "m", None),
    Err(StoreError::NotFound(_))
  ));
  assert!(matches!(
    f.store.put_document(
      &f.editor,
      "demo",
      1,
      DocumentKind::Constitution,
      "x",
      "m",
      None
    ),
    Err(StoreError::Validation(_))
  ));
}

fn admin(f: &Fixture) -> Caller {
  f.store.caller_by_handle("root").unwrap()
}

#[test]
fn put_requires_matching_expected_revision() {
  let f = fixture();
  let first = f
    .store
    .put_document(
      &f.editor,
      "demo",
      1,
      DocumentKind::Spec,
      "v1",
      "first",
      None,
    )
    .unwrap();
  match f
    .store
    .put_document(
      &f.editor,
      "demo",
      1,
      DocumentKind::Spec,
      "v2",
      "no expected",
      None,
    )
    .unwrap_err()
  {
    StoreError::Conflict { current_revision } => assert_eq!(current_revision, first),
    other => panic!("expected conflict, got {other:?}"),
  }
  let second = f
    .store
    .put_document(
      &f.editor,
      "demo",
      1,
      DocumentKind::Spec,
      "v2",
      "second",
      Some(first),
    )
    .unwrap();
  match f
    .store
    .put_document(
      &f.editor,
      "demo",
      1,
      DocumentKind::Spec,
      "v3",
      "stale",
      Some(first),
    )
    .unwrap_err()
  {
    StoreError::Conflict { current_revision } => assert_eq!(current_revision, second),
    other => panic!("expected conflict, got {other:?}"),
  }
  let doc = f
    .store
    .get_document(&f.editor, "demo", 1, DocumentKind::Spec)
    .unwrap();
  assert_eq!(doc.body, "v2", "nothing stored on conflict");
  assert!(matches!(
    f.store.put_document(
      &f.editor,
      "demo",
      1,
      DocumentKind::Plan,
      "p",
      "m",
      Some(second)
    ),
    Err(StoreError::Validation(_))
  ));
}

#[test]
fn reader_and_archived_cannot_write() {
  let f = fixture();
  assert!(matches!(
    f.store
      .put_document(&f.reader, "demo", 1, DocumentKind::Spec, "x", "m", None),
    Err(StoreError::Forbidden)
  ));
  assert!(matches!(
    f.store
      .put_document(&admin(&f), "demo", 1, DocumentKind::Spec, "x", "m", None),
    Err(StoreError::Forbidden)
  ));
  assert!(matches!(
    f.store
      .put_document(&f.outsider, "demo", 1, DocumentKind::Spec, "x", "m", None),
    Err(StoreError::Forbidden)
  ));

  f.store
    .put_document(&f.editor, "demo", 1, DocumentKind::Spec, "x", "m", None)
    .unwrap();
  f.store
    .set_spec_status(&f.owner, "demo", 1, spec_store::model::SpecStatus::Archived)
    .unwrap();
  assert!(matches!(
    f.store
      .put_document(&f.editor, "demo", 1, DocumentKind::Plan, "p", "m", None),
    Err(StoreError::Forbidden)
  ));
  assert!(matches!(
    f.store
      .put_document(&f.owner, "demo", 1, DocumentKind::Plan, "p", "m", None),
    Err(StoreError::Forbidden)
  ));
  assert_eq!(
    f.store
      .get_document(&f.reader, "demo", 1, DocumentKind::Spec)
      .unwrap()
      .body,
    "x",
    "archived documents stay readable"
  );
}

#[test]
fn constitution_owner_only_and_version_must_change() {
  let f = fixture();
  assert!(matches!(
    f.store
      .put_constitution(&f.editor, "demo", "c", "1.0", "m", None),
    Err(StoreError::Forbidden)
  ));
  assert!(matches!(
    f.store
      .put_constitution(&f.reader, "demo", "c", "1.0", "m", None),
    Err(StoreError::Forbidden)
  ));
  assert!(matches!(
    f.store
      .put_constitution(&admin(&f), "demo", "c", "1.0", "m", None),
    Err(StoreError::Forbidden)
  ));
  let first = f
    .store
    .put_constitution(&f.owner, "demo", "c1", "1.0", "ratified", None)
    .unwrap();
  assert!(matches!(
    f.store
      .put_constitution(&f.owner, "demo", "c2", "1.0", "same version", Some(first)),
    Err(StoreError::Conflict { .. })
  ));
  assert!(matches!(
    f.store
      .put_constitution(&f.owner, "demo", "c2", "1.1", "no expected", None),
    Err(StoreError::Conflict { .. })
  ));
  let second = f
    .store
    .put_constitution(&f.owner, "demo", "c2", "1.1", "amended", Some(first))
    .unwrap();
  let current = f.store.get_constitution(&f.reader, "demo").unwrap();
  assert_eq!(current.version, "1.1");
  assert_eq!(current.revision.id, second);
  let audit = f.store.list_audit(Some("demo"), 10).unwrap();
  assert!(audit.iter().any(|e| e.action == "constitution.put"));
}

#[test]
fn rejects_over_1mb_and_keeps_bytes() {
  let f = fixture();
  let limit = spec_store::model::MAX_BODY_BYTES;
  assert!(matches!(
    f.store.put_document(
      &f.editor,
      "demo",
      1,
      DocumentKind::Spec,
      &"x".repeat(limit + 1),
      "m",
      None
    ),
    Err(StoreError::Validation(_))
  ));
  assert!(
    f.store
      .get_document(&f.editor, "demo", 1, DocumentKind::Spec)
      .is_err()
  );

  let exact = "y".repeat(limit);
  let id = f
    .store
    .put_document(&f.editor, "demo", 1, DocumentKind::Spec, &exact, "m", None)
    .unwrap();
  let body = "# Title\r\n\r\n- [ ] T01 (FR-1) x — Done when: y  \r\nünïcödé → ok\n";
  f.store
    .put_document(
      &f.editor,
      "demo",
      1,
      DocumentKind::Spec,
      body,
      "crlf",
      Some(id),
    )
    .unwrap();
  assert_eq!(
    f.store
      .get_document(&f.editor, "demo", 1, DocumentKind::Spec)
      .unwrap()
      .body,
    body
  );
}

#[test]
fn concurrent_puts_exactly_one_wins() {
  let f = fixture();
  let first = f
    .store
    .put_document(&f.editor, "demo", 1, DocumentKind::Tasks, "base", "m", None)
    .unwrap();
  let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
  let handles: Vec<_> = (0..2)
    .map(|i| {
      let store = f.store.clone();
      let caller = f.editor.clone();
      let barrier = barrier.clone();
      std::thread::spawn(move || {
        barrier.wait();
        store.put_document(
          &caller,
          "demo",
          1,
          DocumentKind::Tasks,
          &format!("w{i}"),
          "race",
          Some(first),
        )
      })
    })
    .collect();
  let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
  let wins = results.iter().filter(|r| r.is_ok()).count();
  let conflicts = results
    .iter()
    .filter(|r| matches!(r, Err(StoreError::Conflict { .. })))
    .count();
  assert_eq!((wins, conflicts), (1, 1), "{results:?}");
  let revisions: i64 = f
    .store
    .with_conn(|c| Ok(c.query_row("SELECT count(*) FROM revisions", [], |r| r.get(0))?))
    .unwrap();
  assert_eq!(revisions, 2);
}

#[test]
fn list_revisions_newest_first() {
  let f = fixture();
  let first = f
    .store
    .put_document(
      &f.editor,
      "demo",
      1,
      DocumentKind::Spec,
      "v1",
      "first",
      None,
    )
    .unwrap();
  let second = f
    .store
    .put_document(
      &f.owner,
      "demo",
      1,
      DocumentKind::Spec,
      "v2",
      "second",
      Some(first),
    )
    .unwrap();

  let revisions = f
    .store
    .list_revisions(&f.reader, "demo", Some(1), DocumentKind::Spec)
    .unwrap();
  assert_eq!(revisions.len(), 2);
  assert_eq!(revisions[0].id, second);
  assert_eq!(revisions[0].author, "owner");
  assert_eq!(revisions[0].message, "second");
  assert_eq!(revisions[0].version, None);
  assert_eq!(revisions[1].id, first);
  assert_eq!(revisions[1].author, "editor");
  assert!(revisions[1].created_at <= revisions[0].created_at);

  let c1 = f
    .store
    .put_constitution(&f.owner, "demo", "c1", "1.0", "ratified", None)
    .unwrap();
  f.store
    .put_constitution(&f.owner, "demo", "c2", "1.1", "amended", Some(c1))
    .unwrap();
  let constitution = f
    .store
    .list_revisions(&f.reader, "demo", None, DocumentKind::Constitution)
    .unwrap();
  assert_eq!(constitution.len(), 2);
  assert_eq!(constitution[0].version.as_deref(), Some("1.1"));
  assert_eq!(constitution[1].version.as_deref(), Some("1.0"));

  assert!(matches!(
    f.store
      .list_revisions(&f.reader, "demo", Some(1), DocumentKind::Plan),
    Err(StoreError::NotFound(_))
  ));
  assert!(matches!(
    f.store
      .list_revisions(&f.reader, "demo", Some(1), DocumentKind::Constitution),
    Err(StoreError::Validation(_))
  ));
  assert!(matches!(
    f.store
      .list_revisions(&f.reader, "demo", None, DocumentKind::Spec),
    Err(StoreError::Validation(_))
  ));
  assert!(matches!(
    f.store
      .list_revisions(&f.outsider, "demo", Some(1), DocumentKind::Spec),
    Err(StoreError::Forbidden)
  ));
}

#[test]
fn get_revision_body_and_not_found() {
  let f = fixture();
  let first = f
    .store
    .put_document(
      &f.editor,
      "demo",
      1,
      DocumentKind::Tasks,
      "v1",
      "first",
      None,
    )
    .unwrap();
  f.store
    .put_document(
      &f.editor,
      "demo",
      1,
      DocumentKind::Tasks,
      "v2",
      "second",
      Some(first),
    )
    .unwrap();

  let revision = f.store.get_revision(&f.reader, "demo", first).unwrap();
  assert_eq!(revision.body, "v1");
  assert_eq!(revision.meta.id, first);
  assert_eq!(revision.meta.author, "editor");
  assert_eq!(revision.meta.message, "first");
  assert_eq!(revision.kind, DocumentKind::Tasks);
  assert_eq!(revision.number, Some(1));

  let c1 = f
    .store
    .put_constitution(&f.owner, "demo", "c1", "1.0", "ratified", None)
    .unwrap();
  let constitution = f.store.get_revision(&f.reader, "demo", c1).unwrap();
  assert_eq!(constitution.kind, DocumentKind::Constitution);
  assert_eq!(constitution.number, None);
  assert_eq!(constitution.meta.version.as_deref(), Some("1.0"));

  assert!(matches!(
    f.store.get_revision(&f.reader, "demo", 999),
    Err(StoreError::NotFound(_))
  ));
  assert!(matches!(
    f.store.get_revision(&f.outsider, "demo", first),
    Err(StoreError::Forbidden)
  ));

  let admin = f.store.caller_by_handle("root").unwrap();
  f.store.create_project(&admin, "other", "Other").unwrap();
  f.store
    .set_member(&admin, "other", "reader", Role::Reader)
    .unwrap();
  assert!(
    matches!(
      f.store.get_revision(&f.reader, "other", first),
      Err(StoreError::NotFound(_))
    ),
    "revisions are scoped to the project they belong to"
  );
}
