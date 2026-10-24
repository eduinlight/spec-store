mod common;

use sdd_vault_core::markdown::markers::MarkerKind;
use sdd_vault_core::model::{AuditResult, Caller, DocumentKind, Role, SpecStatus, StoreError, Via};
use sdd_vault_core::store::views::activity::ActivityFilter;
use sdd_vault_core::store::views::overview::AttentionKind;

const SPEC_1: &str = "# Spec 001 — Billing views

## 1. Context and goal

Show invoices as cards.

## 5. Functional requirements

- **FR-1** (US-1): THE SYSTEM SHALL list invoices.
- **FR-2** (US-1): THE SYSTEM SHALL show totals.
- **FR-3** (US-1): THE SYSTEM SHALL export. [NEEDS CLARIFICATION: which format?]
";

const TASKS_1: &str = "# Tasks

## Phase 1 — Setup
- [ ] T01 (FR-1) First — Done when: green
- [ ] T02 [P] (FR-1, FR-2) Second — Done when: shown
- [ ] T03 (FR: —) Housekeeping — Done when: clean

## Phase 2 — Polish
- [ ] T04 (FR-2) Fourth — Done when: polished
";

const SPEC_2: &str = "# Spec 002 — Search

## 1. Context and goal

Find invoices quickly.

## 5. Functional requirements

- **FR-1** (US-1): THE SYSTEM SHALL search. [NEEDS CLARIFICATION: fuzzy or exact?]
- **FR-2** (US-1): THE SYSTEM SHALL rank. [NEEDS RESEARCH: ranking library]
";

struct Fixture {
  store: common::TestStore,
  root: Caller,
  editor: Caller,
  reader: Caller,
  outsider: Caller,
}

fn fixture() -> Fixture {
  let store = common::test_store();
  let root = common::person(&store, "root");
  common::organization(&store, &root, common::ORG);
  let editor = common::person(&store, "editor");
  let reader = common::person(&store, "reader");
  common::join(&store, &root, common::ORG, &editor);
  common::join(&store, &root, common::ORG, &reader);
  store.create_project(&root, "acme/demo", "Demo").unwrap();
  store
    .set_member(&root, "acme/demo", "editor", Role::Editor)
    .unwrap();
  store
    .set_member(&root, "acme/demo", "reader", Role::Reader)
    .unwrap();
  Fixture {
    outsider: common::person(&store, "outsider"),
    store,
    root,
    editor,
    reader,
  }
}

fn via(caller: &Caller, client: &str) -> Caller {
  caller.clone().with_via(Via::new(client, None))
}

fn seed_first_spec(f: &Fixture) {
  let cursor = via(&f.root, "Cursor");
  f.store
    .create_spec(&cursor, "acme/demo", "billing")
    .unwrap();
  f.store
    .put_document(
      &cursor,
      "acme/demo",
      1,
      DocumentKind::Spec,
      SPEC_1,
      "edit",
      None,
    )
    .unwrap();
  f.store
    .put_document(
      &cursor,
      "acme/demo",
      1,
      DocumentKind::Tasks,
      TASKS_1,
      "edit",
      None,
    )
    .unwrap();
}

fn done(f: &Fixture, client: &str, task: &str) -> i64 {
  f.store
    .mark_task_done(&via(&f.root, client), "acme/demo", 1, task, "abc1234")
    .unwrap();
  last_audit_id(f)
}

fn last_audit_id(f: &Fixture) -> i64 {
  common::int(&f.store, "SELECT max(id) FROM audit_log")
}

fn backdate(f: &Fixture, audit_id: i64, interval: &str) {
  common::exec(
    &f.store,
    &format!(
      "UPDATE audit_log SET created_at = now() - interval '{interval}' WHERE id = {audit_id}"
    ),
  );
}

#[test]
fn overview_counts_completed_tasks_against_the_previous_period() {
  let f = fixture();
  seed_first_spec(&f);
  done(&f, "Claude Code", "T01");
  let two_days = done(&f, "Cursor", "T02");
  backdate(&f, two_days, "2 days");
  let ten_days = done(&f, "Cursor", "T03");
  backdate(&f, ten_days, "10 days");

  let week = f
    .store
    .project_dashboard(&f.reader, "acme/demo", 7)
    .unwrap();
  assert_eq!(week.days, 7);
  assert_eq!(week.tasks_completed, 2);
  assert_eq!(week.tasks_completed_previous, 1);

  let fortnight = f
    .store
    .project_dashboard(&f.reader, "acme/demo", 14)
    .unwrap();
  assert_eq!(fortnight.tasks_completed, 3);
  assert_eq!(fortnight.tasks_completed_previous, 0);
}

