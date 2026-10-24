mod common;

use sdd_vault_core::model::{AuditResult, Caller, DocumentKind, Role, SpecStatus, StoreError, Via};
use sdd_vault_core::store::AuditEntry;
use sdd_vault_core::store::transfer::ImportOptions;
use sdd_vault_core::tree::ProjectTree;

const CONSTITUTION: &str = "# Constitution\n\nVersion 1.2\n";
const SPEC: &str = "# Spec 002\n\n- **FR-1**: THE SYSTEM SHALL work\n";
const TASKS: &str = "## Phase\n- [x] T01 (FR: —) One — Done when: ok · commit: abc1234\n- [ ] T02 (FR: —) Two — Done when: ok\n";

struct Fixture {
  store: common::TestStore,
  root: Caller,
  owner: Caller,
  editor: Caller,
  reader: Caller,
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
  for (handle, role) in [
    ("owner", Role::Owner),
    ("editor", Role::Editor),
    ("reader", Role::Reader),
  ] {
    store.set_member(&root, "acme/demo", handle, role).unwrap();
  }
  store
    .put_constitution(&root, "acme/demo", CONSTITUTION, "1.2", "init", None)
    .unwrap();
  store.create_spec(&root, "acme/demo", "first").unwrap();
  store.create_spec(&root, "acme/demo", "second").unwrap();
  store
    .put_document(
      &root,
      "acme/demo",
      1,
      DocumentKind::Spec,
      "# Old\n",
      "old",
      None,
    )
    .unwrap();
  store
    .set_spec_status(&root, "acme/demo", 1, SpecStatus::Archived)
    .unwrap();
  for kind in [DocumentKind::Spec, DocumentKind::Plan, DocumentKind::Tasks] {
    let body = match kind {
      DocumentKind::Tasks => TASKS,
      DocumentKind::Spec => SPEC,
      _ => "# Plan\n",
    };
    store
      .put_document(&root, "acme/demo", 2, kind, body, "write", None)
      .unwrap();
  }
  Fixture {
    owner: people[0].clone(),
    editor: people[1].clone(),
    reader: people[2].clone(),
    root,
    store,
  }
}

fn entries(f: &Fixture, project: &str) -> Vec<AuditEntry> {
  f.store.list_audit(Some(project), 100).unwrap()
}

fn revision_count(f: &Fixture) -> i64 {
  common::int(&f.store, "SELECT count(*) FROM revisions")
}

#[test]
fn an_organization_owner_copies_constitution_specs_and_current_documents() {
  let f = fixture();
  let agent = f.root.clone().with_via(Via::new("Cursor", None));
  let project = f
    .store
    .copy_project(
      &agent,
      "acme/demo",
      "clone",
      "Clone",
      Some("https://git.example/clone"),
    )
    .unwrap();
  assert_eq!(project.address.to_string(), "acme/clone");
  assert_eq!(project.name, "Clone");
  assert_eq!(
    project.repository_url.as_deref(),
    Some("https://git.example/clone")
  );

  let constitution = f.store.get_constitution(&f.root, "acme/clone").unwrap();
  assert_eq!(constitution.body, CONSTITUTION);
  assert_eq!(constitution.version, "1.2");
  assert_eq!(constitution.revision.author, "root");
  assert_eq!(constitution.revision.client.as_deref(), Some("Cursor"));

  let specs = f.store.list_specs(&f.root, "acme/clone").unwrap();
  let shape: Vec<(i64, &str, SpecStatus)> = specs
    .iter()
    .map(|s| (s.number, s.slug.as_str(), s.status))
    .collect();
  assert_eq!(
    shape,
    vec![
      (1, "first", SpecStatus::Archived),
      (2, "second", SpecStatus::Draft)
    ]
  );
  assert_eq!(specs[1].branch, "002-second");
  assert_eq!(
    specs[1].kinds,
    vec![DocumentKind::Spec, DocumentKind::Plan, DocumentKind::Tasks]
  );
  for (kind, expected) in [
    (DocumentKind::Spec, SPEC),
    (DocumentKind::Plan, "# Plan\n"),
    (DocumentKind::Tasks, TASKS),
  ] {
    let document = f
      .store
      .get_document(&f.root, "acme/clone", 2, kind)
      .unwrap();
    assert_eq!(document.body, expected);
    assert_eq!(document.revision.author, "root");
  }
  let old = f
    .store
    .get_document(&f.root, "acme/clone", 1, DocumentKind::Spec)
    .unwrap();
  assert_eq!(old.body, "# Old\n");
  let history = f
    .store
    .list_revisions(&f.root, "acme/clone", Some(2), DocumentKind::Spec)
    .unwrap();
  assert_eq!(history.len(), 1);

  let log = entries(&f, "acme/clone");
  let copied = log.iter().find(|e| e.action == "project.copy").unwrap();
  assert_eq!(copied.result, AuditResult::Ok);
  assert_eq!(copied.actor, "root");
  assert_eq!(copied.client.as_deref(), Some("Cursor"));
  assert!(copied.target.contains("acme/demo"));
  assert!(copied.target.contains("acme/clone"));
}

