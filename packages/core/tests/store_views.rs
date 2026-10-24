mod common;

use sdd_vault_core::markdown::requirements::parse_requirements;
use sdd_vault_core::markdown::tasks::parse_tasks;
use sdd_vault_core::model::{Caller, DocumentKind, Role, SpecStatus, StoreError, Via};
use sdd_vault_core::store::views::readiness::ReadinessKey;
use sdd_vault_core::store::views::traceability::Coverage;

const SPEC_V1: &str = "# Spec 001 — Billing views

Status: draft

## 1. Context and goal

Show invoices as cards.
More detail here.

## 5. Functional requirements

- **FR-1** (US-1): THE SYSTEM SHALL list invoices. [NEEDS CLARIFICATION: which currency?]
- **FR-2** (US-1): THE SYSTEM SHALL show totals.
- **FR-3** (US-1): THE SYSTEM SHALL export.
- **NFR-1**: Fast.
";

const SPEC_V2: &str = "# Spec 001 — Billing views

## 1. Context and goal

Show invoices as cards.

## 5. Functional requirements

- **FR-1** (US-1): THE SYSTEM SHALL list invoices in euros.
- **FR-2** (US-1): THE SYSTEM SHALL show totals.
- **FR-3** (US-1): THE SYSTEM SHALL export.
- **NFR-1**: Fast.
";

const PLAN: &str = "# Plan\n\nResearch: [NEEDS RESEARCH: pdf library]\n";

const TASKS_V1: &str = "# Tasks

## Phase 1
- [ ] T01 (FR-1) First — Done when: green
- [ ] T02 [P] (FR-1, FR-2) Second — Done when: green
- [ ] T03 (FR: —) Housekeeping — Done when: clean
";

const TASKS_V2: &str = "# Tasks

## Phase 1
- [x] T01 (FR-1) First — Done when: green · commit: abc1234
- [ ] T02 [P] (FR-1, FR-2) Second — Done when: green
- [ ] T03 (FR: —) Housekeeping — Done when: clean
";

const TASKS_V3: &str = "# Tasks

## Phase 1
- [x] T01 (FR-1) First — Done when: green · commit: abc1234
- [x] T02 [P] (FR-1, FR-2) Second — Done when: green · commit: def5678
- [ ] T03 (FR: —) Housekeeping — Done when: clean
";

struct Fixture {
  store: common::TestStore,
  root: Caller,
  member: Caller,
  outsider: Caller,
}

fn fixture() -> Fixture {
  let store = common::test_store();
  let root = common::person(&store, "root");
  common::organization(&store, &root, common::ORG);
  let member = common::person(&store, "member");
  common::join(&store, &root, common::ORG, &member);
  let outsider = common::person(&store, "outsider");
  store.create_project(&root, "acme/demo", "Demo").unwrap();
  store
    .set_member(&root, "acme/demo", "member", Role::Editor)
    .unwrap();
  Fixture {
    store,
    root,
    member,
    outsider,
  }
}

fn via(caller: &Caller, client: &str) -> Caller {
  caller.clone().with_via(Via::new(client, None))
}

fn put(
  f: &Fixture,
  caller: &Caller,
  number: i64,
  kind: DocumentKind,
  body: &str,
  expected: Option<i64>,
) -> i64 {
  f.store
    .put_document(caller, "acme/demo", number, kind, body, "edit", expected)
    .unwrap()
}

fn seed_history(f: &Fixture) {
  let cursor = via(&f.root, "Cursor");
  let claude = via(&f.root, "Claude Code");
  f.store
    .create_spec(&cursor, "acme/demo", "billing")
    .unwrap();
  let spec = put(f, &cursor, 1, DocumentKind::Spec, SPEC_V1, None);
  put(f, &cursor, 1, DocumentKind::Plan, PLAN, None);
  let tasks = put(f, &cursor, 1, DocumentKind::Tasks, TASKS_V1, None);
  let tasks = put(f, &claude, 1, DocumentKind::Tasks, TASKS_V2, Some(tasks));
  put(f, &f.member, 1, DocumentKind::Tasks, TASKS_V3, Some(tasks));
  put(f, &claude, 1, DocumentKind::Spec, SPEC_V2, Some(spec));
}

