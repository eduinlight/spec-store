mod common;

use axum::http::StatusCode;

#[tokio::test]
async fn lists_only_the_callers_authorized_clients() {
  let app = common::app().await;
  app
    .keycloak
    .grant_consent("subject-alice", "claude-code-1", "Claude Code");
  app
    .keycloak
    .grant_consent("subject-bob", "other-agent", "Other agent");
  let response = app.authorized("GET", "/api/v1/me/clients", "alice").await;
  assert_eq!(response.status(), StatusCode::OK);
  let body = common::body_json(response).await;
  let clients = body["clients"].as_array().expect("clients");
  assert_eq!(clients.len(), 1);
  assert_eq!(clients[0]["client_id"], "claude-code-1");
  assert_eq!(clients[0]["name"], "Claude Code");
  assert!(
    clients[0]["scopes"]
      .as_array()
      .unwrap()
      .iter()
      .any(|s| s == "sdd-vault-mcp")
  );
  assert!(clients[0]["granted_at"].as_str().unwrap().ends_with('Z'));
}

#[tokio::test]
async fn revoking_removes_the_grant_in_the_identity_provider() {
  let app = common::app().await;
  app
    .keycloak
    .grant_consent("subject-alice", "claude-code-1", "Claude Code");
  app
    .keycloak
    .grant_consent("subject-alice", "cursor-2", "Cursor");
  let response = app
    .authorized("DELETE", "/api/v1/me/clients/claude-code-1", "alice")
    .await;
  assert_eq!(response.status(), StatusCode::NO_CONTENT);
  assert_eq!(
    app.keycloak.consent_client_ids("subject-alice"),
    vec!["cursor-2"]
  );
  let listed = common::body_json(app.authorized("GET", "/api/v1/me/clients", "alice").await).await;
  assert_eq!(listed["clients"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn revoking_an_unknown_client_is_not_found() {
  let app = common::app().await;
  let response = app
    .authorized("DELETE", "/api/v1/me/clients/nothing", "alice")
    .await;
  assert_eq!(response.status(), StatusCode::NOT_FOUND);
  assert_eq!(common::body_json(response).await["code"], "not_found");
}

#[tokio::test]
async fn listing_requires_authentication() {
  let app = common::app().await;
  assert_eq!(
    app.get("/api/v1/me/clients").await.status(),
    StatusCode::UNAUTHORIZED
  );
}
