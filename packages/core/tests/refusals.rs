mod common;

use sdd_vault_core::model::{AuditResult, Caller, DocumentKind, Role, StoreError, Via};
use sdd_vault_core::store::AuditEntry;

fn acting_through(caller: &Caller, client: &str, origin: &str) -> Caller {
  Caller {
    via: Via {
      client: client.to_string(),
      origin: Some(origin.to_string()),
    },
    ..caller.clone()
  }
}

struct Fixture {
  store: common::TestStore,
  owner: Caller,
  editor: Caller,
  reader: Caller,
  outsider: Caller,
}

fn fixture() -> Fixture {
  let store = common::test_store();
  let root = common::person(&store, "root");
  common::organization(&store, &root, common::ORG);
  let people: Vec<Caller> = ["owner", "editor", "reader"]
    .iter()
    .map(|h| {
      let caller = common::person(&store, h);
      common::join(&store, &root, common::ORG, &caller);
      caller
    })
    .collect();
  store.create_project(&root, "acme/demo", "Demo").unwrap();
  store
    .set_member(&root, "acme/demo", "owner", Role::Owner)
    .unwrap();
  store
    .set_member(&root, "acme/demo", "editor", Role::Editor)
    .unwrap();
  store
    .set_member(&root, "acme/demo", "reader", Role::Reader)
    .unwrap();
  common::exec(
    &store,
    "INSERT INTO specs (project_id, number, slug, status, branch)
     VALUES ((SELECT id FROM projects WHERE slug = 'demo'), 1, 'first', 'draft', '001-first')",
  );
  Fixture {
    outsider: common::person(&store, "outsider"),
    owner: people[0].clone(),
    editor: people[1].clone(),
    reader: people[2].clone(),
    store,
  }
}

fn audit_count(f: &Fixture) -> i64 {
  common::int(&f.store, "SELECT count(*) FROM audit_log")
}

fn revision_count(f: &Fixture) -> i64 {
  common::int(&f.store, "SELECT count(*) FROM revisions")
}

fn entries(f: &Fixture) -> Vec<AuditEntry> {
  f.store.list_audit(Some("acme/demo"), 50).unwrap()
}

#[test]
fn an_editor_writing_the_constitution_is_denied_and_recorded() {
  let f = fixture();
  let agent = acting_through(&f.editor, "Cursor", "203.0.113.9");
  let revisions_before = revision_count(&f);
  let error = f
    .store
    .put_constitution(&agent, "acme/demo", "# C\n", "1.0", "init", None)
    .unwrap_err();
  assert!(matches!(error, StoreError::Forbidden));
  assert_eq!(revision_count(&f), revisions_before);
  let all = entries(&f);
  let entry = all.first().unwrap();
  assert_eq!(entry.action, "constitution.put");
  assert_eq!(entry.result, AuditResult::Denied);
  assert_eq!(entry.actor, "editor");
  assert_eq!(entry.project.as_deref(), Some("acme/demo"));
  assert_eq!(entry.client.as_deref(), Some("Cursor"));
  assert_eq!(entry.origin.as_deref(), Some("203.0.113.9"));
}

#[test]
fn a_reader_writing_a_document_is_denied_and_recorded_with_the_attempted_target() {
  let f = fixture();
  let before = revision_count(&f);
  let error = f
    .store
    .put_document(
      &f.reader,
      "acme/demo",
      1,
      DocumentKind::Spec,
      "# S\n",
      "m",
      None,
    )
    .unwrap_err();
  assert!(matches!(error, StoreError::Forbidden));
  assert_eq!(revision_count(&f), before);
  let all = entries(&f);
  let entry = all.first().unwrap();
  assert_eq!(entry.action, "document.put");
  assert_eq!(entry.target, "001/spec");
  assert_eq!(entry.result, AuditResult::Denied);
  assert_eq!(entry.actor, "reader");
}

#[test]
fn a_stale_document_write_is_a_conflict_naming_both_revisions() {
  let f = fixture();
  let first = f
    .store
    .put_document(
      &f.editor,
      "acme/demo",
      1,
      DocumentKind::Spec,
      "# A\n",
      "a",
      None,
    )
    .unwrap();
  let second = f
    .store
    .put_document(
      &f.editor,
      "acme/demo",
      1,
      DocumentKind::Spec,
      "# B\n",
      "b",
      Some(first),
    )
    .unwrap();
  let revisions_before = revision_count(&f);
  let agent = acting_through(&f.owner, "Claude Code", "198.51.100.4");
  let error = f
    .store
    .put_document(
      &agent,
      "acme/demo",
      1,
      DocumentKind::Spec,
      "# C\n",
      "c",
      Some(first),
    )
    .unwrap_err();
  assert!(matches!(error, StoreError::Conflict { current_revision } if current_revision == second));
  assert_eq!(revision_count(&f), revisions_before);
  let all = entries(&f);
  let entry = all.first().unwrap();
  assert_eq!(entry.action, "document.put");
  assert_eq!(entry.result, AuditResult::Conflict);
  assert_eq!(
    entry.detail,
    Some(format!("expected r{first}, current r{second}"))
  );
  assert_eq!(entry.client.as_deref(), Some("Claude Code"));
  assert_eq!(entry.actor, "owner");
}

