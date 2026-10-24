mod common;

use std::net::SocketAddr;

use axum::body::Body;
use axum::extract::ConnectInfo;
use axum::http::Request;
use sdd_vault_api::ApiConfig;
use sdd_vault_core::model::AuditResult;
use sdd_vault_core::store::AuditEntry;
use sdd_vault_testkit::TokenSpec;
use serde_json::json;

fn creating_organization(token: &str, slug: &str, forwarded_for: Option<&str>) -> Request<Body> {
  let mut request = Request::builder()
    .method("POST")
    .uri("/api/v1/orgs")
    .header("host", "api.example.test")
    .header("authorization", format!("Bearer {token}"))
    .header("content-type", "application/json");
  if let Some(forwarded_for) = forwarded_for {
    request = request.header("x-forwarded-for", forwarded_for);
  }
  request
    .body(Body::from(json!({"slug": slug, "name": slug}).to_string()))
    .unwrap()
}

fn from_peer(mut request: Request<Body>, peer: &str) -> Request<Body> {
  let address: SocketAddr = peer.parse().unwrap();
  request.extensions_mut().insert(ConnectInfo(address));
  request
}

fn first_entry(app: &common::TestApp) -> AuditEntry {
  app
    .state
    .store
    .list_audit(None, 1)
    .unwrap()
    .into_iter()
    .next()
    .unwrap()
}

fn token_for(app: &common::TestApp, client_id: &str) -> String {
  let mut spec = TokenSpec::new("alice", common::API_AUDIENCE);
  spec.client_id = client_id.to_string();
  app.keycloak.mint(&spec)
}

#[tokio::test]
async fn the_origin_is_the_socket_peer_when_no_proxy_is_trusted() {
  let app = common::app().await;
  let response = app
    .send(from_peer(
      creating_organization(&app.token("alice"), "acme", Some("203.0.113.7")),
      "192.0.2.10:51234",
    ))
    .await;
  assert_eq!(response.status(), axum::http::StatusCode::CREATED);
  let entry = first_entry(&app);
  assert_eq!(entry.origin.as_deref(), Some("192.0.2.10"));
  assert_eq!(entry.result, AuditResult::Ok);
}

#[tokio::test]
async fn the_origin_is_unknown_without_a_socket_peer_or_trusted_header() {
  let app = common::app().await;
  app.organization("alice", "acme").await;
  let entry = first_entry(&app);
  assert_eq!(entry.origin, None);
}

#[tokio::test]
async fn trusted_hops_pick_the_forwarded_entry_from_the_right() {
  let config = ApiConfig {
    trusted_proxy_hops: 2,
    ..common::config()
  };
  let app = common::app_with(config, None).await;
  let forged = "198.51.100.99, 203.0.113.7, 10.8.0.1";
  let response = app
    .send(from_peer(
      creating_organization(&app.token("alice"), "acme", Some(forged)),
      "10.8.0.2:443",
    ))
    .await;
  assert_eq!(response.status(), axum::http::StatusCode::CREATED);
  let entry = first_entry(&app);
  assert_eq!(entry.origin.as_deref(), Some("203.0.113.7"));
}

#[tokio::test]
async fn trusted_hops_without_a_header_fall_back_to_the_peer() {
  let config = ApiConfig {
    trusted_proxy_hops: 2,
    ..common::config()
  };
  let app = common::app_with(config, None).await;
  let response = app
    .send(from_peer(
      creating_organization(&app.token("alice"), "acme", None),
      "10.8.0.2:443",
    ))
    .await;
  assert_eq!(response.status(), axum::http::StatusCode::CREATED);
  assert_eq!(first_entry(&app).origin.as_deref(), Some("10.8.0.2"));
}

#[tokio::test]
async fn the_cli_client_is_named_by_its_client_id() {
  let app = common::app().await;
  let token = token_for(&app, "sdd-vault-cli");
  let response = app.send(creating_organization(&token, "acme", None)).await;
  assert_eq!(response.status(), axum::http::StatusCode::CREATED);
  assert_eq!(first_entry(&app).client.as_deref(), Some("sddvault CLI"));
}

#[tokio::test]
async fn another_bearer_client_is_named_by_its_registered_name_or_api_client() {
  let app = common::app().await;
  app
    .keycloak
    .grant_consent("subject-alice", "registered-tool", "Registered Tool");
  let named = token_for(&app, "registered-tool");
  let response = app.send(creating_organization(&named, "acme", None)).await;
  assert_eq!(response.status(), axum::http::StatusCode::CREATED);
  assert_eq!(first_entry(&app).client.as_deref(), Some("Registered Tool"));

  let anonymous = token_for(&app, "unregistered-tool");
  let response = app
    .send(creating_organization(&anonymous, "beta", None))
    .await;
  assert_eq!(response.status(), axum::http::StatusCode::CREATED);
  assert_eq!(first_entry(&app).client.as_deref(), Some("API client"));
}

#[tokio::test]
async fn browser_sessions_are_the_web_app() {
  let app = common::app().await;
  let cookie = app.browser_sign_in("alice").await;
  let response = app
    .browser(
      "POST",
      "/api/v1/orgs",
      &cookie,
      true,
      Some(json!({"slug": "acme", "name": "acme"})),
    )
    .await;
  assert_eq!(response.status(), axum::http::StatusCode::CREATED);
  assert_eq!(first_entry(&app).client.as_deref(), Some("Web app"));
}