#[test]
fn overview_lists_every_day_of_the_period_with_completions_by_client() {
  let f = fixture();
  seed_first_spec(&f);
  done(&f, "Claude Code", "T01");
  done(&f, "Claude Code", "T02");
  done(&f, "Cursor", "T03");
  let old = done(&f, "Cursor", "T04");
  backdate(&f, old, "3 days");

  let overview = f
    .store
    .project_dashboard(&f.reader, "acme/demo", 7)
    .unwrap();
  assert_eq!(overview.completed_per_day.len(), 7);
  let days: Vec<&str> = overview
    .completed_per_day
    .iter()
    .map(|d| d.day.as_str())
    .collect();
  let mut sorted = days.clone();
  sorted.sort();
  assert_eq!(days, sorted);
  let today = overview.completed_per_day.last().unwrap();
  let counts: Vec<(&str, i64)> = today
    .by_client
    .iter()
    .map(|c| (c.client.as_str(), c.count))
    .collect();
  assert_eq!(counts, [("Claude Code", 2), ("Cursor", 1)]);
  let busy: Vec<_> = overview
    .completed_per_day
    .iter()
    .filter(|d| !d.by_client.is_empty())
    .collect();
  assert_eq!(busy.len(), 2);
  assert_eq!(overview.completed_per_day[3].by_client[0].client, "Cursor");
  assert!(overview.completed_per_day[0].by_client.is_empty());
}

#[test]
fn overview_only_accepts_the_three_periods_and_hides_the_project_from_outsiders() {
  let f = fixture();
  for days in [0, 1, 8, 31, -7] {
    assert!(matches!(
      f.store.project_dashboard(&f.reader, "acme/demo", days),
      Err(StoreError::Validation(_))
    ));
  }
  for days in [7, 14, 30] {
    assert!(
      f.store
        .project_dashboard(&f.reader, "acme/demo", days)
        .is_ok()
    );
  }
  assert!(matches!(
    f.store.project_dashboard(&f.outsider, "acme/demo", 7),
    Err(StoreError::NotFound(_))
  ));
  assert!(matches!(
    f.store.active_specs(&f.outsider, "acme/demo"),
    Err(StoreError::NotFound(_))
  ));
  assert!(matches!(
    f.store
      .project_activity(&f.outsider, "acme/demo", ActivityFilter::All, None, 50),
    Err(StoreError::NotFound(_))
  ));
}

#[test]
fn agents_active_counts_distinct_mcp_clients_with_writes_in_the_last_fifteen_minutes() {
  let f = fixture();
  seed_first_spec(&f);
  assert_eq!(
    f.store
      .project_dashboard(&f.reader, "acme/demo", 7)
      .unwrap()
      .agents_active,
    1
  );
  let cursor_rows = common::int(
    &f.store,
    "SELECT count(*) FROM audit_log WHERE client = 'Cursor'",
  );
  assert!(cursor_rows > 0);
  common::exec(
    &f.store,
    "UPDATE audit_log SET created_at = now() - interval '20 minutes' WHERE client = 'Cursor'",
  );
  assert_eq!(
    f.store
      .project_dashboard(&f.reader, "acme/demo", 7)
      .unwrap()
      .agents_active,
    0
  );
  done(&f, "Claude Code", "T01");
  done(&f, "Claude Code", "T02");
  done(&f, "Web app", "T03");
  done(&f, "sddvault CLI", "T04");
  assert_eq!(
    f.store
      .project_dashboard(&f.reader, "acme/demo", 7)
      .unwrap()
      .agents_active,
    1
  );
  f.store
    .create_spec(&via(&f.root, "Cursor"), "acme/demo", "search")
    .unwrap();
  assert_eq!(
    f.store
      .project_dashboard(&f.reader, "acme/demo", 7)
      .unwrap()
      .agents_active,
    2
  );
}

