mod common;

use axum::http::StatusCode;
use sdd_vault_core::identity::{IdentityConfig, Verifier};

#[tokio::test]
async fn liveness_always_answers() {
  let app = common::app().await;
  let response = app.get("/healthz").await;
  assert_eq!(response.status(), StatusCode::OK);
  assert_eq!(common::body_text(response).await, "ok");
}

#[tokio::test]
async fn readiness_is_ok_when_database_and_identity_provider_answer() {
  let app = common::app().await;
  let response = app.get("/readyz").await;
  assert_eq!(response.status(), StatusCode::OK);
  let body = common::body_json(response).await;
  assert_eq!(body["database"], "ok");
  assert_eq!(body["identity_provider"], "ok");
}

#[tokio::test]
async fn readiness_fails_when_the_database_is_unreachable() {
  let mut app = common::app().await;
  drop(app.db.take());
  let response = app.get("/readyz").await;
  assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
  let body = common::body_json(response).await;
  assert_eq!(body["database"], "unavailable");
}

#[tokio::test]
async fn readiness_fails_when_the_identity_provider_is_unreachable() {
  let verifier = Verifier::new(IdentityConfig {
    issuer: "http://127.0.0.1:1/realms/sdd-vault".into(),
    jwks_url: "http://127.0.0.1:1/realms/sdd-vault/protocol/openid-connect/certs".into(),
  });
  let app = common::app_with(common::config(), Some(verifier)).await;
  let response = app.get("/readyz").await;
  assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
  let body = common::body_json(response).await;
  assert_eq!(body["identity_provider"], "unavailable");
  assert_eq!(body["database"], "ok");
}
