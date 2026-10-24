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
  let project = &listed["projects"][0];
  assert_eq!(listed["projects"].as_array().unwrap().len(), 1);
  assert_eq!(project["address"], "acme/demo");
  assert_eq!(project["slug"], "demo");
  assert_eq!(project["name"], "Demo");
  assert_eq!(project["role"], "editor");
  assert_eq!(project["archived"], false);
  assert_eq!(project["constitution_version"], serde_json::Value::Null);
  assert_eq!(project["constitution_updated_at"], serde_json::Value::Null);
  assert_eq!(project["active_spec"], serde_json::Value::Null);
  assert_eq!(project["open_markers"], 0);
  assert_eq!(project["last_activity"]["actor"], "alice");
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

#[tokio::test]
async fn the_list_carries_constitution_active_spec_tasks_and_markers() {
  let app = setup().await;
  let alice = app.person("alice");
  app
    .state
    .store
    .put_constitution(&alice, "acme/demo", "# Constitution", "2.0", "init", None)
    .unwrap();
  app
    .state
    .store
    .create_spec(&alice, "acme/demo", "first")
    .unwrap();
  app
    .state
    .store
    .put_document(
      &alice,
      "acme/demo",
      1,
      sdd_vault_core::model::DocumentKind::Tasks,
      "## Phase\n- [x] T01 (FR: —) One — Done when: ok · commit: abc1234\n- [ ] T02 (FR: —) Two — Done when: ok [NEEDS CLARIFICATION: x]\n",
      "tasks",
      None,
    )
    .unwrap();
  let listed = common::body_json(
    app
      .authorized("GET", "/api/v1/orgs/acme/projects", "alice")
      .await,
  )
  .await;
  let project = &listed["projects"][0];
  assert_eq!(project["constitution_version"], "2.0");
  assert!(
    project["constitution_updated_at"]
      .as_str()
      .unwrap()
      .ends_with('Z')
  );
  assert_eq!(
    project["active_spec"],
    json!({"number": 1, "slug": "first", "status": "draft", "tasks_done": 1, "tasks_total": 2})
  );
  assert_eq!(project["open_markers"], 1);
  assert!(
    project["last_activity"]["at"]
      .as_str()
      .unwrap()
      .ends_with('Z')
  );
}