#[test]
fn overview_reports_coverage_markers_and_active_specs() {
  let f = fixture();
  seed_first_spec(&f);
  let empty = fixture();
  let nothing = empty
    .store
    .project_dashboard(&empty.reader, "acme/demo", 7)
    .unwrap();
  assert_eq!(nothing.active_specs, 0);
  assert!(nothing.coverage.is_none());
  assert_eq!(nothing.open_markers, 0);
  assert!(nothing.attention.is_empty());

  let overview = f
    .store
    .project_dashboard(&f.reader, "acme/demo", 7)
    .unwrap();
  assert_eq!(overview.active_specs, 1);
  assert_eq!(overview.open_markers, 1);
  let coverage = overview.coverage.unwrap();
  assert_eq!(
    (coverage.spec_number, coverage.covered, coverage.total),
    (1, 0, 3)
  );

  done(&f, "Claude Code", "T01");
  done(&f, "Claude Code", "T02");
  done(&f, "Claude Code", "T04");
  f.store.create_spec(&f.root, "acme/demo", "search").unwrap();
  f.store
    .put_document(
      &f.root,
      "acme/demo",
      2,
      DocumentKind::Spec,
      SPEC_2,
      "edit",
      None,
    )
    .unwrap();
  let overview = f
    .store
    .project_dashboard(&f.reader, "acme/demo", 7)
    .unwrap();
  assert_eq!(overview.active_specs, 2);
  assert_eq!(overview.open_markers, 3);
  let coverage = overview.coverage.unwrap();
  assert_eq!(
    (coverage.spec_number, coverage.covered, coverage.total),
    (2, 0, 2)
  );
}

#[test]
fn a_finished_approved_spec_is_no_longer_active() {
  let f = fixture();
  seed_first_spec(&f);
  for task in ["T01", "T02", "T03", "T04"] {
    done(&f, "Claude Code", task);
  }
  f.store
    .set_spec_status(&f.root, "acme/demo", 1, SpecStatus::Clarified)
    .unwrap();
  f.store
    .set_spec_status(&f.root, "acme/demo", 1, SpecStatus::Approved)
    .unwrap();
  assert_eq!(
    f.store
      .project_dashboard(&f.reader, "acme/demo", 7)
      .unwrap()
      .active_specs,
    0
  );
  f.store.create_spec(&f.root, "acme/demo", "search").unwrap();
  f.store
    .set_spec_status(&f.root, "acme/demo", 2, SpecStatus::Archived)
    .unwrap();
  assert_eq!(
    f.store
      .project_dashboard(&f.reader, "acme/demo", 7)
      .unwrap()
      .active_specs,
    0
  );
}

#[test]
fn attention_lists_approvals_for_owners_questions_and_requirements_without_work() {
  let f = fixture();
  seed_first_spec(&f);
  f.store
    .set_spec_status(&f.editor, "acme/demo", 1, SpecStatus::Clarified)
    .unwrap();
  f.store.create_spec(&f.root, "acme/demo", "search").unwrap();
  f.store
    .put_document(
      &f.root,
      "acme/demo",
      2,
      DocumentKind::Spec,
      SPEC_2,
      "edit",
      None,
    )
    .unwrap();

  let owner = f.store.project_dashboard(&f.root, "acme/demo", 7).unwrap();
  let items: Vec<(AttentionKind, i64, usize)> = owner
    .attention
    .iter()
    .map(|a| (a.kind, a.spec_number, a.count))
    .collect();
  assert_eq!(
    items,
    [
      (AttentionKind::ApprovalWaiting, 1, 1),
      (AttentionKind::OpenQuestions, 1, 1),
      (AttentionKind::OpenQuestions, 2, 1),
      (AttentionKind::RequirementWithoutWork, 2, 2),
    ]
  );
  assert_eq!(owner.attention[3].detail, ["FR-1", "FR-2"]);

  let reader = f
    .store
    .project_dashboard(&f.reader, "acme/demo", 7)
    .unwrap();
  assert!(
    reader
      .attention
      .iter()
      .all(|a| a.kind != AttentionKind::ApprovalWaiting)
  );
  assert_eq!(reader.attention.len(), 3);
  assert_eq!(AttentionKind::ApprovalWaiting.as_str(), "approval_waiting");
  assert_eq!(AttentionKind::OpenQuestions.as_str(), "open_questions");
  assert_eq!(
    AttentionKind::RequirementWithoutWork.as_str(),
    "requirement_without_work"
  );
}

