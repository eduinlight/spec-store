mod common;

use axum::http::StatusCode;
use serde_json::json;
use spec_store_core::model::{DocumentKind, Role};

const TASKS: &str = "# Tasks\n\n## Phase 1 — Setup\n- [x] T01 (FR-1) First task — Done when: done `abc1234`\n- [ ] T02 [P] (FR-2) Second task — Done when: green\n";

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
  store
    .put_constitution(
      &alice,
      "acme/demo",
      "# Constitution\n",
      "1.0",
      "ratify",
      None,
    )
    .unwrap();
  store.create_spec(&alice, "acme/demo", "first").unwrap();
  let first = store
    .put_document(
      &alice,
      "acme/demo",
      1,
      DocumentKind::Spec,
      "# Spec\n\nline a\n",
      "draft",
      None,
    )
    .unwrap();
  store
    .put_document(
      &alice,
      "acme/demo",
      1,
      DocumentKind::Spec,
      "# Spec\n\nline b\n\n[NEEDS CLARIFICATION: who?]\n",
      "second",
      Some(first),
    )
    .unwrap();
  store
    .put_document(
      &alice,
      "acme/demo",
      1,
      DocumentKind::Tasks,
      TASKS,
      "tasks",
      None,
    )
    .unwrap();
  app
}

#[tokio::test]
async fn project_status_specs_and_spec_summary() {
  let app = seeded().await;
  let status = common::body_json(
    app
      .authorized("GET", "/api/v1/projects/acme/demo/status", "rita")
      .await,
  )
  .await;
  assert_eq!(status["constitution_version"], "1.0");
  assert_eq!(status["specs"][0]["branch"], "001-first");
  assert_eq!(status["active"]["tasks_done"], 1);
  assert_eq!(status["active"]["tasks_total"], 2);
  assert_eq!(status["active"]["markers"], 1);
  assert_eq!(status["active"]["next_task"]["id"], "T02");

  let specs = common::body_json(
    app
      .authorized("GET", "/api/v1/projects/acme/demo/specs", "rita")
      .await,
  )
  .await;
  assert_eq!(specs["specs"][0]["kinds"], json!(["spec", "tasks"]));
  let spec = common::body_json(
    app
      .authorized("GET", "/api/v1/projects/acme/demo/specs/1", "rita")
      .await,
  )
  .await;
  assert_eq!(spec["slug"], "first");
  assert_eq!(spec["status"], "draft");
  assert_eq!(
    app
      .authorized("GET", "/api/v1/projects/acme/demo/specs/9", "rita")
      .await
      .status(),
    StatusCode::NOT_FOUND
  );
}

#[tokio::test]
async fn documents_constitution_tasks_and_markers() {
  let app = seeded().await;
  let document = common::body_json(
    app
      .authorized(
        "GET",
        "/api/v1/projects/acme/demo/specs/1/documents/spec",
        "rita",
      )
      .await,
  )
  .await;
  assert!(document["body"].as_str().unwrap().contains("line b"));
  assert_eq!(document["revision"]["author"], "alice");
  assert_eq!(document["revision"]["message"], "second");
  assert_eq!(
    app
      .authorized(
        "GET",
        "/api/v1/projects/acme/demo/specs/1/documents/plan",
        "rita"
      )
      .await
      .status(),
    StatusCode::NOT_FOUND
  );
  let constitution = common::body_json(
    app
      .authorized("GET", "/api/v1/projects/acme/demo/constitution", "rita")
      .await,
  )
  .await;
  assert_eq!(constitution["version"], "1.0");

  let tasks = common::body_json(
    app
      .authorized("GET", "/api/v1/projects/acme/demo/specs/1/tasks", "rita")
      .await,
  )
  .await;
  assert_eq!(tasks["tasks"][0]["id"], "T01");
  assert_eq!(tasks["tasks"][0]["done"], true);
  assert_eq!(tasks["tasks"][0]["commit"], "abc1234");
  assert_eq!(tasks["tasks"][1]["parallel"], true);
  assert_eq!(tasks["tasks"][1]["frs"], json!(["FR-2"]));
  assert_eq!(tasks["tasks"][1]["phase"], "Phase 1 — Setup");

  let markers = common::body_json(
    app
      .authorized("GET", "/api/v1/projects/acme/demo/specs/1/markers", "rita")
      .await,
  )
  .await;
  assert_eq!(markers["markers"][0]["kind"], "clarification");
  assert_eq!(markers["markers"][0]["text"], "who?");
  assert_eq!(markers["markers"][0]["document_kind"], "spec");
}

