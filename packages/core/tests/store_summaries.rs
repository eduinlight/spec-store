mod common;

use sdd_vault_core::model::{Caller, DocumentKind, Role, SpecStatus, StoreError, Via};

const TASKS: &str = "# Tasks — Spec 002

## Phase 1 — Setup
- [x] T01 (FR: —) Create the crate — Done when: `cargo build` passes · commit: abc1234
- [ ] T02 (FR-1) Write tests — Done when: tests written and red
- [ ] T03 (FR-1) Implement — Done when: T02 green
";

const SPEC: &str = "# Spec 002

- **FR-1**: THE SYSTEM SHALL work [NEEDS CLARIFICATION: how?]
";

const PLAN: &str = "# Plan\n\n[NEEDS RESEARCH: library] and [NEEDS CLARIFICATION: scope]\n";

struct Fixture {
  store: common::TestStore,
  root: Caller,
  member: Caller,
}

fn fixture() -> Fixture {
  let store = common::test_store();
  let root = common::person(&store, "root");
  common::organization(&store, &root, common::ORG);
  let member = common::person(&store, "member");
  common::join(&store, &root, common::ORG, &member);
  Fixture {
    store,
    root,
    member,
  }
}

fn via(caller: &Caller, client: &str) -> Caller {
  caller.clone().with_via(Via::new(client, None))
}

#[test]
fn an_empty_project_has_no_constitution_spec_or_activity_beyond_its_creation() {
  let f = fixture();
  f.store
    .create_project(&f.root, "acme/empty", "Empty")
    .unwrap();
  let summaries = f.store.project_summaries(&f.root, "acme").unwrap();
  assert_eq!(summaries.len(), 1);
  let summary = &summaries[0];
  assert_eq!(summary.slug, "empty");
  assert_eq!(summary.address.to_string(), "acme/empty");
  assert_eq!(summary.name, "Empty");
  assert_eq!(summary.role, Some(Role::Owner));
  assert!(!summary.archived);
  assert!(summary.constitution.is_none());
  assert!(summary.active_spec.is_none());
  assert_eq!(summary.open_markers, 0);
  let activity = summary
    .last_activity
    .as_ref()
    .expect("creation is activity");
  assert_eq!(activity.actor, "root");
}

#[test]
fn summaries_carry_constitution_active_spec_tasks_markers_and_last_activity() {
  let f = fixture();
  f.store
    .create_project(&f.root, "acme/demo", "Demo")
    .unwrap();
  let writer = via(&f.root, "Cursor");
  f.store
    .put_constitution(&writer, "acme/demo", "# Constitution", "1.2", "init", None)
    .unwrap();
  f.store.create_spec(&writer, "acme/demo", "first").unwrap();
  f.store.create_spec(&writer, "acme/demo", "second").unwrap();
  f.store
    .set_spec_status(&writer, "acme/demo", 1, SpecStatus::Archived)
    .unwrap();
  for (kind, body) in [
    (DocumentKind::Tasks, TASKS),
    (DocumentKind::Spec, SPEC),
    (DocumentKind::Plan, PLAN),
  ] {
    f.store
      .put_document(&writer, "acme/demo", 2, kind, body, "write", None)
      .unwrap();
  }
  let summary = f
    .store
    .project_summaries(&f.root, "acme")
    .unwrap()
    .remove(0);
  let constitution = summary.constitution.expect("constitution");
  assert_eq!(constitution.version, "1.2");
  assert!(constitution.updated_at.ends_with('Z'));
  let active = summary.active_spec.expect("active spec");
  assert_eq!(active.number, 2);
  assert_eq!(active.slug, "second");
  assert_eq!(active.status, SpecStatus::Draft);
  assert_eq!(active.tasks_done, 1);
  assert_eq!(active.tasks_total, 3);
  assert_eq!(summary.open_markers, 3);
  let activity = summary.last_activity.expect("activity");
  assert_eq!(activity.actor, "root");
  assert_eq!(activity.client.as_deref(), Some("Cursor"));
}