#[test]
fn active_specs_show_the_next_task_the_phase_and_the_agent_prompt() {
  let f = fixture();
  assert!(
    f.store
      .active_specs(&f.reader, "acme/demo")
      .unwrap()
      .active
      .is_none()
  );
  seed_first_spec(&f);
  done(&f, "Claude Code", "T01");
  let view = f.store.active_specs(&f.reader, "acme/demo").unwrap();
  let active = view.active.unwrap();
  assert_eq!((active.number, active.slug.as_str()), (1, "billing"));
  assert_eq!(active.title, "Billing views");
  assert_eq!(active.status, SpecStatus::Draft);
  assert_eq!(active.branch, "001-billing");
  assert_eq!((active.tasks_done, active.tasks_total), (1, 4));
  assert_eq!(active.current_phase.as_deref(), Some("Phase 1 — Setup"));
  assert_eq!(active.open_markers, 1);
  let next = active.next_task.unwrap();
  assert_eq!(next.id, "T02");
  assert!(next.parallel);
  assert_eq!(next.frs, ["FR-1", "FR-2"]);
  assert_eq!(next.description, "Second");
  assert_eq!(next.done_when, "shown");
  assert_eq!(
    active.agent_prompt.as_deref(),
    Some(
      "Work on SDD Vault project acme/demo, spec 001-billing, task T02: Second. Done when: shown. Read the spec, plan and task through the sdd-vault MCP server first, follow the constitution, write tests first, and mark the task done with its commit."
    )
  );

  for task in ["T02", "T03", "T04"] {
    done(&f, "Claude Code", task);
  }
  let finished = f
    .store
    .active_specs(&f.reader, "acme/demo")
    .unwrap()
    .active
    .unwrap();
  assert!(finished.next_task.is_none());
  assert!(finished.agent_prompt.is_none());
  assert!(finished.current_phase.is_none());
}

#[test]
fn drafts_carry_their_summary_and_open_markers() {
  let f = fixture();
  seed_first_spec(&f);
  f.store
    .set_spec_status(&f.root, "acme/demo", 1, SpecStatus::Clarified)
    .unwrap();
  f.store.create_spec(&f.root, "acme/demo", "search").unwrap();
  f.store
    .put_document(
      &f.root,
      "acme/demo",
      2,
      DocumentKind::Spec,
      SPEC_2,
      "edit",
      None,
    )
    .unwrap();
  f.store.create_spec(&f.root, "acme/demo", "bare").unwrap();
  let view = f.store.active_specs(&f.reader, "acme/demo").unwrap();
  assert_eq!(view.active.as_ref().unwrap().number, 3);
  let numbers: Vec<i64> = view.drafts.iter().map(|d| d.number).collect();
  assert_eq!(numbers, [2]);
  let draft = &view.drafts[0];
  assert_eq!(draft.slug, "search");
  assert_eq!(draft.title, "Search");
  assert_eq!(draft.summary, "Find invoices quickly.");
  let markers: Vec<(MarkerKind, &str)> = draft
    .markers
    .iter()
    .map(|m| (m.kind, m.text.as_str()))
    .collect();
  assert_eq!(
    markers,
    [
      (MarkerKind::Clarification, "fuzzy or exact?"),
      (MarkerKind::Research, "ranking library"),
    ]
  );
}

fn seed_activity(f: &Fixture) {
  seed_first_spec(f);
  done(f, "Claude Code", "T01");
  let _ = f.store.put_document(
    &f.reader,
    "acme/demo",
    1,
    DocumentKind::Plan,
    "# Plan",
    "edit",
    None,
  );
  f.store
    .set_member(&f.root, "acme/demo", "reader", Role::Editor)
    .unwrap();
}

#[test]
fn activity_lists_newest_first_with_attribution_and_summary_keys() {
  let f = fixture();
  seed_activity(&f);
  let page = f
    .store
    .project_activity(&f.reader, "acme/demo", ActivityFilter::All, None, 50)
    .unwrap();
  assert!(!page.has_more);
  let ids: Vec<i64> = page.rows.iter().map(|r| r.audit_id).collect();
  let mut sorted = ids.clone();
  sorted.sort_by(|a, b| b.cmp(a));
  assert_eq!(ids, sorted);
  let newest = &page.rows[0];
  assert_eq!(newest.action, "member.set");
  assert_eq!(newest.actor, "root");
  assert_eq!(newest.result, AuditResult::Ok);
  assert_eq!(newest.summary_key, "member.set");
  assert!(newest.at.ends_with('Z'));
  let denied = &page.rows[1];
  assert_eq!(denied.action, "document.put");
  assert_eq!(denied.result, AuditResult::Denied);
  assert_eq!(denied.actor, "reader");
  assert_eq!(denied.summary_key, "denied");
  let task = page.rows.iter().find(|r| r.action == "task.done").unwrap();
  assert_eq!(task.client.as_deref(), Some("Claude Code"));
  assert_eq!(task.target, "001/T01@abc1234");
  assert_eq!(task.summary_key, "task.done");
}

