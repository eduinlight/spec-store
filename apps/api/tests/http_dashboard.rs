mod common;

use axum::http::StatusCode;
use sdd_vault_core::model::{Caller, DocumentKind, Role, SpecStatus, Via};
use serde_json::json;

const SPEC_1: &str = "# Spec 001 — Billing views

## 1. Context and goal

Show invoices as cards.

## 5. Functional requirements

- **FR-1** (US-1): THE SYSTEM SHALL list invoices.
- **FR-2** (US-1): THE SYSTEM SHALL export. [NEEDS CLARIFICATION: which format?]
";

const TASKS_1: &str = "# Tasks\n\n## Phase 1 — Setup\n- [ ] T01 (FR-1) First — Done when: green\n- [ ] T02 [P] (FR-1) Second — Done when: shown\n";

const SPEC_2: &str = "# Spec 002 — Search\n\n## 1. Context and goal\n\nFind invoices quickly.\n\n## 5. Functional requirements\n\n- **FR-1** (US-1): THE SYSTEM SHALL search. [NEEDS CLARIFICATION: fuzzy or exact?]\n";

fn via(caller: &Caller, client: &str) -> Caller {
  caller.clone().with_via(Via::new(client, None))
}

async fn seeded() -> common::TestApp {
  let app = common::app().await;
  app.organization("alice", "acme").await;
  app.admit("acme", "rita");
  app.admit("acme", "mallory");
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
  let put = |number, kind, body: &str| {
    store
      .put_document(&cursor, "acme/demo", number, kind, body, "edit", None)
      .unwrap()
  };
  put(1, DocumentKind::Spec, SPEC_1);
  put(1, DocumentKind::Tasks, TASKS_1);
  store
    .mark_task_done(&claude, "acme/demo", 1, "T01", "abc1234")
    .unwrap();
  store
    .set_spec_status(&alice, "acme/demo", 1, SpecStatus::Clarified)
    .unwrap();
  store.create_spec(&cursor, "acme/demo", "search").unwrap();
  put(2, DocumentKind::Spec, SPEC_2);
  app
}

async fn read(app: &common::TestApp, path: &str, user: &str) -> serde_json::Value {
  let response = app
    .authorized("GET", &format!("/api/v1/projects/acme/demo{path}"), user)
    .await;
  assert_eq!(response.status(), StatusCode::OK, "{path}");
  common::body_json(response).await
}

#[tokio::test]
async fn overview_reports_kpis_the_daily_chart_and_attention_items() {
  let app = seeded().await;
  let overview = read(&app, "/overview?days=7", "alice").await;
  assert_eq!(overview["days"], 7);
  assert_eq!(overview["active_specs"], 2);
  assert_eq!(overview["tasks_completed"], 1);
  assert_eq!(overview["tasks_completed_previous"], 0);
  assert_eq!(
    overview["coverage"],
    json!({"spec_number": 2, "covered": 0, "total": 1})
  );
  assert_eq!(overview["open_markers"], 2);
  assert_eq!(overview["agents_active"], 3);
  let per_day = overview["completed_per_day"].as_array().unwrap();
  assert_eq!(per_day.len(), 7);
  assert_eq!(
    per_day[6]["by_client"],
    json!([{"client": "Claude Code", "count": 1}])
  );
  assert!(per_day[0]["day"].as_str().unwrap().len() == 10);
  assert_eq!(
    overview["attention"],
    json!([
      {"kind": "approval_waiting", "spec_number": 1, "spec_slug": "billing", "count": 1, "detail": []},
      {"kind": "open_questions", "spec_number": 1, "spec_slug": "billing", "count": 1, "detail": []},
      {"kind": "open_questions", "spec_number": 2, "spec_slug": "search", "count": 1, "detail": []},
      {"kind": "requirement_without_work", "spec_number": 2, "spec_slug": "search", "count": 1, "detail": ["FR-1"]},
    ])
  );
  let reader = read(&app, "/overview", "rita").await;
  assert_eq!(reader["days"], 7);
  assert!(
    reader["attention"]
      .as_array()
      .unwrap()
      .iter()
      .all(|item| item["kind"] != "approval_waiting")
  );
}

#[tokio::test]
async fn overview_validates_the_period_and_hides_the_project() {
  let app = seeded().await;
  for days in ["5", "0", "31", "-7"] {
    let response = app
      .authorized(
        "GET",
        &format!("/api/v1/projects/acme/demo/overview?days={days}"),
        "alice",
      )
      .await;
    assert_eq!(
      response.status(),
      StatusCode::UNPROCESSABLE_ENTITY,
      "{days}"
    );
    assert_eq!(common::body_json(response).await["code"], "validation");
  }
  for path in ["overview", "active", "activity"] {
    let response = app
      .authorized(
        "GET",
        &format!("/api/v1/projects/acme/demo/{path}"),
        "mallory",
      )
      .await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND, "{path}");
    let response = app.get(&format!("/api/v1/projects/acme/demo/{path}")).await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED, "{path}");
  }
}