#[test]
fn the_active_spec_is_none_when_every_spec_is_archived() {
  let f = fixture();
  f.store
    .create_project(&f.root, "acme/demo", "Demo")
    .unwrap();
  f.store.create_spec(&f.root, "acme/demo", "first").unwrap();
  f.store
    .set_spec_status(&f.root, "acme/demo", 1, SpecStatus::Archived)
    .unwrap();
  let summary = f
    .store
    .project_summaries(&f.root, "acme")
    .unwrap()
    .remove(0);
  assert!(summary.active_spec.is_none());
  assert_eq!(summary.open_markers, 0);
}

#[test]
fn a_spec_without_a_tasks_document_counts_zero_tasks() {
  let f = fixture();
  f.store
    .create_project(&f.root, "acme/demo", "Demo")
    .unwrap();
  f.store.create_spec(&f.root, "acme/demo", "first").unwrap();
  let active = f
    .store
    .project_summaries(&f.root, "acme")
    .unwrap()
    .remove(0)
    .active_spec
    .unwrap();
  assert_eq!((active.tasks_done, active.tasks_total), (0, 0));
}

#[test]
fn last_activity_ignores_refused_attempts() {
  let f = fixture();
  f.store
    .create_project(&f.root, "acme/demo", "Demo")
    .unwrap();
  f.store
    .set_member(&f.root, "acme/demo", "member", Role::Reader)
    .unwrap();
  let refused = f
    .store
    .create_spec(&via(&f.member, "Intruder"), "acme/demo", "nope");
  assert!(matches!(refused, Err(StoreError::Forbidden)));
  let summary = f
    .store
    .project_summaries(&f.root, "acme")
    .unwrap()
    .remove(0);
  let activity = summary.last_activity.unwrap();
  assert_eq!(activity.actor, "root");
  assert_ne!(activity.client.as_deref(), Some("Intruder"));
}

#[test]
fn members_see_their_projects_with_their_role_and_owners_see_all() {
  let f = fixture();
  f.store
    .create_project(&f.root, "acme/alpha", "Alpha")
    .unwrap();
  f.store
    .create_project(&f.root, "acme/beta", "Beta")
    .unwrap();
  f.store
    .set_member(&f.root, "acme/beta", "member", Role::Editor)
    .unwrap();
  let seen = f.store.project_summaries(&f.member, "acme").unwrap();
  assert_eq!(seen.len(), 1);
  assert_eq!(seen[0].slug, "beta");
  assert_eq!(seen[0].role, Some(Role::Editor));
  let all = f.store.project_summaries(&f.root, "acme").unwrap();
  let slugs: Vec<&str> = all.iter().map(|s| s.slug.as_str()).collect();
  assert_eq!(slugs, vec!["alpha", "beta"]);
  assert!(all.iter().all(|s| s.role == Some(Role::Owner)));
}

#[test]
fn an_operator_sees_every_project_without_a_role() {
  let f = fixture();
  f.store
    .create_project(&f.root, "acme/alpha", "Alpha")
    .unwrap();
  let operator = common::operator(&f.store, "ops");
  let seen = f.store.project_summaries(&operator, "acme").unwrap();
  assert_eq!(seen.len(), 1);
  assert_eq!(seen[0].role, None);
}

#[test]
fn outsiders_and_unknown_organizations_are_not_found() {
  let f = fixture();
  let outsider = common::person(&f.store, "outsider");
  assert!(matches!(
    f.store.project_summaries(&outsider, "acme"),
    Err(StoreError::NotFound(_))
  ));
  assert!(matches!(
    f.store.project_summaries(&f.root, "ghost"),
    Err(StoreError::NotFound(_))
  ));
}

#[test]
fn summaries_of_other_organizations_are_not_mixed_in() {
  let f = fixture();
  common::organization(&f.store, &f.root, "other");
  f.store
    .create_project(&f.root, "acme/alpha", "Alpha")
    .unwrap();
  f.store
    .create_project(&f.root, "other/gamma", "Gamma")
    .unwrap();
  let seen = f.store.project_summaries(&f.root, "other").unwrap();
  assert_eq!(seen.len(), 1);
  assert_eq!(seen[0].address.to_string(), "other/gamma");
}