#[test]
fn cards_summarize_title_documents_counts_and_contributors() {
  let f = fixture();
  seed_history(&f);
  f.store.create_spec(&f.root, "acme/demo", "empty").unwrap();
  let cards = f.store.spec_cards(&f.member, "acme/demo").unwrap();
  assert_eq!(cards.len(), 2);
  assert_eq!(cards[0].number, 2);
  assert_eq!(cards[0].title, "empty");
  assert_eq!(cards[0].summary, "");
  assert!(cards[0].documents.is_empty());
  assert_eq!(cards[0].requirement_count, 0);
  assert!(cards[0].contributors.is_empty());
  assert_eq!(cards[0].updated_at, cards[0].created_at);

  let card = &cards[1];
  assert_eq!(card.number, 1);
  assert_eq!(card.slug, "billing");
  assert_eq!(card.title, "Billing views");
  assert_eq!(card.summary, "Show invoices as cards.");
  assert_eq!(card.status, SpecStatus::Draft);
  assert_eq!(card.branch, "001-billing");
  assert_eq!(
    card.documents,
    [DocumentKind::Spec, DocumentKind::Plan, DocumentKind::Tasks]
  );
  assert_eq!(card.requirement_count, 3);
  assert_eq!((card.tasks_done, card.tasks_total), (2, 3));
  assert_eq!(card.open_markers, 1);
  assert_eq!(card.contributors, ["root", "member"]);
  assert!(card.updated_at >= card.created_at);
  let latest = f
    .store
    .list_revisions(&f.member, "acme/demo", Some(1), DocumentKind::Spec)
    .unwrap();
  assert_eq!(card.updated_at, latest[0].created_at);
}

#[test]
fn outsiders_get_not_found_from_every_view() {
  let f = fixture();
  seed_history(&f);
  let store = &f.store;
  let denied = |result: Result<(), StoreError>| {
    assert!(matches!(result, Err(StoreError::NotFound(_))), "{result:?}")
  };
  denied(store.spec_cards(&f.outsider, "acme/demo").map(|_| ()));
  denied(
    store
      .spec_traceability(&f.outsider, "acme/demo", 1)
      .map(|_| ()),
  );
  denied(
    store
      .spec_readiness(&f.outsider, "acme/demo", 1)
      .map(|_| ()),
  );
  denied(store.spec_markers(&f.outsider, "acme/demo", 1).map(|_| ()));
  denied(store.spec_tasks(&f.outsider, "acme/demo", 1).map(|_| ()));
  denied(
    store
      .spec_traceability(&f.member, "acme/demo", 9)
      .map(|_| ()),
  );
}

#[test]
fn traceability_joins_requirements_tasks_and_commits() {
  let f = fixture();
  seed_history(&f);
  let trace = f
    .store
    .spec_traceability(&f.member, "acme/demo", 1)
    .unwrap();
  let rows: Vec<(&str, Coverage, usize)> = trace
    .requirements
    .iter()
    .map(|r| (r.id.as_str(), r.status, r.tasks.len()))
    .collect();
  assert_eq!(
    rows,
    [
      ("FR-1", Coverage::Covered, 2),
      ("FR-2", Coverage::Covered, 1),
      ("FR-3", Coverage::NoTasks, 0),
    ]
  );
  assert_eq!(
    trace.requirements[0].text,
    "THE SYSTEM SHALL list invoices in euros."
  );
  assert_eq!(
    trace.requirements[0].tasks[0].commit.as_deref(),
    Some("abc1234")
  );
  assert_eq!(
    trace.requirements[0].tasks[1].commit.as_deref(),
    Some("def5678")
  );
  assert_eq!(trace.tasks_without_requirements.len(), 1);
  assert_eq!(trace.tasks_without_requirements[0].id, "T03");
}

#[test]
fn readiness_reports_each_item() {
  let f = fixture();
  seed_history(&f);
  let readiness = f.store.spec_readiness(&f.member, "acme/demo", 1).unwrap();
  assert!(!readiness.ready);
  let by_key = |key: ReadinessKey| readiness.items.iter().find(|i| i.key == key).unwrap();
  let markers = by_key(ReadinessKey::OpenMarkersResolved);
  assert!(!markers.ok);
  assert_eq!(markers.count, Some(1));
  let covered = by_key(ReadinessKey::RequirementsCovered);
  assert!(!covered.ok);
  assert_eq!((covered.count, covered.total), (Some(2), Some(3)));
  assert!(by_key(ReadinessKey::PlanWritten).ok);
  let tasks = by_key(ReadinessKey::TasksWritten);
  assert!(tasks.ok);
  assert_eq!(tasks.count, Some(3));
  assert!(!by_key(ReadinessKey::ConstitutionExists).ok);

  f.store
    .put_constitution(&f.root, "acme/demo", "# Constitution", "1.0", "init", None)
    .unwrap();
  let readiness = f.store.spec_readiness(&f.member, "acme/demo", 1).unwrap();
  assert!(
    readiness
      .items
      .iter()
      .find(|i| i.key == ReadinessKey::ConstitutionExists)
      .unwrap()
      .ok
  );
}

