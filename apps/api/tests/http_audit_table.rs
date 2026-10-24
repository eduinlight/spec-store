mod common;

use axum::http::{StatusCode, header};
use serde_json::{Value, json};

async fn setup() -> common::TestApp {
  let app = common::app().await;
  app.organization("alice", "acme").await;
  app.admit("acme", "bob");
  app.admit("acme", "dave");
  app.organization("carol", "beta").await;
  app
    .json(
      "POST",
      "/api/v1/orgs/acme/projects",
      "alice",
      json!({"slug": "demo", "name": "Demo"}),
    )
    .await;
  app
    .json(
      "POST",
      "/api/v1/orgs/acme/projects",
      "alice",
      json!({"slug": "other", "name": "Other"}),
    )
    .await;
  app
    .json(
      "PUT",
      "/api/v1/projects/acme/demo/members/bob",
      "alice",
      json!({"role": "reader"}),
    )
    .await;
  let refused = app
    .json(
      "PUT",
      "/api/v1/projects/acme/demo/members/bob",
      "bob",
      json!({"role": "owner"}),
    )
    .await;
  assert_eq!(refused.status(), StatusCode::FORBIDDEN);
  app
}

async fn table(app: &common::TestApp, path: &str, user: &str) -> Value {
  let response = app.authorized("GET", path, user).await;
  assert_eq!(response.status(), StatusCode::OK, "{path}");
  common::body_json(response).await
}

fn actions(table: &Value) -> Vec<String> {
  table["entries"]
    .as_array()
    .unwrap()
    .iter()
    .map(|e| e["action"].as_str().unwrap().to_string())
    .collect()
}

#[tokio::test]
async fn project_table_returns_entries_total_and_facets() {
  let app = setup().await;
  let page = table(&app, "/api/v1/projects/acme/demo/audit/table", "alice").await;
  assert_eq!(page["total"], 3);
  assert_eq!(page["page"], 1);
  assert_eq!(page["page_size"], 25);
  assert_eq!(
    actions(&page),
    vec!["member.set", "member.set", "project.create"]
  );
  let first = &page["entries"][0];
  for field in [
    "id",
    "actor",
    "organization",
    "project",
    "action",
    "target",
    "created_at",
    "client",
    "origin",
    "result",
    "detail",
  ] {
    assert!(first.get(field).is_some(), "{field}");
  }
  assert_eq!(first["project"], "acme/demo");
  assert_eq!(first["result"], "denied");
  assert_eq!(first["actor"], "bob");
  let facets = &page["facets"];
  assert_eq!(facets["actors"][0], json!({"value": "alice", "count": 2}));
  assert_eq!(facets["actors"][1], json!({"value": "bob", "count": 1}));
  assert_eq!(facets["results"][0], json!({"value": "ok", "count": 2}));
  assert_eq!(facets["results"][1], json!({"value": "denied", "count": 1}));
  assert!(facets["clients"].is_array());
  assert!(facets["actions"].is_array());
}

#[tokio::test]
async fn organization_table_covers_every_project_of_the_organization() {
  let app = setup().await;
  let page = table(&app, "/api/v1/orgs/acme/audit/table", "alice").await;
  assert_eq!(page["total"], 7);
  let projects: Vec<Option<&str>> = page["entries"]
    .as_array()
    .unwrap()
    .iter()
    .map(|e| e["project"].as_str())
    .collect();
  assert!(projects.contains(&Some("acme/other")));
  assert!(projects.contains(&None));
}

#[tokio::test]
async fn filters_sort_and_paging_come_from_query_parameters() {
  let app = setup().await;
  let base = "/api/v1/projects/acme/demo/audit/table";
  let denied = table(&app, &format!("{base}?result=denied"), "alice").await;
  assert_eq!(denied["total"], 1);
  let both = table(&app, &format!("{base}?actor=alice&actor=bob"), "alice").await;
  assert_eq!(both["total"], 3);
  let one = table(&app, &format!("{base}?actor=bob"), "alice").await;
  assert_eq!(one["total"], 1);
  assert_eq!(one["facets"]["actors"].as_array().unwrap().len(), 2);
  let action = table(&app, &format!("{base}?action=project.create"), "alice").await;
  assert_eq!(action["total"], 1);
  let text = table(&app, &format!("{base}?q=BOB"), "alice").await;
  assert_eq!(text["total"], 2);
  let oldest = table(&app, &format!("{base}?sort=oldest"), "alice").await;
  assert_eq!(actions(&oldest)[0], "project.create");
  let paged = table(&app, &format!("{base}?page=2&page_size=10"), "alice").await;
  assert_eq!(paged["page"], 2);
  assert_eq!(paged["page_size"], 10);
  assert_eq!(paged["total"], 3);
  assert!(paged["entries"].as_array().unwrap().is_empty());
  let period = table(&app, &format!("{base}?period=day"), "alice").await;
  assert_eq!(period["total"], 3);
  let encoded = table(&app, &format!("{base}?q=%25"), "alice").await;
  assert_eq!(encoded["total"], 0);
}

#[tokio::test]
async fn invalid_parameters_are_validation_errors() {
  let app = setup().await;
  let base = "/api/v1/projects/acme/demo/audit/table";
  for query in [
    "page=0",
    "page_size=7",
    "page_size=abc",
    "period=year",
    "sort=sideways",
    "result=unknown",
  ] {
    let status = app
      .authorized("GET", &format!("{base}?{query}"), "alice")
      .await
      .status();
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{query}");
  }
}

