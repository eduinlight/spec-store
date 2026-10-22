mod common;

use axum::http::StatusCode;
use serde_json::{Value, json};
use spec_store_core::model::Role;

async fn setup() -> common::TestApp {
  let app = common::app().await;
  app.organization("alice", "acme").await;
  app.admit("acme", "eddie");
  app
    .json(
      "POST",
      "/api/v1/orgs/acme/projects",
      "alice",
      json!({"slug": "demo", "name": "Demo"}),
    )
    .await;
  let alice = app.state.store.caller_by_handle("alice", false).unwrap();
  app
    .state
    .store
    .set_member(&alice, "acme/demo", "eddie", Role::Editor)
    .unwrap();
  app
}

fn tree() -> Value {
  json!({
    "constitution": {"relative_path": "docs/constitution.md", "body": "# C\n\nVersion 1.0\n"},
    "specs": [{
      "folder": "001-alpha",
      "number": 1,
      "slug": "alpha",
      "documents": [["spec", {"relative_path": "specs/001-alpha/spec.md", "body": "# S\r\n\r\nStatus: approved\r\n"}]]
    }],
    "skipped": [],
    "problems": []
  })
}

#[tokio::test]
async fn owners_import_and_dry_runs_change_nothing() {
  let app = setup().await;
  let dry = app
    .json(
      "POST",
      "/api/v1/projects/acme/demo/import",
      "alice",
      json!({"tree": tree(), "dry_run": true}),
    )
    .await;
  assert_eq!(dry.status(), StatusCode::OK);
  let report = common::body_json(dry).await;
  assert_eq!(report["specs"][0]["branch"], "001-alpha");
  assert_eq!(report["specs"][0]["status"], "approved");
  assert_eq!(report["documents"], 2);
  let alice = app.state.store.caller_by_handle("alice", false).unwrap();
  assert!(
    app
      .state
      .store
      .list_specs(&alice, "acme/demo")
      .unwrap()
      .is_empty()
  );

  let real = app
    .json(
      "POST",
      "/api/v1/projects/acme/demo/import",
      "alice",
      json!({"tree": tree()}),
    )
    .await;
  assert_eq!(real.status(), StatusCode::OK);
  let exported = common::body_json(
    app
      .authorized("GET", "/api/v1/projects/acme/demo/export", "eddie")
      .await,
  )
  .await;
  let files = exported["files"].as_array().unwrap();
  assert_eq!(files.len(), 2);
  assert_eq!(files[1]["body"], "# S\r\n\r\nStatus: approved\r\n");
  let one = common::body_json(
    app
      .authorized("GET", "/api/v1/projects/acme/demo/export?spec=1", "eddie")
      .await,
  )
  .await;
  assert_eq!(one["files"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn editors_cannot_import_and_outsiders_get_not_found() {
  let app = setup().await;
  let refused = app
    .json(
      "POST",
      "/api/v1/projects/acme/demo/import",
      "eddie",
      json!({"tree": tree()}),
    )
    .await;
  assert_eq!(refused.status(), StatusCode::FORBIDDEN);
  let outsider = app
    .json(
      "POST",
      "/api/v1/projects/acme/demo/import",
      "otto",
      json!({"tree": tree()}),
    )
    .await;
  assert_eq!(outsider.status(), StatusCode::NOT_FOUND);
  assert_eq!(
    app
      .authorized("GET", "/api/v1/projects/acme/demo/export", "otto")
      .await
      .status(),
    StatusCode::NOT_FOUND
  );
  let unauthenticated = app
    .send(
      axum::http::Request::post("/api/v1/projects/acme/demo/import")
        .header("host", "api.example.test")
        .header("content-type", "application/json")
        .body(axum::body::Body::from(json!({"tree": tree()}).to_string()))
        .unwrap(),
    )
    .await;
  assert_eq!(unauthenticated.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn imports_larger_than_a_single_document_are_accepted() {
  let app = setup().await;
  let body = "x".repeat(900 * 1024);
  let mut big = tree();
  big["specs"][0]["documents"] = json!([
    ["spec", {"relative_path": "specs/001-alpha/spec.md", "body": body}],
    ["plan", {"relative_path": "specs/001-alpha/plan.md", "body": body}],
    ["tasks", {"relative_path": "specs/001-alpha/tasks.md", "body": body}]
  ]);
  let response = app
    .json(
      "POST",
      "/api/v1/projects/acme/demo/import",
      "alice",
      json!({"tree": big}),
    )
    .await;
  assert_eq!(response.status(), StatusCode::OK);
}