#[tokio::test]
async fn history_revisions_and_diffs() {
  let app = seeded().await;
  let history = common::body_json(
    app
      .authorized(
        "GET",
        "/api/v1/projects/acme/demo/revisions?spec=1&kind=spec",
        "rita",
      )
      .await,
  )
  .await;
  let revisions = history["revisions"].as_array().unwrap();
  assert_eq!(revisions.len(), 2);
  assert_eq!(revisions[0]["message"], "second");
  let newest = revisions[0]["id"].as_i64().unwrap();
  let oldest = revisions[1]["id"].as_i64().unwrap();

  let current = common::body_json(
    app
      .authorized(
        "GET",
        &format!("/api/v1/projects/acme/demo/revisions/{newest}"),
        "rita",
      )
      .await,
  )
  .await;
  assert_eq!(current["current"], true);
  assert_eq!(current["kind"], "spec");
  assert_eq!(current["number"], 1);
  let old = common::body_json(
    app
      .authorized(
        "GET",
        &format!("/api/v1/projects/acme/demo/revisions/{oldest}"),
        "rita",
      )
      .await,
  )
  .await;
  assert_eq!(old["current"], false);
  assert_eq!(old["current_revision"], newest);

  let diff = common::body_json(
    app
      .authorized(
        "GET",
        &format!("/api/v1/projects/acme/demo/revisions/{newest}/diff"),
        "rita",
      )
      .await,
  )
  .await;
  assert_eq!(diff["previous_revision"], oldest);
  let lines = diff["hunks"][0]["lines"].as_array().unwrap();
  assert!(
    lines
      .iter()
      .any(|l| l["tag"] == "delete" && l["text"] == "line a")
  );
  assert!(
    lines
      .iter()
      .any(|l| l["tag"] == "insert" && l["text"] == "line b")
  );
  let first_diff = common::body_json(
    app
      .authorized(
        "GET",
        &format!("/api/v1/projects/acme/demo/revisions/{oldest}/diff"),
        "rita",
      )
      .await,
  )
  .await;
  assert_eq!(first_diff["previous_revision"], serde_json::Value::Null);

  let constitution_history = common::body_json(
    app
      .authorized(
        "GET",
        "/api/v1/projects/acme/demo/revisions?kind=constitution",
        "rita",
      )
      .await,
  )
  .await;
  assert_eq!(constitution_history["revisions"][0]["version"], "1.0");
  assert_eq!(
    app
      .authorized("GET", "/api/v1/projects/acme/demo/revisions/99999", "rita")
      .await
      .status(),
    StatusCode::NOT_FOUND
  );
}

#[tokio::test]
async fn outsiders_read_nothing() {
  let app = seeded().await;
  for path in [
    "/api/v1/projects/acme/demo/status",
    "/api/v1/projects/acme/demo/specs",
    "/api/v1/projects/acme/demo/specs/1/documents/spec",
    "/api/v1/projects/acme/demo/constitution",
    "/api/v1/projects/acme/demo/revisions?kind=constitution",
  ] {
    assert_eq!(
      app.authorized("GET", path, "otto").await.status(),
      StatusCode::NOT_FOUND,
      "{path}"
    );
  }
}
