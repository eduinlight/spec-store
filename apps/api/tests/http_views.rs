mod common;

use axum::http::StatusCode;
use sdd_vault_core::model::{Caller, DocumentKind, Role, Via};
use serde_json::json;

const SPEC_V1: &str = "# Spec 001 — Billing views

## 1. Context and goal

Show invoices as cards.

## 5. Functional requirements

- **FR-1** (US-1): THE SYSTEM SHALL list invoices. [NEEDS CLARIFICATION: which currency?]
- **FR-2** (US-1): THE SYSTEM SHALL show totals.
";

const SPEC_V2: &str = "# Spec 001 — Billing views

## 1. Context and goal

Show invoices as cards.

## 5. Functional requirements

- **FR-1** (US-1): THE SYSTEM SHALL list invoices in euros.
- **FR-2** (US-1): THE SYSTEM SHALL show totals.
";

const TASKS_V1: &str = "# Tasks\n\n## Phase 1 — Setup\n- [ ] T01 (FR-1) First — Done when: green\n- [ ] T02 [P] (FR-1, FR-2) Second — Done when: green\n- [ ] T03 (FR: —) Housekeeping — Done when: clean\n";

const TASKS_V2: &str = "# Tasks\n\n## Phase 1 — Setup\n- [x] T01 (FR-1) First — Done when: green · commit: abc1234\n- [ ] T02 [P] (FR-1, FR-2) Second — Done when: green\n- [ ] T03 (FR: —) Housekeeping — Done when: clean\n";

fn via(caller: &Caller, client: &str) -> Caller {
  caller.clone().with_via(Via::new(client, None))
}

async fn seeded() -> common::TestApp {
  let app = common::app().await;
  app.organization("alice", "acme").await;
  app.admit("acme", "rita");
  app
    .json(
      "POST",
      "/api/v1/orgs/acme/projects",
      "alice",
      json!({"slug": "demo", "name": "Demo"}),
    )
    .await;
  let store = &app.state.store;
  let alice = store.caller_by_handle("alice", false).unwrap();
  store
    .set_member(&alice, "acme/demo", "rita", Role::Reader)
    .unwrap();
  let cursor = via(&alice, "Cursor");
  let claude = via(&alice, "Claude Code");
  store.create_spec(&cursor, "acme/demo", "billing").unwrap();
  let put = |caller: &Caller, kind, body: &str, expected| {
    store
      .put_document(caller, "acme/demo", 1, kind, body, "edit", expected)
      .unwrap()
  };
  let spec = put(&cursor, DocumentKind::Spec, SPEC_V1, None);
  let tasks = put(&cursor, DocumentKind::Tasks, TASKS_V1, None);
  put(&claude, DocumentKind::Tasks, TASKS_V2, Some(tasks));
  put(&claude, DocumentKind::Spec, SPEC_V2, Some(spec));
  app
}

async fn read(app: &common::TestApp, path: &str) -> serde_json::Value {
  let response = app
    .authorized("GET", &format!("/api/v1/projects/acme/demo{path}"), "rita")
    .await;
  assert_eq!(response.status(), StatusCode::OK, "{path}");
  common::body_json(response).await
}

#[tokio::test]
async fn cards_list_every_spec_with_its_summary_fields() {
  let app = seeded().await;
  let body = read(&app, "/specs/cards").await;
  let card = &body["specs"][0];
  assert_eq!(card["number"], 1);
  assert_eq!(card["slug"], "billing");
  assert_eq!(card["title"], "Billing views");
  assert_eq!(card["summary"], "Show invoices as cards.");
  assert_eq!(card["status"], "draft");
  assert_eq!(card["branch"], "001-billing");
  assert_eq!(card["documents"], json!(["spec", "tasks"]));
  assert_eq!(card["requirement_count"], 2);
  assert_eq!(card["tasks_done"], 1);
  assert_eq!(card["tasks_total"], 3);
  assert_eq!(card["open_markers"], 0);
  assert_eq!(card["contributors"], json!(["alice"]));
  assert!(card["created_at"].as_str().unwrap().ends_with('Z'));
  assert!(card["updated_at"].as_str().unwrap() >= card["created_at"].as_str().unwrap());
}