#[test]
fn a_constitution_conflict_is_recorded_too() {
  let f = fixture();
  let first = f
    .store
    .put_constitution(&f.owner, "acme/demo", "# C\n", "1.0", "init", None)
    .unwrap();
  let error = f
    .store
    .put_constitution(
      &f.owner,
      "acme/demo",
      "# D\n",
      "1.1",
      "again",
      Some(first + 100),
    )
    .unwrap_err();
  assert!(matches!(error, StoreError::Conflict { .. }));
  let all = entries(&f);
  let entry = all.first().unwrap();
  assert_eq!(entry.action, "constitution.put");
  assert_eq!(entry.result, AuditResult::Conflict);
  assert_eq!(
    entry.detail,
    Some(format!("expected r{}, current r{first}", first + 100))
  );
}

#[test]
fn a_caller_who_is_not_a_member_gets_not_found_and_leaves_no_trace() {
  let f = fixture();
  let before = audit_count(&f);
  let error = f
    .store
    .put_document(
      &f.outsider,
      "acme/demo",
      1,
      DocumentKind::Spec,
      "# S\n",
      "m",
      None,
    )
    .unwrap_err();
  assert!(matches!(error, StoreError::NotFound(_)));
  let error = f
    .store
    .put_constitution(&f.outsider, "acme/demo", "# C\n", "1.0", "m", None)
    .unwrap_err();
  assert!(matches!(error, StoreError::NotFound(_)));
  let error = f
    .store
    .create_project(&f.outsider, "acme/other", "Other")
    .unwrap_err();
  assert!(matches!(error, StoreError::NotFound(_)));
  assert_eq!(audit_count(&f), before);
}

#[test]
fn validation_errors_are_not_recorded() {
  let f = fixture();
  let before = audit_count(&f);
  let error = f
    .store
    .create_spec(&f.editor, "acme/demo", "Not A Slug")
    .unwrap_err();
  assert!(matches!(error, StoreError::Validation(_)));
  let error = f
    .store
    .put_document(
      &f.editor,
      "acme/demo",
      1,
      DocumentKind::Spec,
      "# S\n",
      "m",
      Some(99),
    )
    .unwrap_err();
  assert!(matches!(error, StoreError::Validation(_)));
  assert_eq!(audit_count(&f), before);
}

#[test]
fn successful_writes_keep_result_ok() {
  let f = fixture();
  f.store
    .put_document(
      &f.editor,
      "acme/demo",
      1,
      DocumentKind::Spec,
      "# S\n",
      "m",
      None,
    )
    .unwrap();
  let all = entries(&f);
  assert!(all.iter().all(|e| e.result == AuditResult::Ok));
}

#[test]
fn project_management_refusals_are_recorded() {
  let f = fixture();
  let error = f
    .store
    .set_member(&f.editor, "acme/demo", "reader", Role::Editor)
    .unwrap_err();
  assert!(matches!(error, StoreError::Forbidden));
  let error = f
    .store
    .remove_member(&f.editor, "acme/demo", "reader")
    .unwrap_err();
  assert!(matches!(error, StoreError::Forbidden));
  let error = f
    .store
    .set_repository_url(&f.editor, "acme/demo", Some("https://example.com/r.git"))
    .unwrap_err();
  assert!(matches!(error, StoreError::Forbidden));
  let error = f
    .store
    .create_spec(&f.reader, "acme/demo", "nope")
    .unwrap_err();
  assert!(matches!(error, StoreError::Forbidden));
  let error = f
    .store
    .set_spec_status(
      &f.reader,
      "acme/demo",
      1,
      sdd_vault_core::model::SpecStatus::Approved,
    )
    .unwrap_err();
  assert!(matches!(error, StoreError::Forbidden));
  let error = f
    .store
    .mark_task_done(&f.reader, "acme/demo", 1, "T01", &"a".repeat(40))
    .unwrap_err();
  assert!(matches!(error, StoreError::Forbidden));
  let denied: Vec<String> = entries(&f)
    .into_iter()
    .filter(|e| e.result == AuditResult::Denied)
    .map(|e| e.action)
    .collect();
  for action in [
    "member.set",
    "member.remove",
    "project.repo",
    "spec.create",
    "spec.status",
    "task.done",
  ] {
    assert!(denied.iter().any(|a| a == action), "{action} in {denied:?}");
  }
}

#[test]
fn organization_refusals_are_recorded_against_the_organization() {
  let f = fixture();
  let error = f
    .store
    .create_project(&f.editor, "acme/other", "Other")
    .unwrap_err();
  assert!(matches!(error, StoreError::Forbidden));
  let error = f
    .store
    .create_invitation(&f.editor, "acme", "x@example.com", "en", "https://x")
    .unwrap_err();
  assert!(matches!(error, StoreError::Forbidden));
  let operator = common::operator(&f.store, "ops");
  let page = f
    .store
    .list_organization_audit(&operator, "acme", None, None, 50)
    .unwrap();
  let denied: Vec<&AuditEntry> = page
    .entries
    .iter()
    .filter(|e| e.result == AuditResult::Denied)
    .collect();
  assert_eq!(denied.len(), 2, "{denied:?}");
  assert!(
    denied
      .iter()
      .all(|e| e.organization.as_deref() == Some("acme") && e.project.is_none())
  );
  assert!(
    denied
      .iter()
      .any(|e| e.action == "project.create" && e.target == "acme/other")
  );
  assert!(denied.iter().any(|e| e.action == "invitation.create"));
}

#[test]
fn the_original_error_is_returned_when_recording_fails() {
  let f = fixture();
  common::exec(
    &f.store,
    "ALTER TABLE audit_log ADD CONSTRAINT refuse_everything CHECK (false) NOT VALID",
  );
  let error = f
    .store
    .put_document(
      &f.reader,
      "acme/demo",
      1,
      DocumentKind::Spec,
      "# S\n",
      "m",
      None,
    )
    .unwrap_err();
  assert!(matches!(error, StoreError::Forbidden), "{error:?}");
}
