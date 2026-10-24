mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use sdd_vault_api::ApiConfig;

async fn status_for(app: &common::TestApp, host: &str, path: &str) -> StatusCode {
  app
    .send(
      Request::get(path)
        .header("host", host)
        .body(Body::empty())
        .unwrap(),
    )
    .await
    .status()
}

#[tokio::test]
async fn configured_hosts_are_accepted_and_others_refused() {
  let app = common::app().await;
  let token = app.token("alice");
  let authorized = |host: &str| {
    Request::get("/api/v1/me")
      .header("host", host)
      .header("authorization", format!("Bearer {token}"))
      .body(Body::empty())
      .unwrap()
  };
  assert_ne!(
    app.send(authorized("api.example.test")).await.status(),
    StatusCode::MISDIRECTED_REQUEST
  );
  assert_ne!(
    app.send(authorized("app.example.test")).await.status(),
    StatusCode::MISDIRECTED_REQUEST
  );
  assert_eq!(
    app.send(authorized("evil.example.test")).await.status(),
    StatusCode::MISDIRECTED_REQUEST
  );
}

#[tokio::test]
async fn health_routes_answer_whatever_the_host() {
  let app = common::app().await;
  assert_eq!(
    status_for(&app, "10.42.0.7:4000", "/healthz").await,
    StatusCode::OK
  );
  assert_eq!(
    status_for(&app, "10.42.0.7:4000", "/readyz").await,
    StatusCode::OK
  );
}

#[tokio::test]
async fn without_configured_hosts_only_loopback_is_accepted() {
  let config = ApiConfig {
    allowed_hosts: Vec::new(),
    ..common::config()
  };
  let app = common::app_with(config, None).await;
  assert_ne!(
    status_for(&app, "localhost:4000", "/api/v1/me").await,
    StatusCode::MISDIRECTED_REQUEST
  );
  assert_ne!(
    status_for(&app, "127.0.0.1:4000", "/api/v1/me").await,
    StatusCode::MISDIRECTED_REQUEST
  );
  assert_eq!(
    status_for(&app, "api.example.test", "/api/v1/me").await,
    StatusCode::MISDIRECTED_REQUEST
  );
}
