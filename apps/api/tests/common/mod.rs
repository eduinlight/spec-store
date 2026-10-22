#![allow(dead_code)]

use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, Response};
use http_body_util::BodyExt;
use serde_json::Value;
use spec_store_api::{ApiConfig, AppState, router};
use spec_store_core::identity::{IdentityConfig, Verifier};
use spec_store_core::keycloak::{AdminConfig, KeycloakAdmin};
use spec_store_core::store::Store;
use spec_store_testkit::keycloak::{ADMIN_CLIENT_ID, ADMIN_CLIENT_SECRET, REALM};
use spec_store_testkit::{FakeKeycloak, TestDb, TokenSpec};
use tower::ServiceExt;

pub const API_AUDIENCE: &str = "spec-store-api";

pub fn migrate(url: &str) {
  spec_store_core::db::open_with_size(url, 1).expect("migrate template");
}

pub struct TestApp {
  pub router: Router,
  pub state: AppState,
  pub keycloak: FakeKeycloak,
  pub db: Option<TestDb>,
}

pub fn config() -> ApiConfig {
  ApiConfig {
    allowed_hosts: vec!["api.example.test".into(), "app.example.test".into()],
    audience: API_AUDIENCE.into(),
  }
}

pub async fn app_with(config: ApiConfig, verifier: Option<Verifier>) -> TestApp {
  let keycloak = FakeKeycloak::start().await;
  let db = TestDb::new(migrate);
  let store = Store::open_with_size(&db.url, 4).expect("store");
  let verifier = verifier.unwrap_or_else(|| {
    Verifier::new(IdentityConfig {
      issuer: keycloak.issuer(),
      jwks_url: keycloak.jwks_url(),
    })
  });
  let admin = KeycloakAdmin::new(AdminConfig {
    base_url: keycloak.base_url.clone(),
    realm: REALM.into(),
    client_id: ADMIN_CLIENT_ID.into(),
    client_secret: ADMIN_CLIENT_SECRET.into(),
  });
  let state = AppState::new(store, Arc::new(verifier), config).with_keycloak_admin(Arc::new(admin));
  TestApp {
    router: router(state.clone()),
    state,
    keycloak,
    db: Some(db),
  }
}

pub async fn app() -> TestApp {
  app_with(config(), None).await
}

impl TestApp {
  pub fn token(&self, username: &str) -> String {
    self.keycloak.mint(&TokenSpec::new(username, API_AUDIENCE))
  }

  pub async fn send(&self, request: Request<Body>) -> Response<Body> {
    self
      .router
      .clone()
      .oneshot(request)
      .await
      .expect("response")
  }

  pub async fn authorized(&self, method: &str, path: &str, username: &str) -> Response<Body> {
    self
      .send(
        Request::builder()
          .method(method)
          .uri(path)
          .header("host", "api.example.test")
          .header("authorization", format!("Bearer {}", self.token(username)))
          .body(Body::empty())
          .unwrap(),
      )
      .await
  }

  pub async fn get(&self, path: &str) -> Response<Body> {
    self
      .send(
        Request::get(path)
          .header("host", "api.example.test")
          .body(Body::empty())
          .unwrap(),
      )
      .await
  }
}

pub async fn body_text(response: Response<Body>) -> String {
  let bytes = response
    .into_body()
    .collect()
    .await
    .expect("body")
    .to_bytes();
  String::from_utf8(bytes.to_vec()).expect("utf-8")
}

pub async fn body_json(response: Response<Body>) -> Value {
  serde_json::from_str(&body_text(response).await).expect("json body")
}
