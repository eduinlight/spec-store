mod common;

use std::time::Duration;

use axum::http::StatusCode;
use sdd_vault_api::outbox::Sender;
use sdd_vault_testkit::mailpit;
use serde_json::{Value, json};

const MEMBERS: &str = "/api/v1/projects/acme/demo/members";

async fn setup() -> common::TestApp {
  let app = common::app().await;
  app.organization("alice", "acme").await;
  for handle in ["bob", "erin", "rita"] {
    app.admit("acme", handle);
  }
  let created = app
    .json(
      "POST",
      "/api/v1/orgs/acme/projects",
      "alice",
      json!({"slug": "demo", "name": "Demo Project"}),
    )
    .await;
  assert_eq!(created.status(), StatusCode::CREATED);
  for (handle, role) in [("alice", "owner"), ("erin", "editor"), ("rita", "reader")] {
    let response = app
      .json(
        "PUT",
        &format!("{MEMBERS}/{handle}"),
        "alice",
        json!({"role": role}),
      )
      .await;
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
  }
  app
}

fn accept_token(message: &mailpit::Message) -> String {
  let marker = format!("{}/invitations/", common::APP_URL);
  let start = message.text.find(&marker).expect("accept link") + marker.len();
  message.text[start..]
    .chars()
    .take_while(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
    .collect()
}

async fn code(response: axum::http::Response<axum::body::Body>) -> (StatusCode, Value) {
  let status = response.status();
  (status, common::body_json(response).await)
}

#[tokio::test]
async fn members_carry_name_role_guest_flag_joined_and_last_active() {
  let app = setup().await;
  let (status, body) = code(app.authorized("GET", MEMBERS, "rita").await).await;
  assert_eq!(status, StatusCode::OK);
  let handles: Vec<&str> = body["members"]
    .as_array()
    .unwrap()
    .iter()
    .map(|m| m["handle"].as_str().unwrap())
    .collect();
  assert_eq!(handles, vec!["alice", "erin", "rita"]);
  let alice = &body["members"][0];
  assert_eq!(alice["name"], "alice");
  assert_eq!(alice["role"], "owner");
  assert_eq!(alice["guest"], false);
  assert!(alice["joined_at"].as_str().unwrap().ends_with('Z'));
  assert!(alice["last_active_at"].as_str().unwrap().ends_with('Z'));
  assert_eq!(body["members"][2]["last_active_at"], Value::Null);
  let (status, _) = code(app.authorized("GET", MEMBERS, "bob").await).await;
  assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn an_organization_member_is_added_at_once_and_a_role_can_change_and_end() {
  let app = setup().await;
  let (status, body) = code(
    app
      .json(
        "POST",
        &format!("{MEMBERS}/invitations"),
        "alice",
        json!({"email": "bob@example.com", "role": "editor"}),
      )
      .await,
  )
  .await;
  assert_eq!(status, StatusCode::OK);
  assert_eq!(body, json!({"added": "bob"}));
  let changed = app
    .json(
      "PUT",
      &format!("{MEMBERS}/bob"),
      "alice",
      json!({"role": "reader"}),
    )
    .await;
  assert_eq!(changed.status(), StatusCode::NO_CONTENT);
  let (_, body) = code(app.authorized("GET", MEMBERS, "alice").await).await;
  let bob = body["members"]
    .as_array()
    .unwrap()
    .iter()
    .find(|m| m["handle"] == "bob")
    .unwrap();
  assert_eq!(bob["role"], "reader");
  let removed = app
    .authorized("DELETE", &format!("{MEMBERS}/bob"), "alice")
    .await;
  assert_eq!(removed.status(), StatusCode::NO_CONTENT);
  let (status, _) = code(app.authorized("GET", MEMBERS, "bob").await).await;
  assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn an_outside_email_is_emailed_and_joins_as_a_guest_through_the_link() {
  let app = setup().await;
  let address = mailpit::unique_address("guest");
  let (status, body) = code(
    app
      .json(
        "POST",
        &format!("{MEMBERS}/invitations"),
        "alice",
        json!({"email": address, "role": "reader", "language": "es"}),
      )
      .await,
  )
  .await;
  assert_eq!(status, StatusCode::CREATED);
  assert_eq!(body["invited"], address.as_str());
  assert!(body["id"].as_i64().is_some());
  assert!(body.get("added").is_none());
  assert!(body.get("token").is_none());

  let sender = Sender::new(app.state.store.clone(), common::smtp());
  assert_eq!(sender.run_once().await, 1);
  let messages = mailpit::wait_for_messages(&address, 1, Duration::from_secs(10)).await;
  let message = &messages[0];
  assert!(
    message.subject.contains("Demo Project"),
    "{}",
    message.subject
  );
  assert!(
    message.subject.contains("Te invitaron"),
    "{}",
    message.subject
  );
  assert!(message.text.contains("alice"), "{}", message.text);
  assert!(message.text.contains("lector"), "{}", message.text);

  let token = accept_token(message);
  let guest_token = app.token_with_email("gwen", &address);
  let preview = common::body_json(
    app
      .json_with_token(
        "GET",
        &format!("/api/v1/invitations/{token}"),
        &guest_token,
        Value::Null,
      )
      .await,
  )
  .await;
  assert_eq!(preview["organization"], "acme");
  assert_eq!(preview["project"], "acme/demo");
  assert_eq!(preview["project_name"], "Demo Project");
  assert_eq!(preview["project_role"], "reader");
  let accepted = app
    .json_with_token(
      "POST",
      &format!("/api/v1/invitations/{token}/accept"),
      &guest_token,
      Value::Null,
    )
    .await;
  assert_eq!(accepted.status(), StatusCode::NO_CONTENT);

  let orgs = common::body_json(
    app
      .json_with_token("GET", "/api/v1/orgs", &guest_token, Value::Null)
      .await,
  )
  .await;
  assert_eq!(
    orgs["organizations"],
    json!([{"slug": "acme", "name": "acme", "role": null, "guest": true}])
  );
  let (_, members) = code(app.authorized("GET", MEMBERS, "alice").await).await;
  let guest = members["members"]
    .as_array()
    .unwrap()
    .iter()
    .find(|m| m["handle"] == "gwen")
    .unwrap();
  assert_eq!(guest["guest"], true);
  assert_eq!(guest["role"], "reader");

  let promoted = app
    .json(
      "PUT",
      &format!("{MEMBERS}/gwen"),
      "alice",
      json!({"role": "editor"}),
    )
    .await;
  assert_eq!(promoted.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn owners_list_resend_and_revoke_pending_project_invitations() {
  let app = setup().await;
  let address = mailpit::unique_address("pending");
  let (_, created) = code(
    app
      .json(
        "POST",
        &format!("{MEMBERS}/invitations"),
        "alice",
        json!({"email": address, "role": "editor"}),
      )
      .await,
  )
  .await;
  let id = created["id"].as_i64().unwrap();
  let (status, listed) = code(
    app
      .authorized("GET", &format!("{MEMBERS}/invitations"), "alice")
      .await,
  )
  .await;
  assert_eq!(status, StatusCode::OK);
  assert_eq!(listed["invitations"].as_array().unwrap().len(), 1);
  assert_eq!(listed["invitations"][0]["email"], address.as_str());
  assert_eq!(listed["invitations"][0]["role"], "editor");
  assert_eq!(listed["invitations"][0]["state"], "pending");
  assert_eq!(listed["invitations"][0]["inviter"], "alice");

  let resent = app
    .authorized(
      "POST",
      &format!("{MEMBERS}/invitations/{id}/resend"),
      "alice",
    )
    .await;
  assert_eq!(resent.status(), StatusCode::NO_CONTENT);
  let sender = Sender::new(app.state.store.clone(), common::smtp());
  assert_eq!(sender.run_once().await, 2);
  assert_eq!(
    mailpit::wait_for_messages(&address, 2, Duration::from_secs(10))
      .await
      .len(),
    2
  );

  let revoked = app
    .authorized("DELETE", &format!("{MEMBERS}/invitations/{id}"), "alice")
    .await;
  assert_eq!(revoked.status(), StatusCode::NO_CONTENT);
  let (_, listed) = code(
    app
      .authorized("GET", &format!("{MEMBERS}/invitations"), "alice")
      .await,
  )
  .await;
  assert_eq!(listed["invitations"][0]["state"], "revoked");
  let (status, _) = code(
    app
      .authorized("DELETE", &format!("{MEMBERS}/invitations/{id}"), "alice")
      .await,
  )
  .await;
  assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
  let (status, _) = code(
    app
      .authorized("DELETE", &format!("{MEMBERS}/invitations/9999"), "alice")
      .await,
  )
  .await;
  assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn only_project_owners_manage_members_and_invitations() {
  let app = setup().await;
  let (_, created) = code(
    app
      .json(
        "POST",
        &format!("{MEMBERS}/invitations"),
        "alice",
        json!({"email": "someone@example.org", "role": "reader"}),
      )
      .await,
  )
  .await;
  let id = created["id"].as_i64().unwrap();
  for user in ["erin", "rita"] {
    let invite = app
      .json(
        "POST",
        &format!("{MEMBERS}/invitations"),
        user,
        json!({"email": "x@example.org", "role": "reader"}),
      )
      .await;
    assert_eq!(invite.status(), StatusCode::FORBIDDEN, "{user}");
    for (method, path) in [
      ("GET", format!("{MEMBERS}/invitations")),
      ("POST", format!("{MEMBERS}/invitations/{id}/resend")),
      ("DELETE", format!("{MEMBERS}/invitations/{id}")),
      ("DELETE", format!("{MEMBERS}/alice")),
    ] {
      let response = app.authorized(method, &path, user).await;
      assert_eq!(
        response.status(),
        StatusCode::FORBIDDEN,
        "{user} {method} {path}"
      );
    }
    let change = app
      .json(
        "PUT",
        &format!("{MEMBERS}/rita"),
        user,
        json!({"role": "owner"}),
      )
      .await;
    assert_eq!(change.status(), StatusCode::FORBIDDEN, "{user}");
  }
  for (method, path) in [
    ("GET", format!("{MEMBERS}/invitations")),
    ("POST", format!("{MEMBERS}/invitations/{id}/resend")),
    ("DELETE", format!("{MEMBERS}/invitations/{id}")),
  ] {
    let response = app.authorized(method, &path, "bob").await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND, "{method} {path}");
  }
  let unauthenticated = app.get(&format!("{MEMBERS}/invitations")).await;
  assert_eq!(unauthenticated.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn invitations_are_validated_and_the_last_owner_stays() {
  let app = setup().await;
  for (body, cause) in [
    (json!({"email": "x@example.org", "role": "admin"}), "role"),
    (json!({"email": "nope", "role": "reader"}), "email"),
    (
      json!({"email": "erin@example.com", "role": "reader"}),
      "already a member",
    ),
    (
      json!({"email": "alice@example.com", "role": "reader"}),
      "owns the organization",
    ),
  ] {
    let (status, error) = code(
      app
        .json("POST", &format!("{MEMBERS}/invitations"), "alice", body)
        .await,
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{cause}");
    assert!(
      error["message"].as_str().unwrap().contains(cause),
      "{error}"
    );
  }
  let first = app
    .json(
      "POST",
      &format!("{MEMBERS}/invitations"),
      "alice",
      json!({"email": "again@example.org", "role": "reader"}),
    )
    .await;
  assert_eq!(first.status(), StatusCode::CREATED);
  let (status, error) = code(
    app
      .json(
        "POST",
        &format!("{MEMBERS}/invitations"),
        "alice",
        json!({"email": "again@example.org", "role": "reader"}),
      )
      .await,
  )
  .await;
  assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
  assert!(
    error["message"].as_str().unwrap().contains("pending"),
    "{error}"
  );

  let (status, error) = code(
    app
      .json(
        "PUT",
        &format!("{MEMBERS}/alice"),
        "alice",
        json!({"role": "editor"}),
      )
      .await,
  )
  .await;
  assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
  assert!(
    error["message"]
      .as_str()
      .unwrap()
      .contains("without an owner"),
    "{error}"
  );
  let (status, error) = code(
    app
      .authorized("DELETE", &format!("{MEMBERS}/alice"), "alice")
      .await,
  )
  .await;
  assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
  assert!(
    error["message"]
      .as_str()
      .unwrap()
      .contains("without an owner"),
    "{error}"
  );
}