#[tokio::test]
async fn table_access_follows_the_audit_rules() {
  let app = setup().await;
  let project = "/api/v1/projects/acme/demo/audit/table";
  let org = "/api/v1/orgs/acme/audit/table";
  let status = |path: &'static str, user: &'static str| {
    let app = &app;
    async move { app.authorized("GET", path, user).await.status() }
  };
  assert_eq!(status(project, "bob").await, StatusCode::FORBIDDEN);
  assert_eq!(status(org, "bob").await, StatusCode::FORBIDDEN);
  assert_eq!(status(project, "dave").await, StatusCode::NOT_FOUND);
  assert_eq!(status(org, "carol").await, StatusCode::NOT_FOUND);
  assert_eq!(status(project, "ops").await, StatusCode::OK);
  assert_eq!(status(org, "ops").await, StatusCode::OK);
  assert_eq!(
    status("/api/v1/orgs/missing/audit/table", "alice").await,
    StatusCode::NOT_FOUND
  );
  let anonymous = app.get(project).await;
  assert_eq!(anonymous.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn existing_audit_endpoints_keep_working() {
  let app = setup().await;
  let page = table(&app, "/api/v1/projects/acme/demo/audit", "alice").await;
  assert_eq!(page["entries"].as_array().unwrap().len(), 3);
  assert_eq!(page["has_more"], false);
  let org = table(&app, "/api/v1/orgs/acme/audit", "alice").await;
  assert_eq!(org["entries"].as_array().unwrap().len(), 7);
}

#[tokio::test]
async fn export_downloads_the_filtered_entries_as_csv() {
  let app = setup().await;
  let response = app
    .authorized(
      "GET",
      "/api/v1/projects/acme/demo/audit/export.csv?result=ok&sort=oldest",
      "alice",
    )
    .await;
  assert_eq!(response.status(), StatusCode::OK);
  assert_eq!(
    response.headers()[header::CONTENT_TYPE],
    "text/csv; charset=utf-8"
  );
  let disposition = response.headers()[header::CONTENT_DISPOSITION]
    .to_str()
    .unwrap()
    .to_string();
  assert!(
    disposition.starts_with("attachment; filename=\"audit-acme-demo-"),
    "{disposition}"
  );
  assert!(disposition.ends_with(".csv\""), "{disposition}");
  let body = common::body_text(response).await;
  let lines: Vec<&str> = body.split("\r\n").filter(|l| !l.is_empty()).collect();
  assert_eq!(
    lines[0],
    "time,actor,client,action,target,result,detail,origin"
  );
  assert_eq!(lines.len(), 3);
  assert!(lines[1].contains(",alice,"));
  assert!(lines[1].contains(",project.create,"));
}

#[tokio::test]
async fn export_by_ids_and_the_export_itself_is_audited() {
  let app = setup().await;
  let page = table(&app, "/api/v1/projects/acme/demo/audit/table", "alice").await;
  let ids: Vec<String> = page["entries"]
    .as_array()
    .unwrap()
    .iter()
    .take(2)
    .map(|e| e["id"].as_i64().unwrap().to_string())
    .collect();
  let other = table(
    &app,
    "/api/v1/orgs/acme/audit/table?action=project.create",
    "alice",
  )
  .await;
  let foreign = other["entries"]
    .as_array()
    .unwrap()
    .iter()
    .find(|e| e["project"] == "acme/other")
    .unwrap()["id"]
    .as_i64()
    .unwrap();
  let query = format!("ids={},{},{foreign}", ids[0], ids[1]);
  let response = app
    .authorized(
      "GET",
      &format!("/api/v1/projects/acme/demo/audit/export.csv?{query}"),
      "alice",
    )
    .await;
  assert_eq!(response.status(), StatusCode::OK);
  let body = common::body_text(response).await;
  assert_eq!(body.split("\r\n").filter(|l| !l.is_empty()).count(), 3);
  let audited = table(
    &app,
    "/api/v1/projects/acme/demo/audit/table?action=audit.export",
    "alice",
  )
  .await;
  assert_eq!(audited["total"], 1);
  assert_eq!(audited["entries"][0]["target"], "2 entries");
  assert_eq!(audited["entries"][0]["actor"], "alice");

  let org = app
    .authorized("GET", "/api/v1/orgs/acme/audit/export.csv", "alice")
    .await;
  assert_eq!(org.status(), StatusCode::OK);
  let disposition = org.headers()[header::CONTENT_DISPOSITION]
    .to_str()
    .unwrap()
    .to_string();
  assert!(
    disposition.starts_with("attachment; filename=\"audit-acme-"),
    "{disposition}"
  );
  assert!(!disposition.contains("demo"));
}

#[tokio::test]
async fn export_rejects_bad_ids_and_unauthorized_callers() {
  let app = setup().await;
  let bad = app
    .authorized(
      "GET",
      "/api/v1/projects/acme/demo/audit/export.csv?ids=1,x",
      "alice",
    )
    .await;
  assert_eq!(bad.status(), StatusCode::UNPROCESSABLE_ENTITY);
  let forbidden = app
    .authorized("GET", "/api/v1/projects/acme/demo/audit/export.csv", "bob")
    .await;
  assert_eq!(forbidden.status(), StatusCode::FORBIDDEN);
  let hidden = app
    .authorized("GET", "/api/v1/orgs/acme/audit/export.csv", "carol")
    .await;
  assert_eq!(hidden.status(), StatusCode::NOT_FOUND);
  let anonymous = app.get("/api/v1/orgs/acme/audit/export.csv").await;
  assert_eq!(anonymous.status(), StatusCode::UNAUTHORIZED);
}
