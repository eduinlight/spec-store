mod common;

use std::time::Duration;

use axum::http::StatusCode;
use sdd_vault_api::outbox::Sender;
use sdd_vault_testkit::mailpit;
use serde_json::{Value, json};

async fn org(app: &common::TestApp) {
  let created = app
    .json(
      "POST",
      "/api/v1/orgs",
      "alice",
      json!({"slug": "acme", "name": "Acme Labs"}),
    )
    .await;
  assert_eq!(created.status(), StatusCode::CREATED);
}

fn accept_token(message: &mailpit::Message) -> String {
  let marker = format!("{}/invitations/", common::APP_URL);
  let start = message
    .text
    .find(&marker)
    .expect("accept link in the text body")
    + marker.len();
  message.text[start..]
    .chars()
    .take_while(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
    .collect()
}

#[tokio::test]
async fn an_invitation_email_names_the_organization_and_inviter_and_links_to_acceptance() {
  let app = common::app().await;
  org(&app).await;
  let address = mailpit::unique_address("invitee");
  let created = app
    .json(
      "POST",
      "/api/v1/orgs/acme/invitations",
      "alice",
      json!({"email": address, "language": "es"}),
    )
    .await;
  assert_eq!(created.status(), StatusCode::CREATED);
  let body = common::body_json(created).await;
  assert_eq!(body["email"], address.as_str());
  assert!(
    body.get("token").is_none(),
    "the token only travels by email"
  );

  let sender = Sender::new(app.state.store.clone(), common::smtp());
  assert_eq!(sender.run_once().await, 1);
  let messages = mailpit::wait_for_messages(&address, 1, Duration::from_secs(10)).await;
  assert_eq!(messages.len(), 1);
  let message = &messages[0];
  assert!(message.subject.contains("Acme Labs"), "{}", message.subject);
  assert!(
    message.subject.contains("Te invitaron") || message.subject.contains("invit"),
    "{}",
    message.subject
  );
  assert!(message.text.contains("alice"), "{}", message.text);
  assert!(message.html.contains("Acme Labs"));

  let token = accept_token(message);
  let invitee_token = app.token_with_email("invitee", &address);
  let preview = app
    .json_with_token(
      "GET",
      &format!("/api/v1/invitations/{token}"),
      &invitee_token,
      Value::Null,
    )
    .await;
  assert_eq!(common::body_json(preview).await["organization"], "acme");
  let accepted = app
    .json_with_token(
      "POST",
      &format!("/api/v1/invitations/{token}/accept"),
      &invitee_token,
      Value::Null,
    )
    .await;
  assert_eq!(accepted.status(), StatusCode::NO_CONTENT);
  let orgs = app
    .json_with_token("GET", "/api/v1/orgs", &invitee_token, Value::Null)
    .await;
  assert_eq!(
    common::body_json(orgs).await["organizations"][0]["role"],
    "member"
  );
}

#[tokio::test]
async fn owners_list_resend_and_revoke_invitations() {
  let app = common::app().await;
  org(&app).await;
  let address = mailpit::unique_address("resend");
  let created = common::body_json(
    app
      .json(
        "POST",
        "/api/v1/orgs/acme/invitations",
        "alice",
        json!({"email": address}),
      )
      .await,
  )
  .await;
  let id = created["id"].as_i64().unwrap();
  let listed = common::body_json(
    app
      .authorized("GET", "/api/v1/orgs/acme/invitations", "alice")
      .await,
  )
  .await;
  assert_eq!(listed["invitations"][0]["state"], "pending");
  assert_eq!(listed["invitations"][0]["inviter"], "alice");

  let resent = app
    .authorized(
      "POST",
      &format!("/api/v1/orgs/acme/invitations/{id}/resend"),
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
    .authorized(
      "DELETE",
      &format!("/api/v1/orgs/acme/invitations/{id}"),
      "alice",
    )
    .await;
  assert_eq!(revoked.status(), StatusCode::NO_CONTENT);
  let listed = common::body_json(
    app
      .authorized("GET", "/api/v1/orgs/acme/invitations", "alice")
      .await,
  )
  .await;
  assert_eq!(listed["invitations"][0]["state"], "revoked");
  assert_eq!(
    app
      .json(
        "POST",
        "/api/v1/orgs/acme/invitations",
        "bob",
        json!({"email": "x@example.com"})
      )
      .await
      .status(),
    StatusCode::NOT_FOUND
  );
}

#[tokio::test]
async fn pending_invitations_are_listed_for_the_invited_person_and_can_be_declined() {
  let app = common::app().await;
  org(&app).await;
  let address = mailpit::unique_address("pending");
  app
    .json(
      "POST",
      "/api/v1/orgs/acme/invitations",
      "alice",
      json!({"email": address}),
    )
    .await;
  let invitee = app.token_with_email("invitee", &address);
  let mine = common::body_json(
    app
      .json_with_token("GET", "/api/v1/invitations", &invitee, Value::Null)
      .await,
  )
  .await;
  let invitation = &mine["invitations"][0];
  assert_eq!(invitation["organization"], "acme");
  let id = invitation["id"].as_i64().unwrap();
  let declined = app
    .json_with_token(
      "POST",
      &format!("/api/v1/me/invitations/{id}/decline"),
      &invitee,
      Value::Null,
    )
    .await;
  assert_eq!(declined.status(), StatusCode::NO_CONTENT);
  let mine = common::body_json(
    app
      .json_with_token("GET", "/api/v1/invitations", &invitee, Value::Null)
      .await,
  )
  .await;
  assert!(mine["invitations"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn an_invitation_is_recorded_while_mail_is_down_and_sent_when_it_returns() {
  let app = common::app().await;
  org(&app).await;
  let address = mailpit::unique_address("outage");
  let created = app
    .json(
      "POST",
      "/api/v1/orgs/acme/invitations",
      "alice",
      json!({"email": address}),
    )
    .await;
  assert_eq!(created.status(), StatusCode::CREATED);
  let mut down = common::smtp();
  down.port = 1;
  let broken = Sender::new(app.state.store.clone(), down);
  assert_eq!(broken.run_once().await, 0);
  assert!(mailpit::messages_to(&address).await.is_empty());
  app
    .state
    .store
    .with_conn(|conn| {
      use diesel::RunQueryDsl;
      diesel::sql_query("UPDATE email_outbox SET next_attempt_at = now()").execute(conn)?;
      Ok(())
    })
    .unwrap();
  let working = Sender::new(app.state.store.clone(), common::smtp());
  assert_eq!(working.run_once().await, 1);
  assert_eq!(
    mailpit::wait_for_messages(&address, 1, Duration::from_secs(10))
      .await
      .len(),
    1
  );
}
