mod common;

use axum::http::StatusCode;
use serde_json::json;

async fn setup() -> common::TestApp {
  let app = common::app().await;
  app.organization("alice", "acme").await;
  app.admit("acme", "bob");
  let created = app
    .json(
      "POST",
      "/api/v1/orgs/acme/projects",
      "alice",
      json!({"slug": "demo", "name": "Demo"}),
    )
    .await;
  assert_eq!(created.status(), StatusCode::CREATED);
  let body = common::body_json(created).await;
  assert_eq!(body["address"], "acme/demo");
  app
}

#[tokio::test]
async fn owners_create_projects_and_members_see_them_with_their_role() {
  let app = setup().await;
  let set = app
    .json(
      "PUT",
      "/api/v1/projects/acme/demo/members/bob",
      "alice",
      json!({"role": "editor"}),
    )
    .await;
  assert_eq!(set.status(), StatusCode::NO_CONTENT);
  let listed = common::body_json(
    app
      .authorized("GET", "/api/v1/orgs/acme/projects", "bob")
      .await,
  )
  .await;
  assert_eq!(
    listed["projects"],
    json!([{"address": "acme/demo", "slug": "demo", "name": "Demo", "role": "editor"}])
  );
  let project = common::body_json(
    app
      .authorized("GET", "/api/v1/projects/acme/demo", "bob")
      .await,
  )
  .await;
  assert_eq!(project["role"], "editor");
  assert_eq!(project["repository_url"], serde_json::Value::Null);
  let members = common::body_json(
    app
      .authorized("GET", "/api/v1/projects/acme/demo/members", "bob")
      .await,
  )
  .await;
  assert_eq!(
    members["members"],
    json!([{"handle": "bob", "role": "editor"}])
  );
  let removed = app
    .authorized("DELETE", "/api/v1/projects/acme/demo/members/bob", "alice")
    .await;
  assert_eq!(removed.status(), StatusCode::NO_CONTENT);
  assert_eq!(
    app
      .authorized("GET", "/api/v1/projects/acme/demo", "bob")
      .await
      .status(),
    StatusCode::NOT_FOUND
  );
}

#[tokio::test]
async fn repository_url_is_set_validated_and_cleared() {
  let app = setup().await;
  let set = app
    .json(
      "PUT",
      "/api/v1/projects/acme/demo/repository-url",
      "alice",
      json!({"url": "https://github.com/acme/demo"}),
    )
    .await;
  assert_eq!(set.status(), StatusCode::NO_CONTENT);
  let project = common::body_json(
    app
      .authorized("GET", "/api/v1/projects/acme/demo", "alice")
      .await,
  )
  .await;
  assert_eq!(project["repository_url"], "https://github.com/acme/demo");
  let bad = app
    .json(
      "PUT",
      "/api/v1/projects/acme/demo/repository-url",
      "alice",
      json!({"url": "ftp://x"}),
    )
    .await;
  assert_eq!(bad.status(), StatusCode::UNPROCESSABLE_ENTITY);
  let cleared = app
    .authorized(
      "DELETE",
      "/api/v1/projects/acme/demo/repository-url",
      "alice",
    )
    .await;
  assert_eq!(cleared.status(), StatusCode::NO_CONTENT);
  let project = common::body_json(
    app
      .authorized("GET", "/api/v1/projects/acme/demo", "alice")
      .await,
  )
  .await;
  assert_eq!(project["repository_url"], serde_json::Value::Null);
}

#[tokio::test]
async fn other_tenants_get_not_found_and_operators_read_but_cannot_write() {
  let app = setup().await;
  app.organization("carol", "beta").await;
  assert_eq!(
    app
      .authorized("GET", "/api/v1/projects/acme/demo", "carol")
      .await
      .status(),
    StatusCode::NOT_FOUND
  );
  assert_eq!(
    app
      .authorized("GET", "/api/v1/projects/acme/nothing", "carol")
      .await
      .status(),
    StatusCode::NOT_FOUND
  );
  assert_eq!(
    app
      .json(
        "POST",
        "/api/v1/orgs/acme/projects",
        "carol",
        json!({"slug": "x", "name": "X"})
      )
      .await
      .status(),
    StatusCode::NOT_FOUND
  );
  assert_eq!(
    app
      .authorized("GET", "/api/v1/projects/acme/demo", "ops")
      .await
      .status(),
    StatusCode::OK
  );
  assert_eq!(
    app
      .json(
        "PUT",
        "/api/v1/projects/acme/demo/repository-url",
        "ops",
        json!({"url": "https://x.example/r"})
      )
      .await
      .status(),
    StatusCode::FORBIDDEN
  );
  assert_eq!(
    app
      .json(
        "POST",
        "/api/v1/orgs/acme/projects",
        "ops",
        json!({"slug": "x", "name": "X"})
      )
      .await
      .status(),
    StatusCode::FORBIDDEN
  );
  assert_eq!(
    app
      .json(
        "POST",
        "/api/v1/orgs/acme/projects",
        "bob",
        json!({"slug": "x", "name": "X"})
      )
      .await
      .status(),
    StatusCode::FORBIDDEN
  );
}
