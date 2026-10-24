mod common;

use axum::http::StatusCode;
use serde_json::json;

#[tokio::test]
async fn organization_owners_and_operators_read_the_audit_trail() {
  let app = common::app().await;
  app.organization("alice", "acme").await;
  app.admit("acme", "bob");
  app
    .json(
      "POST",
      "/api/v1/orgs/acme/projects",
      "alice",
      json!({"slug": "demo", "name": "Demo"}),
    )
    .await;
  app.organization("carol", "beta").await;

  let page = common::body_json(
    app
      .authorized("GET", "/api/v1/orgs/acme/audit", "alice")
      .await,
  )
  .await;
  let actions: Vec<&str> = page["entries"]
    .as_array()
    .unwrap()
    .iter()
    .map(|e| e["action"].as_str().unwrap())
    .collect();
  assert_eq!(
    actions,
    vec![
      "project.create",
      "organization.member.admit",
      "organization.create"
    ]
  );
  assert_eq!(page["has_more"], false);
  assert_eq!(page["entries"][0]["project"], "acme/demo");
  assert_eq!(page["entries"][0]["actor"], "alice");

  let filtered = common::body_json(
    app
      .authorized("GET", "/api/v1/orgs/acme/audit?project=demo", "alice")
      .await,
  )
  .await;
  assert_eq!(filtered["entries"].as_array().unwrap().len(), 1);
  let paged = common::body_json(
    app
      .authorized("GET", "/api/v1/orgs/acme/audit?limit=1", "alice")
      .await,
  )
  .await;
  assert_eq!(paged["entries"].as_array().unwrap().len(), 1);
  assert_eq!(paged["has_more"], true);
  let before = paged["entries"][0]["id"].as_i64().unwrap();
  let older = common::body_json(
    app
      .authorized(
        "GET",
        &format!("/api/v1/orgs/acme/audit?before={before}"),
        "alice",
      )
      .await,
  )
  .await;
  assert_eq!(older["entries"].as_array().unwrap().len(), 2);

  assert_eq!(
    app
      .authorized("GET", "/api/v1/orgs/acme/audit", "bob")
      .await
      .status(),
    StatusCode::FORBIDDEN
  );
  assert_eq!(
    app
      .authorized("GET", "/api/v1/orgs/acme/audit", "carol")
      .await
      .status(),
    StatusCode::NOT_FOUND
  );
  assert_eq!(
    app
      .authorized("GET", "/api/v1/orgs/acme/audit", "ops")
      .await
      .status(),
    StatusCode::OK
  );

  let global = common::body_json(app.authorized("GET", "/api/v1/audit", "ops").await).await;
  assert_eq!(global["entries"].as_array().unwrap().len(), 4);
  let beta = common::body_json(app.authorized("GET", "/api/v1/audit?org=beta", "ops").await).await;
  assert_eq!(beta["entries"].as_array().unwrap().len(), 1);
  assert_eq!(
    app
      .authorized("GET", "/api/v1/audit", "alice")
      .await
      .status(),
    StatusCode::FORBIDDEN
  );
}

#[tokio::test]
async fn project_owners_read_their_project_audit() {
  let app = common::app().await;
  app.organization("alice", "acme").await;
  app.admit("acme", "bob");
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
      "PUT",
      "/api/v1/projects/acme/demo/members/bob",
      "alice",
      json!({"role": "owner"}),
    )
    .await;
  app
    .json(
      "PUT",
      "/api/v1/projects/acme/demo/members/alice",
      "alice",
      json!({"role": "owner"}),
    )
    .await;
  let page = common::body_json(
    app
      .authorized("GET", "/api/v1/projects/acme/demo/audit", "bob")
      .await,
  )
  .await;
  assert_eq!(page["entries"].as_array().unwrap().len(), 3);
  app
    .json(
      "PUT",
      "/api/v1/projects/acme/demo/members/bob",
      "alice",
      json!({"role": "reader"}),
    )
    .await;
  assert_eq!(
    app
      .authorized("GET", "/api/v1/projects/acme/demo/audit", "bob")
      .await
      .status(),
    StatusCode::FORBIDDEN
  );
}
