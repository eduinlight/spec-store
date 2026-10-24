mod common;

use axum::http::StatusCode;
use sdd_vault_core::model::Role;
use sdd_vault_core::store::invitations::ProjectInvitation;
use serde_json::json;

async fn app_with_guest() -> common::TestApp {
  let app = common::app().await;
  app.organization("alice", "acme").await;
  app.admit("acme", "bob");
  for slug in ["demo", "other"] {
    let created = app
      .json(
        "POST",
        "/api/v1/orgs/acme/projects",
        "alice",
        json!({"slug": slug, "name": slug}),
      )
      .await;
    assert_eq!(created.status(), StatusCode::CREATED);
  }
  let alice = app.person("alice");
  let guest = app.person("gwen");
  let invited = app
    .state
    .store
    .invite_to_project(
      &alice,
      "acme/demo",
      "gwen@example.com",
      Role::Reader,
      "en",
      "https://app.example.test/invitations",
    )
    .unwrap();
  let ProjectInvitation::Invited(invited) = invited else {
    panic!("expected an invitation");
  };
  app
    .state
    .store
    .accept_invitation(&guest, &invited.token, true)
    .unwrap();
  app
}

#[tokio::test]
async fn organization_lists_show_a_guest_the_name_and_slug_with_no_role() {
  let app = app_with_guest().await;
  let listed = common::body_json(app.authorized("GET", "/api/v1/orgs", "gwen").await).await;
  assert_eq!(
    listed["organizations"],
    json!([{"slug": "acme", "name": "acme", "role": null, "guest": true}])
  );
  let member = common::body_json(app.authorized("GET", "/api/v1/orgs", "bob").await).await;
  assert_eq!(
    member["organizations"],
    json!([{"slug": "acme", "name": "acme", "role": "member", "guest": false}])
  );
  let read = common::body_json(app.authorized("GET", "/api/v1/orgs/acme", "gwen").await).await;
  assert_eq!(read["slug"], "acme");
  assert_eq!(read["role"], serde_json::Value::Null);
  assert_eq!(read["guest"], true);
}

#[tokio::test]
async fn a_guest_lists_only_their_projects_and_nothing_else_of_the_organization() {
  let app = app_with_guest().await;
  let projects = common::body_json(
    app
      .authorized("GET", "/api/v1/orgs/acme/projects", "gwen")
      .await,
  )
  .await;
  let listed = projects["projects"].as_array().unwrap();
  assert_eq!(listed.len(), 1);
  assert_eq!(listed[0]["address"], "acme/demo");
  assert_eq!(listed[0]["role"], "reader");
  for path in [
    "/api/v1/orgs/acme/members",
    "/api/v1/orgs/acme/invitations",
    "/api/v1/orgs/acme/audit",
    "/api/v1/projects/acme/other",
    "/api/v1/projects/acme/other/members",
  ] {
    let response = app.authorized("GET", path, "gwen").await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND, "{path}");
  }
  let own = app
    .authorized("GET", "/api/v1/projects/acme/demo", "gwen")
    .await;
  assert_eq!(own.status(), StatusCode::OK);
  let create = app
    .json(
      "POST",
      "/api/v1/orgs/acme/projects",
      "gwen",
      json!({"slug": "mine", "name": "Mine"}),
    )
    .await;
  assert_eq!(create.status(), StatusCode::NOT_FOUND);
}
