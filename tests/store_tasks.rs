mod common;

use spec_store::model::{Caller, DocumentKind, Role, SpecStatus, StoreError};
use spec_store::store::Store;

const TASKS: &str = "# Tasks — Spec 001

## Phase 1 — Setup
- [ ] T01 (FR: —) Create the crate — Done when: `cargo build` passes

## Phase 2 — US-1
- [ ] T02 [P] (FR-1, FR-2) Write tests — Done when: tests written and red
- [ ] T03 (FR-1, FR-2) Implement — Done when: T02 green
";

struct Fixture {
  store: Store,
  owner: Caller,
  editor: Caller,
  reader: Caller,
}

fn fixture() -> Fixture {
  let store = Store::open(common::temp_db_path("store-tasks")).unwrap();
  store.create_user(None, "root", false).unwrap();
  let admin = store.caller_by_handle("root").unwrap();
  for h in ["owner", "editor", "reader"] {
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
  let editor = store.caller_by_handle("editor").unwrap();
  store.create_spec(&editor, "demo", "first").unwrap();
  Fixture {
    owner: store.caller_by_handle("owner").unwrap(),
    reader: store.caller_by_handle("reader").unwrap(),
    editor,
    store,
  }
}

fn put_tasks(f: &Fixture, body: &str) -> i64 {
  f.store
    .put_document(
      &f.editor,
      "demo",
      1,
      DocumentKind::Tasks,
      body,
      "tasks",
      None,
    )
    .unwrap()
}

#[test]
fn mark_done_rewrites_line_with_commit() {
  let f = fixture();
  let first = put_tasks(&f, TASKS);
  let marked = f
    .store
    .mark_task_done(&f.editor, "demo", 1, "T01", "abc1234")
    .unwrap();
  assert!(marked.revision_id > first);
  assert_eq!(
    marked.line,
    "- [x] T01 (FR: —) Create the crate — Done when: `cargo build` passes · commit: abc1234"
  );

  let doc = f
    .store
    .get_document(&f.reader, "demo", 1, DocumentKind::Tasks)
    .unwrap();
  assert_eq!(doc.revision.id, marked.revision_id);
  assert!(doc.body.contains(&marked.line), "{}", doc.body);
  assert!(doc.body.contains("- [ ] T02 [P]"), "other lines untouched");
  assert!(
    doc.revision.message.contains("T01"),
    "{}",
    doc.revision.message
  );
  assert_eq!(doc.revision.author, "editor");

  let audit = f.store.list_audit(Some("demo"), 10).unwrap();
  assert!(
    audit
      .iter()
      .any(|e| e.action == "task.done" && e.target.contains("T01") && e.target.contains("abc1234"))
  );

  f.store
    .mark_task_done(&f.owner, "demo", 1, "T02", &"f".repeat(40))
    .unwrap();
}

#[test]
fn mark_done_rejects_unknown_done_or_bad_commit() {
  let f = fixture();
  assert!(matches!(
    f.store
      .mark_task_done(&f.editor, "demo", 1, "T01", "abc1234"),
    Err(StoreError::NotFound(_))
  ));
  put_tasks(&f, TASKS);
  assert!(matches!(
    f.store
      .mark_task_done(&f.editor, "demo", 1, "T09", "abc1234"),
    Err(StoreError::NotFound(_))
  ));
  for bad in ["xyz", "ABC1234", "abc123", &"a".repeat(41)] {
    assert!(
      matches!(
        f.store.mark_task_done(&f.editor, "demo", 1, "T01", bad),
        Err(StoreError::Validation(_))
      ),
      "{bad:?}"
    );
  }
  f.store
    .mark_task_done(&f.editor, "demo", 1, "T01", "abc1234")
    .unwrap();
  match f
    .store
    .mark_task_done(&f.editor, "demo", 1, "T01", "abc1234")
    .unwrap_err()
  {
    StoreError::Validation(msg) => assert!(msg.contains("already"), "{msg}"),
    other => panic!("expected validation, got {other:?}"),
  }
  assert!(matches!(
    f.store
      .mark_task_done(&f.reader, "demo", 1, "T02", "abc1234"),
    Err(StoreError::Forbidden)
  ));
  assert!(matches!(
    f.store
      .mark_task_done(&f.editor, "demo", 2, "T02", "abc1234"),
    Err(StoreError::NotFound(_))
  ));

  let revisions_before: i64 = f
    .store
    .with_conn(|c| Ok(c.query_row("SELECT count(*) FROM revisions", [], |r| r.get(0))?))
    .unwrap();
  f.store
    .set_spec_status(&f.owner, "demo", 1, SpecStatus::Archived)
    .unwrap();
  assert!(matches!(
    f.store
      .mark_task_done(&f.editor, "demo", 1, "T02", "abc1234"),
    Err(StoreError::Forbidden)
  ));
  let revisions_after: i64 = f
    .store
    .with_conn(|c| Ok(c.query_row("SELECT count(*) FROM revisions", [], |r| r.get(0))?))
    .unwrap();
  assert_eq!(
    revisions_before, revisions_after,
    "rejected marks store nothing"
  );
}

#[test]
fn mark_done_refuses_malformed_or_duplicate_task_lines() {
  let f = fixture();
  let doc = put_tasks(
    &f,
    &format!("{TASKS}- [ ] T03 (FR-1) duplicate — Done when: x\n"),
  );
  assert!(matches!(
    f.store
      .mark_task_done(&f.editor, "demo", 1, "T03", "abc1234"),
    Err(StoreError::Malformed(_))
  ));
  f.store
    .put_document(
      &f.editor,
      "demo",
      1,
      DocumentKind::Tasks,
      &format!("{TASKS}- [ ] T04 broken\n"),
      "m",
      Some(doc),
    )
    .unwrap();
  assert!(matches!(
    f.store
      .mark_task_done(&f.editor, "demo", 1, "T01", "abc1234"),
    Err(StoreError::Malformed(_))
  ));
}