#[test]
fn copying_a_project_without_a_constitution_copies_the_specs_only() {
  let f = fixture();
  f.store
    .create_project(&f.root, "acme/bare", "Bare")
    .unwrap();
  f.store.create_spec(&f.root, "acme/bare", "only").unwrap();
  f.store
    .copy_project(&f.root, "acme/bare", "bare-copy", "Bare copy", None)
    .unwrap();
  assert!(matches!(
    f.store.get_constitution(&f.root, "acme/bare-copy"),
    Err(StoreError::NotFound(_))
  ));
  let specs = f.store.list_specs(&f.root, "acme/bare-copy").unwrap();
  assert_eq!(specs.len(), 1);
  assert_eq!(specs[0].slug, "only");
}

#[test]
fn only_organization_owners_copy_and_the_denial_is_recorded() {
  let f = fixture();
  let before = revision_count(&f);
  let error = f
    .store
    .copy_project(&f.owner, "acme/demo", "clone", "Clone", None)
    .unwrap_err();
  assert!(matches!(error, StoreError::Forbidden));
  assert_eq!(revision_count(&f), before);
  assert!(matches!(
    f.store.project(&f.root, "acme/clone"),
    Err(StoreError::NotFound(_))
  ));
  let denied = f
    .store
    .list_organization_audit(&f.root, "acme", None, None, 10)
    .unwrap()
    .entries
    .into_iter()
    .find(|e| e.action == "project.copy")
    .unwrap();
  assert_eq!(denied.result, AuditResult::Denied);
  assert_eq!(denied.actor, "owner");
}

#[test]
fn copy_validates_the_slug_the_source_the_organization_and_the_url() {
  let f = fixture();
  let duplicate = f
    .store
    .copy_project(&f.root, "acme/demo", "demo", "Again", None);
  assert!(matches!(duplicate, Err(StoreError::Validation(_))));
  let invalid = f
    .store
    .copy_project(&f.root, "acme/demo", "Bad Slug", "Bad", None);
  assert!(matches!(invalid, Err(StoreError::Validation(_))));
  let url = f
    .store
    .copy_project(&f.root, "acme/demo", "clone", "Clone", Some("ftp://nope"));
  assert!(matches!(url, Err(StoreError::Validation(_))));
  let missing = f
    .store
    .copy_project(&f.root, "acme/ghost", "clone", "Clone", None);
  assert!(matches!(missing, Err(StoreError::NotFound(_))));
  assert!(matches!(
    f.store.project(&f.root, "acme/clone"),
    Err(StoreError::NotFound(_))
  ));
}

#[test]
fn a_source_in_another_organization_is_refused() {
  let f = fixture();
  let other = common::person(&f.store, "other");
  common::organization(&f.store, &other, "elsewhere");
  f.store
    .create_project(&other, "elsewhere/theirs", "Theirs")
    .unwrap();
  let error = f
    .store
    .copy_project(&f.root, "elsewhere/theirs", "stolen", "Stolen", None)
    .unwrap_err();
  assert!(matches!(error, StoreError::NotFound(_)));
  assert!(matches!(
    f.store.project(&f.root, "acme/stolen"),
    Err(StoreError::NotFound(_))
  ));
}

#[test]
fn an_archived_project_can_still_be_copied() {
  let f = fixture();
  f.store.archive_project(&f.owner, "acme/demo").unwrap();
  let copy = f
    .store
    .copy_project(&f.root, "acme/demo", "clone", "Clone", None)
    .unwrap();
  assert_eq!(copy.address.to_string(), "acme/clone");
  let writable = f.store.create_spec(&f.root, "acme/clone", "third");
  assert!(writable.is_ok());
}

#[test]
fn project_owners_archive_and_restore_with_audit_entries() {
  let f = fixture();
  let agent = f.owner.clone().with_via(Via::new("Cursor", None));
  f.store.archive_project(&agent, "acme/demo").unwrap();
  let summary = f.store.project_summaries(&f.root, "acme").unwrap();
  assert!(summary[0].archived);
  f.store.restore_project(&agent, "acme/demo").unwrap();
  let summary = f.store.project_summaries(&f.root, "acme").unwrap();
  assert!(!summary[0].archived);
  let log = entries(&f, "acme/demo");
  let archive = log.iter().find(|e| e.action == "project.archive").unwrap();
  let restore = log.iter().find(|e| e.action == "project.restore").unwrap();
  assert_eq!(archive.result, AuditResult::Ok);
  assert_eq!(archive.actor, "owner");
  assert_eq!(archive.client.as_deref(), Some("Cursor"));
  assert_eq!(restore.result, AuditResult::Ok);
  assert!(restore.id > archive.id);
}