#[test]
fn activity_filters_select_tasks_documents_and_security() {
  let f = fixture();
  seed_activity(&f);
  let actions = |filter| -> Vec<(String, AuditResult)> {
    f.store
      .project_activity(&f.reader, "acme/demo", filter, None, 50)
      .unwrap()
      .rows
      .into_iter()
      .map(|r| (r.action, r.result))
      .collect()
  };
  assert_eq!(
    actions(ActivityFilter::Tasks),
    [("task.done".to_string(), AuditResult::Ok)]
  );
  let documents = actions(ActivityFilter::Documents);
  assert!(documents.iter().all(|(a, _)| matches!(
    a.as_str(),
    "document.put"
      | "constitution.put"
      | "spec.create"
      | "spec.status"
      | "project.import"
      | "project.copy"
      | "spec.import"
  )));
  assert_eq!(
    documents
      .iter()
      .filter(|(a, _)| a == "document.put")
      .count(),
    3
  );
  let security = actions(ActivityFilter::Security);
  assert!(security.contains(&("member.set".to_string(), AuditResult::Ok)));
  assert!(security.contains(&("document.put".to_string(), AuditResult::Denied)));
  assert!(security.contains(&("project.create".to_string(), AuditResult::Ok)));
  assert!(!security.iter().any(|(a, _)| a == "task.done"));
  assert_eq!(ActivityFilter::Tasks.as_str(), "tasks");
  assert_eq!(
    "security".parse::<ActivityFilter>().unwrap(),
    ActivityFilter::Security
  );
  assert!("everything".parse::<ActivityFilter>().is_err());
}

#[test]
fn activity_pages_backwards_with_the_before_cursor() {
  let f = fixture();
  seed_activity(&f);
  let first = f
    .store
    .project_activity(&f.reader, "acme/demo", ActivityFilter::All, None, 3)
    .unwrap();
  assert_eq!(first.rows.len(), 3);
  assert!(first.has_more);
  let before = first.rows.last().unwrap().audit_id;
  let second = f
    .store
    .project_activity(&f.reader, "acme/demo", ActivityFilter::All, Some(before), 3)
    .unwrap();
  assert!(second.rows.iter().all(|r| r.audit_id < before));
  assert_eq!(second.rows.len(), 3);
}

#[test]
fn a_conflict_followed_by_a_saved_write_of_the_same_document_is_resolved() {
  let f = fixture();
  seed_first_spec(&f);
  let claude = via(&f.root, "Claude Code");
  let stale = f
    .store
    .put_document(
      &claude,
      "acme/demo",
      1,
      DocumentKind::Plan,
      "# Plan",
      "edit",
      None,
    )
    .unwrap();
  let other = f
    .store
    .put_document(
      &f.editor,
      "acme/demo",
      1,
      DocumentKind::Plan,
      "# Plan 2",
      "edit",
      Some(stale),
    )
    .unwrap();
  assert!(other > stale);
  let conflict = f.store.put_document(
    &claude,
    "acme/demo",
    1,
    DocumentKind::Plan,
    "# Plan 3",
    "edit",
    Some(stale),
  );
  assert!(matches!(conflict, Err(StoreError::Conflict { .. })));
  let conflict_id = last_audit_id(&f);
  let unresolved = f
    .store
    .project_activity(&f.reader, "acme/demo", ActivityFilter::All, None, 50)
    .unwrap();
  let row = unresolved
    .rows
    .iter()
    .find(|r| r.audit_id == conflict_id)
    .unwrap();
  assert_eq!(row.result, AuditResult::Conflict);
  assert_eq!(row.summary_key, "conflict");
  assert_eq!(row.resolved_by_audit_id, None);

  f.store
    .put_document(
      &f.editor,
      "acme/demo",
      1,
      DocumentKind::Plan,
      "# Plan 4",
      "edit",
      Some(other),
    )
    .unwrap();
  let by_other = f
    .store
    .project_activity(&f.reader, "acme/demo", ActivityFilter::All, None, 50)
    .unwrap();
  let row = by_other
    .rows
    .iter()
    .find(|r| r.audit_id == conflict_id)
    .unwrap();
  assert_eq!(row.resolved_by_audit_id, None);

  let current = common::int(
    &f.store,
    "SELECT max(r.id) FROM revisions r JOIN documents d ON d.id = r.document_id WHERE d.kind = 'plan'",
  );
  f.store
    .put_document(
      &claude,
      "acme/demo",
      1,
      DocumentKind::Plan,
      "# Plan 5",
      "edit",
      Some(current),
    )
    .unwrap();
  let saved_id = last_audit_id(&f);
  let resolved = f
    .store
    .project_activity(
      &f.reader,
      "acme/demo",
      ActivityFilter::All,
      Some(saved_id),
      50,
    )
    .unwrap();
  let row = resolved
    .rows
    .iter()
    .find(|r| r.audit_id == conflict_id)
    .unwrap();
  assert_eq!(row.resolved_by_audit_id, Some(saved_id));
  assert_eq!(row.summary_key, "conflict_resolved");
}
