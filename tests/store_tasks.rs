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

const SPEC_WITH_MARKERS: &str = "# Spec 001 — First\n\n## 5. Functional requirements\n- **FR-1**: WHEN x [NEEDS CLARIFICATION: which actor?]\n\n## 12. Open questions\n- [NEEDS CLARIFICATION: Who approves?]\n";
const PLAN_WITH_MARKER: &str =
  "# Plan\n\n## 2. Technical context\n- Unknowns: [NEEDS RESEARCH: rmcp version]\n";

#[test]
fn list_tasks_pending_filter_and_malformed() {
  let f = fixture();
  assert!(matches!(
    f.store.list_tasks(&f.reader, "demo", 1, false),
    Err(StoreError::NotFound(_))
  ));
  put_tasks(&f, &format!("{TASKS}- [ ] T04 broken line\n"));
  f.store
    .mark_task_done(&f.editor, "demo", 1, "T01", "abc1234")
    .ok();

  let all = f.store.list_tasks(&f.reader, "demo", 1, false).unwrap();
  let ids: Vec<&str> = all.tasks.iter().map(|t| t.id.as_str()).collect();
  assert_eq!(ids, vec!["T01", "T02", "T03"]);
  assert_eq!(all.malformed.len(), 1);
  assert_eq!(all.malformed[0].line, 9);
  assert_eq!(all.tasks[1].phase, "Phase 2 — US-1");
  assert!(all.tasks[1].parallel);
  assert_eq!(all.tasks[1].frs, vec!["FR-1", "FR-2"]);
  assert_eq!(all.tasks[1].done_when, "tests written and red");

  let doc = f.store.put_document(
    &f.editor,
    "demo",
    1,
    DocumentKind::Tasks,
    TASKS,
    "clean",
    None,
  );
  assert!(doc.is_err(), "existing document needs expected revision");
  let current = f
    .store
    .get_document(&f.editor, "demo", 1, DocumentKind::Tasks)
    .unwrap()
    .revision
    .id;
  f.store
    .put_document(
      &f.editor,
      "demo",
      1,
      DocumentKind::Tasks,
      TASKS,
      "clean",
      Some(current),
    )
    .unwrap();
  f.store
    .mark_task_done(&f.editor, "demo", 1, "T01", "abc1234")
    .unwrap();
  let pending = f.store.list_tasks(&f.reader, "demo", 1, true).unwrap();
  let ids: Vec<&str> = pending.tasks.iter().map(|t| t.id.as_str()).collect();
  assert_eq!(ids, vec!["T02", "T03"]);
  assert!(pending.malformed.is_empty());
  let all = f.store.list_tasks(&f.reader, "demo", 1, false).unwrap();
  assert!(all.tasks[0].done);
  assert_eq!(all.tasks[0].commit.as_deref(), Some("abc1234"));
}

#[test]
fn list_markers_across_documents() {
  let f = fixture();
  assert!(
    f.store
      .list_markers(&f.reader, "demo", 1)
      .unwrap()
      .is_empty()
  );
  f.store
    .put_document(
      &f.editor,
      "demo",
      1,
      DocumentKind::Plan,
      PLAN_WITH_MARKER,
      "plan",
      None,
    )
    .unwrap();
  f.store
    .put_document(
      &f.editor,
      "demo",
      1,
      DocumentKind::Spec,
      SPEC_WITH_MARKERS,
      "spec",
      None,
    )
    .unwrap();
  put_tasks(&f, TASKS);

  let markers = f.store.list_markers(&f.reader, "demo", 1).unwrap();
  assert_eq!(markers.len(), 3);
  assert_eq!(markers[0].document_kind, DocumentKind::Spec);
  assert_eq!(
    markers[0].kind,
    spec_store::markdown::markers::MarkerKind::Clarification
  );
  assert_eq!(markers[0].heading, "5. Functional requirements");
  assert_eq!(markers[0].text, "which actor?");
  assert_eq!(markers[1].heading, "12. Open questions");
  assert_eq!(markers[2].document_kind, DocumentKind::Plan);
  assert_eq!(
    markers[2].kind,
    spec_store::markdown::markers::MarkerKind::Research
  );
  assert_eq!(markers[2].text, "rmcp version");
  assert!(matches!(
    f.store.list_markers(&f.reader, "demo", 2),
    Err(StoreError::NotFound(_))
  ));
}