#[tokio::test]
async fn traceability_returns_requirement_rows_and_orphan_tasks() {
  let app = seeded().await;
  let body = read(&app, "/specs/1/traceability").await;
  let first = &body["requirements"][0];
  assert_eq!(first["id"], "FR-1");
  assert_eq!(first["text"], "THE SYSTEM SHALL list invoices in euros.");
  assert_eq!(first["status"], "in_progress");
  assert_eq!(first["tasks"][0]["id"], "T01");
  assert_eq!(first["tasks"][0]["done"], true);
  assert_eq!(first["tasks"][0]["commit"], "abc1234");
  assert_eq!(first["tasks"][1]["done"], false);
  assert_eq!(first["tasks"][1]["commit"], json!(null));
  assert_eq!(body["requirements"][1]["status"], "not_started");
  assert_eq!(body["tasks_without_requirements"][0]["id"], "T03");
}

#[tokio::test]
async fn readiness_lists_the_checklist() {
  let app = seeded().await;
  let body = read(&app, "/specs/1/readiness").await;
  assert_eq!(body["ready"], false);
  let items = body["items"].as_array().unwrap();
  let keys: Vec<&str> = items.iter().map(|i| i["key"].as_str().unwrap()).collect();
  assert_eq!(
    keys,
    [
      "open_markers_resolved",
      "requirements_covered",
      "plan_written",
      "tasks_written",
      "constitution_exists"
    ]
  );
  assert_eq!(items[0]["ok"], true);
  assert_eq!(items[0]["count"], 0);
  assert_eq!(items[1]["ok"], true);
  assert_eq!(items[1]["count"], 2);
  assert_eq!(items[1]["total"], 2);
  assert_eq!(items[2]["ok"], false);
  assert_eq!(items[2]["count"], json!(null));
  assert_eq!(items[3]["count"], 3);
}

#[tokio::test]
async fn markers_return_open_and_resolved() {
  let app = seeded().await;
  let body = read(&app, "/specs/1/markers").await;
  assert_eq!(body["markers"], json!([]));
  let resolved = &body["resolved"][0];
  assert_eq!(resolved["document_kind"], "spec");
  assert_eq!(resolved["kind"], "clarification");
  assert_eq!(resolved["heading"], "5. Functional requirements");
  assert_eq!(resolved["text"], "which currency?");
  assert_eq!(resolved["resolved_by"]["author"], "alice");
  assert_eq!(resolved["resolved_by"]["client"], "Claude Code");
  assert!(
    resolved["resolved_by"]["at"]
      .as_str()
      .unwrap()
      .ends_with('Z')
  );
  assert!(resolved["resolved_by"]["revision"].as_i64().unwrap() > 0);
}

#[tokio::test]
async fn tasks_carry_phase_flags_and_who_completed_them() {
  let app = seeded().await;
  let body = read(&app, "/specs/1/tasks").await;
  let tasks = body["tasks"].as_array().unwrap();
  assert_eq!(tasks.len(), 3);
  assert_eq!(tasks[0]["id"], "T01");
  assert_eq!(tasks[0]["phase"], "Phase 1 — Setup");
  assert_eq!(tasks[0]["parallel"], false);
  assert_eq!(tasks[0]["frs"], json!(["FR-1"]));
  assert_eq!(tasks[0]["done_when"], "green");
  assert_eq!(tasks[0]["done"], true);
  assert_eq!(tasks[0]["commit"], "abc1234");
  assert_eq!(tasks[0]["completed_by"]["author"], "alice");
  assert_eq!(tasks[0]["completed_by"]["client"], "Claude Code");
  assert!(
    tasks[0]["completed_by"]["at"]
      .as_str()
      .unwrap()
      .ends_with('Z')
  );
  assert_eq!(tasks[1]["parallel"], true);
  assert_eq!(tasks[1]["completed_by"], json!(null));
  assert_eq!(body["malformed"], json!([]));
}

#[tokio::test]
async fn outsiders_and_unknown_specs_get_not_found() {
  let app = seeded().await;
  for path in [
    "/specs/cards",
    "/specs/1/traceability",
    "/specs/1/readiness",
    "/specs/1/markers",
    "/specs/1/tasks",
  ] {
    assert_eq!(
      app
        .authorized("GET", &format!("/api/v1/projects/acme/demo{path}"), "otto")
        .await
        .status(),
      StatusCode::NOT_FOUND,
      "{path}"
    );
  }
  for path in [
    "/specs/9/traceability",
    "/specs/9/readiness",
    "/specs/9/markers",
  ] {
    assert_eq!(
      app
        .authorized("GET", &format!("/api/v1/projects/acme/demo{path}"), "rita")
        .await
        .status(),
      StatusCode::NOT_FOUND,
      "{path}"
    );
  }
  assert_eq!(
    app
      .get("/api/v1/projects/acme/demo/specs/cards")
      .await
      .status(),
    StatusCode::UNAUTHORIZED
  );
}