#[test]
fn only_project_owners_archive_and_restore() {
  let f = fixture();
  for caller in [&f.editor, &f.reader] {
    assert!(matches!(
      f.store.archive_project(caller, "acme/demo"),
      Err(StoreError::Forbidden)
    ));
  }
  let outsider = common::person(&f.store, "outsider");
  assert!(matches!(
    f.store.archive_project(&outsider, "acme/demo"),
    Err(StoreError::NotFound(_))
  ));
  f.store.archive_project(&f.owner, "acme/demo").unwrap();
  for caller in [&f.editor, &f.reader] {
    assert!(matches!(
      f.store.restore_project(caller, "acme/demo"),
      Err(StoreError::Forbidden)
    ));
  }
  let operator = common::operator(&f.store, "ops");
  assert!(matches!(
    f.store.restore_project(&operator, "acme/demo"),
    Err(StoreError::Forbidden)
  ));
}

#[test]
fn archiving_twice_or_restoring_an_active_project_is_a_validation_error() {
  let f = fixture();
  assert!(matches!(
    f.store.restore_project(&f.owner, "acme/demo"),
    Err(StoreError::Validation(_))
  ));
  f.store.archive_project(&f.owner, "acme/demo").unwrap();
  assert!(matches!(
    f.store.archive_project(&f.owner, "acme/demo"),
    Err(StoreError::Validation(_))
  ));
}

#[test]
fn every_write_to_an_archived_project_is_forbidden_and_recorded_as_denied() {
  let f = fixture();
  f.store.archive_project(&f.owner, "acme/demo").unwrap();
  let before = revision_count(&f);
  let specs_before = f.store.list_specs(&f.root, "acme/demo").unwrap().len();

  let results: Vec<(&str, Result<(), StoreError>)> = vec![
    (
      "document.put",
      f.store
        .put_document(
          &f.editor,
          "acme/demo",
          2,
          DocumentKind::Plan,
          "# P\n",
          "m",
          None,
        )
        .map(|_| ()),
    ),
    (
      "constitution.put",
      f.store
        .put_constitution(&f.owner, "acme/demo", "# C\n", "9.9", "m", None)
        .map(|_| ()),
    ),
    (
      "spec.create",
      f.store
        .create_spec(&f.owner, "acme/demo", "third")
        .map(|_| ()),
    ),
    (
      "spec.status",
      f.store
        .set_spec_status(&f.owner, "acme/demo", 2, SpecStatus::Clarified)
        .map(|_| ()),
    ),
    (
      "task.done",
      f.store
        .mark_task_done(&f.editor, "acme/demo", 2, "T02", "abc1234")
        .map(|_| ()),
    ),
    (
      "member.set",
      f.store
        .set_member(&f.owner, "acme/demo", "reader", Role::Editor),
    ),
    (
      "member.remove",
      f.store.remove_member(&f.owner, "acme/demo", "reader"),
    ),
    (
      "project.repo",
      f.store
        .set_repository_url(&f.owner, "acme/demo", Some("https://git.example/x")),
    ),
    (
      "project.import",
      f.store
        .import_tree(
          &f.owner,
          "acme/demo",
          &ProjectTree::default(),
          &ImportOptions::default(),
        )
        .map(|_| ()),
    ),
  ];
  for (action, result) in &results {
    assert!(
      matches!(result, Err(StoreError::Forbidden)),
      "{action} should be forbidden, got {result:?}"
    );
  }
  assert_eq!(revision_count(&f), before);
  assert_eq!(
    f.store.list_specs(&f.root, "acme/demo").unwrap().len(),
    specs_before
  );

  let log = entries(&f, "acme/demo");
  for (action, _) in &results {
    let entry = log
      .iter()
      .find(|e| e.action == *action && e.result == AuditResult::Denied)
      .unwrap_or_else(|| panic!("no denied entry for {action}"));
    assert_eq!(entry.project.as_deref(), Some("acme/demo"));
  }
}

#[test]
fn reads_and_audit_listing_keep_working_on_an_archived_project() {
  let f = fixture();
  f.store.archive_project(&f.owner, "acme/demo").unwrap();
  assert_eq!(
    f.store
      .get_constitution(&f.reader, "acme/demo")
      .unwrap()
      .version,
    "1.2"
  );
  assert!(
    f.store
      .get_document(&f.reader, "acme/demo", 2, DocumentKind::Spec)
      .is_ok()
  );
  assert!(f.store.list_tasks(&f.reader, "acme/demo", 2, false).is_ok());
  assert!(f.store.project_status(&f.reader, "acme/demo").is_ok());
  assert!(f.store.list_members(&f.reader, "acme/demo").is_ok());
  assert!(
    f.store
      .list_audit_page(&f.owner, Some("acme/demo"), None, 10)
      .is_ok()
  );
}

#[test]
fn restoring_makes_the_project_writable_again() {
  let f = fixture();
  f.store.archive_project(&f.owner, "acme/demo").unwrap();
  f.store.restore_project(&f.owner, "acme/demo").unwrap();
  let current = f
    .store
    .get_document(&f.editor, "acme/demo", 2, DocumentKind::Plan)
    .unwrap();
  f.store
    .put_document(
      &f.editor,
      "acme/demo",
      2,
      DocumentKind::Plan,
      "# After\n",
      "m",
      Some(current.revision.id),
    )
    .unwrap();
  f.store
    .create_spec(&f.editor, "acme/demo", "third")
    .unwrap();
}