#[test]
fn project_status_counts_phase_next_and_markers() {
  let f = fixture();
  let status = f.store.project_status(&f.reader, "demo").unwrap();
  assert_eq!(status.constitution_version, None);
  assert_eq!(status.specs.len(), 1);
  let active = status.active.expect("spec 001 is active");
  assert_eq!(active.number, 1);
  assert_eq!((active.tasks_done, active.tasks_total), (0, 0));
  assert_eq!(active.current_phase, None);
  assert!(active.next_task.is_none());
  assert_eq!(active.markers, 0);

  f.store
    .put_constitution(&f.owner, "demo", "# C\n", "1.0", "ratified", None)
    .unwrap();
  f.store
    .put_document(
      &f.editor,
      "demo",
      1,
      DocumentKind::Spec,
      SPEC_WITH_MARKERS,
      "spec",
      None,
    )
    .unwrap();
  put_tasks(&f, TASKS);
  f.store
    .mark_task_done(&f.editor, "demo", 1, "T01", "abc1234")
    .unwrap();

  let status = f.store.project_status(&f.reader, "demo").unwrap();
  assert_eq!(status.constitution_version.as_deref(), Some("1.0"));
  assert_eq!(
    status.specs[0].kinds,
    vec![DocumentKind::Spec, DocumentKind::Tasks]
  );
  let active = status.active.unwrap();
  assert_eq!((active.tasks_done, active.tasks_total), (1, 3));
  assert_eq!(active.current_phase.as_deref(), Some("Phase 2 — US-1"));
  let next = active.next_task.unwrap();
  assert_eq!(next.id, "T02");
  assert_eq!(next.done_when, "tests written and red");
  assert_eq!(active.markers, 2);

  f.store
    .set_spec_status(&f.owner, "demo", 1, SpecStatus::Archived)
    .unwrap();
  let status = f.store.project_status(&f.reader, "demo").unwrap();
  assert!(status.active.is_none());
  assert_eq!(status.specs.len(), 1);
}

#[test]
fn status_reflects_last_write() {
  let f = fixture();
  put_tasks(&f, TASKS);
  f.store
    .mark_task_done(&f.editor, "demo", 1, "T01", "abc1234")
    .unwrap();
  f.store
    .mark_task_done(&f.editor, "demo", 1, "T02", "abc1235")
    .unwrap();
  let active = f
    .store
    .project_status(&f.editor, "demo")
    .unwrap()
    .active
    .unwrap();
  assert_eq!((active.tasks_done, active.tasks_total), (2, 3));
  assert_eq!(active.next_task.unwrap().id, "T03");

  let current = f
    .store
    .get_document(&f.editor, "demo", 1, DocumentKind::Tasks)
    .unwrap()
    .revision
    .id;
  let all_done = TASKS.replace("- [ ]", "- [x]");
  f.store
    .put_document(
      &f.editor,
      "demo",
      1,
      DocumentKind::Tasks,
      &all_done,
      "all done",
      Some(current),
    )
    .unwrap();
  let active = f
    .store
    .project_status(&f.editor, "demo")
    .unwrap()
    .active
    .unwrap();
  assert_eq!((active.tasks_done, active.tasks_total), (3, 3));
  assert!(active.next_task.is_none());
  assert_eq!(active.current_phase, None);
  assert!(
    f.store
      .list_tasks(&f.editor, "demo", 1, true)
      .unwrap()
      .tasks
      .is_empty()
  );
}

#[test]
fn project_overview_summarises_every_visible_project() {
  let f = fixture();
  let admin = f.store.caller_by_handle("root").unwrap();
  put_tasks(&f, TASKS);
  f.store
    .mark_task_done(&f.editor, "demo", 1, "T01", "abc1234")
    .unwrap();
  f.store
    .put_constitution(&f.owner, "demo", "# C", "1.2", "m", None)
    .unwrap();
  f.store.create_spec(&f.editor, "demo", "second").unwrap();
  f.store
    .put_document(
      &f.editor,
      "demo",
      2,
      DocumentKind::Spec,
      "# S\n\n[NEEDS CLARIFICATION: a?]\n",
      "m",
      None,
    )
    .unwrap();
  f.store.create_project(&admin, "other", "Other").unwrap();
  f.store.create_project(&admin, "third", "Third").unwrap();
  f.store
    .set_member(&admin, "third", "reader", Role::Reader)
    .unwrap();

  let reader_view = f.store.project_overview(&f.reader).unwrap();
  let slugs: Vec<&str> = reader_view.iter().map(|p| p.slug.as_str()).collect();
  assert_eq!(slugs, vec!["demo", "third"]);
  let demo = &reader_view[0];
  assert_eq!(demo.name, "Demo");
  assert_eq!(demo.role, Some(Role::Reader));
  assert_eq!(demo.constitution_version.as_deref(), Some("1.2"));
  assert_eq!(demo.spec_count, 2);
  let active = demo.active.as_ref().expect("active spec");
  assert_eq!(active.number, 2);
  assert_eq!(active.slug, "second");
  assert_eq!(active.tasks_done, 0);
  assert_eq!(active.tasks_total, 0);
  assert_eq!(active.markers, 1);
  let third = &reader_view[1];
  assert!(third.constitution_version.is_none());
  assert_eq!(third.spec_count, 0);
  assert!(third.active.is_none());

  let admin_view = f.store.project_overview(&admin).unwrap();
  let slugs: Vec<&str> = admin_view.iter().map(|p| p.slug.as_str()).collect();
  assert_eq!(slugs, vec!["demo", "other", "third"]);
  assert!(admin_view.iter().all(|p| p.role.is_none()));

  f.store
    .set_spec_status(&f.owner, "demo", 2, SpecStatus::Archived)
    .unwrap();
  let after = f.store.project_overview(&f.editor).unwrap();
  let active = after[0].active.as_ref().unwrap();
  assert_eq!(active.number, 1);
  assert_eq!(active.tasks_done, 1);
  assert_eq!(active.tasks_total, 3);
  assert_eq!(after[0].role, Some(Role::Editor));
}
