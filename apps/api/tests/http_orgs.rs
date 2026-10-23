mod common;

use axum::http::StatusCode;
use serde_json::json;

#[tokio::test]
async fn people_create_list_and_read_their_organizations() {
  let app = common::app().await;
  let created = app
    .json(
      "POST",
      "/api/v1/orgs",
      "alice",
      json!({"slug": "acme", "name": "Acme"}),
    )
    .await;
  assert_eq!(created.status(), StatusCode::CREATED);
  let body = common::body_json(created).await;
  assert_eq!(body["slug"], "acme");
  assert_eq!(body["role"], "owner");

  let listed = common::body_json(app.authorized("GET", "/api/v1/orgs", "alice").await).await;
  assert_eq!(
    listed["organizations"],
    json!([{"slug": "acme", "name": "Acme", "role": "owner"}])
  );
  let read = common::body_json(app.authorized("GET", "/api/v1/orgs/acme", "alice").await).await;
  assert_eq!(read["name"], "Acme");

  let duplicate = app
    .json(
      "POST",
      "/api/v1/orgs",
      "bob",
      json!({"slug": "acme", "name": "Again"}),
    )
    .await;
  assert_eq!(duplicate.status(), StatusCode::UNPROCESSABLE_ENTITY);
  assert_eq!(common::body_json(duplicate).await["code"], "validation");
}

#[tokio::test]
async fn outsiders_get_not_found_and_operators_read() {
  let app = common::app().await;
  app
    .json(
      "POST",
      "/api/v1/orgs",
      "alice",
      json!({"slug": "acme", "name": "Acme"}),
    )
    .await;
  assert_eq!(
    app
      .authorized("GET", "/api/v1/orgs/acme", "bob")
      .await
      .status(),
    StatusCode::NOT_FOUND
  );
  assert_eq!(
    app
      .authorized("GET", "/api/v1/orgs/nothing", "bob")
      .await
      .status(),
    StatusCode::NOT_FOUND
  );
  assert_eq!(
    app
      .authorized("GET", "/api/v1/orgs/acme", "ops")
      .await
      .status(),
    StatusCode::OK
  );
  let members = common::body_json(
    app
      .authorized("GET", "/api/v1/orgs/acme/members", "ops")
      .await,
  )
  .await;
  assert_eq!(members["members"][0]["handle"], "alice");
}

#[tokio::test]
async fn owners_change_roles_and_remove_members_but_never_the_last_owner() {
  let app = common::app().await;
  app
    .json(
      "POST",
      "/api/v1/orgs",
      "alice",
      json!({"slug": "acme", "name": "Acme"}),
    )
    .await;
  let bob = app
    .state
    .store
    .caller_for(
      &spec_store_core::store::users::Identity {
        subject: "subject-bob".into(),
        handle: "bob".into(),
        email: "bob@example.com".into(),
        name: "bob".into(),
      },
      false,
    )
    .unwrap();
  app
    .state
    .store
    .admit_organization_member("acme", bob.user_id, spec_store_core::model::OrgRole::Member)
    .unwrap();

  let promote = app
    .json(
      "PUT",
      "/api/v1/orgs/acme/members/bob",
      "alice",
      json!({"role": "owner"}),
    )
    .await;
  assert_eq!(promote.status(), StatusCode::NO_CONTENT);
  let members = common::body_json(
    app
      .authorized("GET", "/api/v1/orgs/acme/members", "alice")
      .await,
  )
  .await;
  assert!(
    members["members"]
      .as_array()
      .unwrap()
      .iter()
      .all(|m| m["role"] == "owner")
  );

  let demote_self = app
    .json(
      "PUT",
      "/api/v1/orgs/acme/members/alice",
      "alice",
      json!({"role": "member"}),
    )
    .await;
  assert_eq!(demote_self.status(), StatusCode::NO_CONTENT);
  let last = app
    .json(
      "PUT",
      "/api/v1/orgs/acme/members/bob",
      "bob",
      json!({"role": "member"}),
    )
    .await;
  assert_eq!(last.status(), StatusCode::UNPROCESSABLE_ENTITY);
  assert!(
    common::body_json(last).await["message"]
      .as_str()
      .unwrap()
      .contains("without an owner")
  );

  let removed = app
    .authorized("DELETE", "/api/v1/orgs/acme/members/alice", "bob")
    .await;
  assert_eq!(removed.status(), StatusCode::NO_CONTENT);
  assert_eq!(
    app
      .authorized("GET", "/api/v1/orgs/acme", "alice")
      .await
      .status(),
    StatusCode::NOT_FOUND
  );
  let invalid = app
    .json(
      "PUT",
      "/api/v1/orgs/acme/members/bob",
      "bob",
      json!({"role": "admin"}),
    )
    .await;
  assert_eq!(invalid.status(), StatusCode::UNPROCESSABLE_ENTITY);
}