#[test]
fn a_spec_with_nothing_written_is_not_ready() {
  let f = fixture();
  f.store.create_spec(&f.root, "acme/demo", "bare").unwrap();
  let readiness = f.store.spec_readiness(&f.root, "acme/demo", 1).unwrap();
  assert!(!readiness.ready);
  assert!(readiness.items[0].ok);
  assert!(readiness.items[1..].iter().all(|i| !i.ok));
  assert!(
    f.store
      .spec_traceability(&f.root, "acme/demo", 1)
      .unwrap()
      .requirements
      .is_empty()
  );
  assert!(matches!(
    f.store.spec_tasks(&f.root, "acme/demo", 1),
    Err(StoreError::NotFound(_))
  ));
  let markers = f.store.spec_markers(&f.root, "acme/demo", 1).unwrap();
  assert!(markers.open.is_empty() && markers.resolved.is_empty());
}

#[test]
fn tasks_carry_the_attribution_of_the_revision_that_completed_them() {
  let f = fixture();
  seed_history(&f);
  let tasks = f.store.spec_tasks(&f.member, "acme/demo", 1).unwrap();
  assert_eq!(tasks.parsed.tasks.len(), 3);
  let t01 = &tasks.completions["T01"];
  assert_eq!(t01.author, "root");
  assert_eq!(t01.client.as_deref(), Some("Claude Code"));
  let t02 = &tasks.completions["T02"];
  assert_eq!(t02.author, "member");
  assert_eq!(t02.client, None);
  assert!(t02.at >= t01.at);
  assert!(!tasks.completions.contains_key("T03"));
}

#[test]
fn markers_split_into_open_and_resolved_with_the_resolving_revision() {
  let f = fixture();
  seed_history(&f);
  let markers = f.store.spec_markers(&f.member, "acme/demo", 1).unwrap();
  assert_eq!(markers.open.len(), 1);
  assert_eq!(markers.open[0].document_kind, DocumentKind::Plan);
  assert_eq!(markers.open[0].text, "pdf library");
  assert_eq!(markers.resolved.len(), 1);
  let resolved = &markers.resolved[0];
  assert_eq!(resolved.document_kind, DocumentKind::Spec);
  assert_eq!(resolved.text, "which currency?");
  assert_eq!(resolved.heading, "5. Functional requirements");
  assert_eq!(resolved.resolved_by, "root");
  assert_eq!(resolved.client.as_deref(), Some("Claude Code"));
  let revisions = f
    .store
    .list_revisions(&f.member, "acme/demo", Some(1), DocumentKind::Spec)
    .unwrap();
  assert_eq!(resolved.revision_id, revisions[0].id);
  assert_eq!(resolved.at, revisions[0].created_at);
}

#[test]
fn the_parsers_read_every_spec_and_tasks_document_of_this_repository() {
  let specs = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../specs");
  let mut checked = 0;
  for entry in std::fs::read_dir(specs).unwrap().flatten() {
    let Ok(spec) = std::fs::read_to_string(entry.path().join("spec.md")) else {
      continue;
    };
    let requirements = parse_requirements(&spec);
    let declared = spec.matches("- **FR-").count() + spec.matches("- **NFR-").count();
    assert_eq!(requirements.len(), declared, "{:?}", entry.path());
    assert!(
      requirements.iter().all(|r| !r.text.is_empty()),
      "{:?}",
      entry.path()
    );
    if let Ok(tasks) = std::fs::read_to_string(entry.path().join("tasks.md")) {
      assert!(!parse_tasks(&tasks).tasks.is_empty(), "{:?}", entry.path());
    }
    checked += 1;
  }
  assert!(checked >= 5);
}
