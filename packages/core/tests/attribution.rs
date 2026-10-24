mod common;

use sdd_vault_core::model::{AuditResult, Caller, DocumentKind, Role, Via};

fn acting_through(caller: &Caller, client: &str, origin: Option<&str>) -> Caller {
  Caller {
    via: Via {
      client: client.to_string(),
      origin: origin.map(str::to_string),
    },
    ..caller.clone()
  }
}

struct Fixture {
  store: common::TestStore,
  editor: Caller,
}

fn fixture() -> Fixture {
  let store = common::test_store();
  let root = common::person(&store, "root");
  common::organization(&store, &root, common::ORG);
  let editor = common::person(&store, "editor");
  common::join(&store, &root, common::ORG, &editor);
  store.create_project(&root, "acme/demo", "Demo").unwrap();
  store
    .set_member(&root, "acme/demo", "editor", Role::Editor)
    .unwrap();
  common::exec(
    &store,
    "INSERT INTO specs (project_id, number, slug, status, branch)
     VALUES ((SELECT id FROM projects WHERE slug = 'demo'), 1, 'first', 'draft', '001-first')",
  );
  Fixture { store, editor }
}

#[test]
fn a_write_records_the_client_and_the_origin_on_the_revision_and_the_audit_entry() {
  let f = fixture();
  let agent = acting_through(&f.editor, "Cursor", Some("203.0.113.7"));
  f.store
    .put_document(
      &agent,
      "acme/demo",
      1,
      DocumentKind::Spec,
      "# Spec\n",
      "first",
      None,
    )
    .unwrap();

  let revisions = f
    .store
    .list_revisions(&f.editor, "acme/demo", Some(1), DocumentKind::Spec)
    .unwrap();
  assert_eq!(revisions[0].client.as_deref(), Some("Cursor"));

  let entries = f.store.list_audit(Some("acme/demo"), 10).unwrap();
  let entry = entries.iter().find(|e| e.action == "document.put").unwrap();
  assert_eq!(entry.client.as_deref(), Some("Cursor"));
  assert_eq!(entry.origin.as_deref(), Some("203.0.113.7"));
  assert_eq!(entry.result, AuditResult::Ok);
  assert_eq!(entry.detail, None);
}

#[test]
fn every_kind_of_write_records_the_client() {
  let f = fixture();
  let agent = acting_through(&f.editor, "Claude Code", Some("198.51.100.2"));
  f.store.create_spec(&agent, "acme/demo", "second").unwrap();
  f.store
    .put_document(
      &agent,
      "acme/demo",
      2,
      DocumentKind::Tasks,
      "- [ ] T01 (FR: —) Do it — Done when: ok\n",
      "tasks",
      None,
    )
    .unwrap();
  f.store
    .mark_task_done(&agent, "acme/demo", 2, "T01", &"a".repeat(40))
    .unwrap();
  let entries = f.store.list_audit(Some("acme/demo"), 20).unwrap();
  for action in ["spec.create", "document.put", "task.done"] {
    let entry = entries.iter().find(|e| e.action == action).unwrap();
    assert_eq!(entry.client.as_deref(), Some("Claude Code"), "{action}");
    assert_eq!(entry.origin.as_deref(), Some("198.51.100.2"), "{action}");
  }
  let revisions = f
    .store
    .list_revisions(&f.editor, "acme/demo", Some(2), DocumentKind::Tasks)
    .unwrap();
  assert!(
    revisions
      .iter()
      .all(|r| r.client.as_deref() == Some("Claude Code"))
  );
}

#[test]
fn a_caller_without_a_client_leaves_it_empty() {
  let f = fixture();
  f.store
    .put_document(
      &f.editor,
      "acme/demo",
      1,
      DocumentKind::Spec,
      "# Spec\n",
      "first",
      None,
    )
    .unwrap();
  let revisions = f
    .store
    .list_revisions(&f.editor, "acme/demo", Some(1), DocumentKind::Spec)
    .unwrap();
  assert_eq!(revisions[0].client, None);
  let entries = f.store.list_audit(Some("acme/demo"), 10).unwrap();
  let entry = entries.iter().find(|e| e.action == "document.put").unwrap();
  assert_eq!(entry.client, None);
  assert_eq!(entry.origin, None);
}

#[test]
fn the_current_revision_carries_the_client() {
  let f = fixture();
  let agent = acting_through(&f.editor, "Web app", None);
  f.store
    .put_document(
      &agent,
      "acme/demo",
      1,
      DocumentKind::Spec,
      "# Spec\n",
      "first",
      None,
    )
    .unwrap();
  let document = f
    .store
    .get_document(&f.editor, "acme/demo", 1, DocumentKind::Spec)
    .unwrap();
  assert_eq!(document.revision.client.as_deref(), Some("Web app"));
  let revision = f
    .store
    .get_revision(&f.editor, "acme/demo", document.revision.id)
    .unwrap();
  assert_eq!(revision.meta.client.as_deref(), Some("Web app"));
}

#[test]
fn audit_result_round_trips_through_its_text() {
  for result in AuditResult::ALL {
    assert_eq!(result.as_str().parse::<AuditResult>().unwrap(), result);
  }
  assert!("other".parse::<AuditResult>().is_err());
}
