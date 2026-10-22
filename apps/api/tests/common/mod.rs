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

pub const APP_URL: &str = "https://app.example.test";

pub const LANDING_URL: &str = "https://example.test";
pub const SESSION_COOKIE: &str = "__Host-spec_store_session";

pub fn config() -> ApiConfig {
  ApiConfig {
    allowed_hosts: vec!["api.example.test".into(), "app.example.test".into()],
    audience: API_AUDIENCE.into(),
    app_url: APP_URL.into(),
    landing_url: LANDING_URL.into(),
    app_client_id: spec_store_testkit::keycloak::APP_CLIENT_ID.into(),
    app_client_secret: spec_store_testkit::keycloak::APP_CLIENT_SECRET.into(),
    session_key: spec_store_core::crypto::Sealer::generate_key(),
  }
}

pub fn decode(value: &str) -> String {
  let bytes = value.as_bytes();
  let mut out = Vec::new();
  let mut i = 0;
  while i < bytes.len() {
    match bytes[i] {
      b'%' if i + 2 < bytes.len() => {
        let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap();
        out.push(u8::from_str_radix(hex, 16).unwrap());
        i += 3;
      }
      b'+' => {
        out.push(b' ');
        i += 1;
      }
      b => {
        out.push(b);
        i += 1;
      }
    }
  }
  String::from_utf8(out).unwrap()
}

pub fn query_param(url: &str, name: &str) -> Option<String> {
  let query = url.split_once('?')?.1;
  query.split('&').find_map(|pair| {
    let (key, value) = pair.split_once('=')?;
    (key == name).then(|| decode(value))
  })
}

pub fn location(response: &Response<Body>) -> String {
  response
    .headers()
    .get("location")
    .expect("location header")
    .to_str()
    .unwrap()
    .to_string()
}

pub fn set_cookie(response: &Response<Body>) -> Option<String> {
  response
    .headers()
    .get_all("set-cookie")
    .iter()
    .map(|v| v.to_str().unwrap().to_string())
    .find(|c| c.starts_with(SESSION_COOKIE))
}

pub fn smtp() -> spec_store_core::config::SmtpSettings {
  spec_store_core::config::SmtpSettings {
    host: spec_store_testkit::mailpit::smtp_host(),
    port: spec_store_testkit::mailpit::smtp_port(),
    from: "no-reply@spec-store.test".into(),
    from_name: "spec-store".into(),
    username: None,
    password: None,
    starttls: false,
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
    let spec = TokenSpec::new(username, API_AUDIENCE);
    let spec = if username == "ops" {
      spec.operator()
    } else {
      spec
    };
    self.keycloak.mint(&spec)
  }

  pub fn token_with_email(&self, username: &str, email: &str) -> String {
    let mut spec = TokenSpec::new(username, API_AUDIENCE);
    spec.email = email.to_string();
    self.keycloak.mint(&spec)
  }

  pub async fn json(
    &self,
    method: &str,
    path: &str,
    username: &str,
    body: Value,
  ) -> Response<Body> {
    self
      .json_with_token(method, path, &self.token(username), body)
      .await
  }

  pub async fn json_with_token(
    &self,
    method: &str,
    path: &str,
    token: &str,
    body: Value,
  ) -> Response<Body> {
    self
      .send(
        Request::builder()
          .method(method)
          .uri(path)
          .header("host", "api.example.test")
          .header("authorization", format!("Bearer {token}"))
          .header("content-type", "application/json")
          .body(Body::from(body.to_string()))
          .unwrap(),
      )
      .await
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

impl TestApp {
  pub async fn browser(
    &self,
    method: &str,
    path: &str,
    cookie: &str,
    csrf: bool,
    body: Option<Value>,
  ) -> Response<Body> {
    let mut request = Request::builder()
      .method(method)
      .uri(path)
      .header("host", "app.example.test")
      .header("cookie", cookie.to_string());
    if csrf {
      request = request.header("x-spec-store-request", "1");
    }
    let body = match body {
      Some(value) => {
        request = request.header("content-type", "application/json");
        Body::from(value.to_string())
      }
      None => Body::empty(),
    };
    self.send(request.body(body).unwrap()).await
  }

  pub async fn start_login(&self, query: &str, cookie: Option<&str>) -> Response<Body> {
    let mut request =
      Request::get(format!("/auth/login{query}")).header("host", "app.example.test");
    if let Some(cookie) = cookie {
      request = request.header("cookie", cookie.to_string());
    }
    self.send(request.body(Body::empty()).unwrap()).await
  }

  pub async fn complete_login(&self, username: &str, authorize_url: &str) -> Response<Body> {
    let state = query_param(authorize_url, "state").expect("state");
    let challenge = query_param(authorize_url, "code_challenge").expect("challenge");
    let redirect_uri = query_param(authorize_url, "redirect_uri").expect("redirect uri");
    let code = self.keycloak.authorize(username, &challenge, &redirect_uri);
    self
      .send(
        Request::get(format!("/auth/callback?code={code}&state={state}"))
          .header("host", "app.example.test")
          .body(Body::empty())
          .unwrap(),
      )
      .await
  }

  pub async fn browser_sign_in(&self, username: &str) -> String {
    let started = self.start_login("?return_to=/", None).await;
    let finished = self.complete_login(username, &location(&started)).await;
    let cookie = set_cookie(&finished).expect("session cookie");
    cookie.split(';').next().unwrap().to_string()
  }

  pub fn person(&self, handle: &str) -> spec_store_core::model::Caller {
    self
      .state
      .store
      .caller_for(
        &spec_store_core::store::users::Identity {
          subject: format!("subject-{handle}"),
          handle: handle.to_string(),
          email: format!("{handle}@example.com"),
          name: handle.to_string(),
        },
        false,
      )
      .unwrap()
  }

  pub fn admit(&self, org: &str, handle: &str) {
    let caller = self.person(handle);
    self
      .state
      .store
      .admit_organization_member(org, caller.user_id, spec_store_core::model::OrgRole::Member)
      .unwrap();
  }

  pub async fn organization(&self, owner: &str, slug: &str) {
    let response = self
      .json(
        "POST",
        "/api/v1/orgs",
        owner,
        serde_json::json!({"slug": slug, "name": slug}),
      )
      .await;
    assert_eq!(response.status(), axum::http::StatusCode::CREATED);
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