#[tokio::test]
async fn active_returns_the_next_task_the_prompt_and_the_drafts() {
  let app = seeded().await;
  app
    .state
    .store
    .create_spec(&app.person("alice"), "acme/demo", "third")
    .unwrap();
  let active = read(&app, "/active", "rita").await;
  assert_eq!(active["active"]["number"], 3);
  assert!(active["active"]["next_task"].is_null());
  assert!(active["active"]["agent_prompt"].is_null());
  assert_eq!(active["drafts"].as_array().unwrap().len(), 1);
  assert_eq!(active["drafts"][0]["slug"], "search");
  assert_eq!(active["drafts"][0]["title"], "Search");
  assert_eq!(active["drafts"][0]["summary"], "Find invoices quickly.");
  assert_eq!(
    active["drafts"][0]["markers"],
    json!([{
      "kind": "clarification",
      "document_kind": "spec",
      "heading": "5. Functional requirements",
      "text": "fuzzy or exact?",
      "line": 9
    }])
  );
  app
    .state
    .store
    .set_spec_status(&app.person("alice"), "acme/demo", 3, SpecStatus::Archived)
    .unwrap();
  app
    .state
    .store
    .set_spec_status(&app.person("alice"), "acme/demo", 2, SpecStatus::Archived)
    .unwrap();
  let active = read(&app, "/active", "rita").await;
  assert_eq!(
    active["active"],
    json!({
      "number": 1,
      "slug": "billing",
      "title": "Billing views",
      "status": "clarified",
      "branch": "001-billing",
      "tasks_done": 1,
      "tasks_total": 2,
      "current_phase": "Phase 1 — Setup",
      "open_markers": 1,
      "next_task": {
        "id": "T02",
        "parallel": true,
        "frs": ["FR-1"],
        "description": "Second",
        "done_when": "shown"
      },
      "agent_prompt": "Work on SDD Vault project acme/demo, spec 001-billing, task T02: Second. Done when: shown. Read the spec, plan and task through the sdd-vault MCP server first, follow the constitution, write tests first, and mark the task done with its commit."
    })
  );
  assert_eq!(active["drafts"], json!([]));
}

#[tokio::test]
async fn activity_lists_rows_filters_and_pages() {
  let app = seeded().await;
  let all = read(&app, "/activity", "rita").await;
  assert_eq!(all["has_more"], false);
  let rows = all["rows"].as_array().unwrap();
  assert_eq!(rows[0]["action"], "document.put");
  assert_eq!(rows[0]["actor"], "alice");
  assert_eq!(rows[0]["client"], "Cursor");
  assert_eq!(rows[0]["result"], "ok");
  assert_eq!(rows[0]["summary_key"], "document.put");
  assert!(rows[0]["resolved_by_audit_id"].is_null());
  assert!(rows[0]["at"].as_str().unwrap().ends_with('Z'));
  assert!(rows[0]["audit_id"].as_i64().unwrap() > rows[1]["audit_id"].as_i64().unwrap());

  let tasks = read(&app, "/activity?filter=tasks", "rita").await;
  assert_eq!(tasks["rows"].as_array().unwrap().len(), 1);
  assert_eq!(tasks["rows"][0]["action"], "task.done");
  assert_eq!(tasks["rows"][0]["target"], "001/T01@abc1234");
  let documents = read(&app, "/activity?filter=documents", "rita").await;
  assert!(
    documents["rows"]
      .as_array()
      .unwrap()
      .iter()
      .all(|row| row["action"] != "task.done" && row["action"] != "member.set")
  );
  let security = read(&app, "/activity?filter=security", "rita").await;
  assert!(
    security["rows"]
      .as_array()
      .unwrap()
      .iter()
      .any(|row| row["action"] == "member.set")
  );

  let first = read(&app, "/activity?limit=2", "rita").await;
  assert_eq!(first["rows"].as_array().unwrap().len(), 2);
  assert_eq!(first["has_more"], true);
  let before = first["rows"][1]["audit_id"].as_i64().unwrap();
  let second = read(&app, &format!("/activity?limit=2&before={before}"), "rita").await;
  assert!(
    second["rows"]
      .as_array()
      .unwrap()
      .iter()
      .all(|row| row["audit_id"].as_i64().unwrap() < before)
  );

  let response = app
    .authorized(
      "GET",
      "/api/v1/projects/acme/demo/activity?filter=everything",
      "rita",
    )
    .await;
  assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}
