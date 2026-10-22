use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use std::collections::HashMap;

use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get, post};
use axum::{Form, Json, Router};
use jsonwebtoken::{Algorithm, EncodingKey, Header};
use serde_json::{Value, json};
use tokio::task::JoinHandle;

use crate::keys::{EXPONENT, KEY_A_MODULUS, KEY_A_PEM, KEY_B_MODULUS, KEY_B_PEM};

pub const REALM: &str = "spec-store";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestKey {
  A,
  B,
}

impl TestKey {
  pub fn kid(self) -> &'static str {
    match self {
      TestKey::A => "key-a",
      TestKey::B => "key-b",
    }
  }

  fn pem(self) -> &'static str {
    match self {
      TestKey::A => KEY_A_PEM,
      TestKey::B => KEY_B_PEM,
    }
  }

  fn modulus(self) -> &'static str {
    match self {
      TestKey::A => KEY_A_MODULUS,
      TestKey::B => KEY_B_MODULUS,
    }
  }

  fn jwk(self) -> Value {
    json!({
      "kid": self.kid(),
      "kty": "RSA",
      "alg": "RS256",
      "use": "sig",
      "n": self.modulus(),
      "e": EXPONENT,
    })
  }
}

#[derive(Debug, Clone)]
pub struct TokenSpec {
  pub subject: String,
  pub username: String,
  pub email: String,
  pub name: String,
  pub audience: Vec<String>,
  pub roles: Vec<String>,
  pub ttl_seconds: i64,
  pub key: TestKey,
  pub issuer: Option<String>,
  pub session_id: Option<String>,
  pub client_id: String,
}

impl TokenSpec {
  pub fn new(username: &str, audience: &str) -> TokenSpec {
    TokenSpec {
      subject: format!("subject-{username}"),
      username: username.to_string(),
      email: format!("{username}@example.com"),
      name: username.to_string(),
      audience: vec![audience.to_string()],
      roles: Vec::new(),
      ttl_seconds: 300,
      key: TestKey::A,
      issuer: None,
      session_id: Some(format!("session-{username}")),
      client_id: "spec-store-test".to_string(),
    }
  }

  pub fn operator(mut self) -> TokenSpec {
    self.roles.push("platform-operator".to_string());
    self
  }

  pub fn ttl(mut self, seconds: i64) -> TokenSpec {
    self.ttl_seconds = seconds;
    self
  }

  pub fn key(mut self, key: TestKey) -> TokenSpec {
    self.key = key;
    self
  }

  pub fn issuer(mut self, issuer: &str) -> TokenSpec {
    self.issuer = Some(issuer.to_string());
    self
  }
}

pub fn now_seconds() -> i64 {
  SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .expect("clock after epoch")
    .as_secs() as i64
}

pub const ADMIN_CLIENT_ID: &str = "spec-store-api-admin";
pub const ADMIN_CLIENT_SECRET: &str = "admin-secret";
const ADMIN_TOKEN: &str = "fake-admin-token";

pub struct KeycloakState {
  pub published: Mutex<Vec<TestKey>>,
  pub jwks_fetches: AtomicUsize,
  pub consents: Mutex<HashMap<String, Vec<Value>>>,
  pub admin_token_requests: AtomicUsize,
}

fn admin_authorized(headers: &HeaderMap) -> bool {
  headers
    .get("authorization")
    .and_then(|v| v.to_str().ok())
    .is_some_and(|v| v == format!("Bearer {ADMIN_TOKEN}"))
}

async fn token(
  State(state): State<Arc<KeycloakState>>,
  Form(form): Form<HashMap<String, String>>,
) -> Response {
  let grant = form.get("grant_type").map(String::as_str);
  if grant == Some("client_credentials")
    && form.get("client_id").map(String::as_str) == Some(ADMIN_CLIENT_ID)
    && form.get("client_secret").map(String::as_str) == Some(ADMIN_CLIENT_SECRET)
  {
    state.admin_token_requests.fetch_add(1, Ordering::SeqCst);
    return Json(json!({ "access_token": ADMIN_TOKEN, "expires_in": 300, "token_type": "Bearer" }))
      .into_response();
  }
  (
    StatusCode::UNAUTHORIZED,
    Json(json!({ "error": "unauthorized_client" })),
  )
    .into_response()
}

async fn list_consents(
  State(state): State<Arc<KeycloakState>>,
  Path((_realm, user)): Path<(String, String)>,
  headers: HeaderMap,
) -> Response {
  if !admin_authorized(&headers) {
    return StatusCode::UNAUTHORIZED.into_response();
  }
  let consents = state
    .consents
    .lock()
    .expect("consents")
    .get(&user)
    .cloned()
    .unwrap_or_default();
  Json(Value::Array(consents)).into_response()
}

async fn revoke_consent(
  State(state): State<Arc<KeycloakState>>,
  Path((_realm, user, client)): Path<(String, String, String)>,
  headers: HeaderMap,
) -> Response {
  if !admin_authorized(&headers) {
    return StatusCode::UNAUTHORIZED.into_response();
  }
  let mut consents = state.consents.lock().expect("consents");
  let entries = consents.entry(user).or_default();
  let before = entries.len();
  entries.retain(|c| c["clientId"] != client.as_str());
  if entries.len() == before {
    return (
      StatusCode::NOT_FOUND,
      Json(json!({ "error": "Consent nor offline token not found" })),
    )
      .into_response();
  }
  StatusCode::NO_CONTENT.into_response()
}

pub struct FakeKeycloak {
  pub base_url: String,
  pub address: SocketAddr,
  pub state: Arc<KeycloakState>,
  server: JoinHandle<()>,
}

async fn jwks(State(state): State<Arc<KeycloakState>>) -> Json<Value> {
  state.jwks_fetches.fetch_add(1, Ordering::SeqCst);
  let keys: Vec<Value> = state
    .published
    .lock()
    .expect("published keys")
    .iter()
    .map(|k| k.jwk())
    .collect();
  Json(json!({ "keys": keys }))
}

impl FakeKeycloak {
  pub async fn start() -> FakeKeycloak {
    let state = Arc::new(KeycloakState {
      published: Mutex::new(vec![TestKey::A]),
      jwks_fetches: AtomicUsize::new(0),
      consents: Mutex::new(HashMap::new()),
      admin_token_requests: AtomicUsize::new(0),
    });
    let app = Router::new()
      .route(
        &format!("/realms/{REALM}/protocol/openid-connect/certs"),
        get(jwks),
      )
      .route(
        &format!("/realms/{REALM}/protocol/openid-connect/token"),
        post(token),
      )
      .route(
        "/admin/realms/{realm}/users/{user}/consents",
        get(list_consents),
      )
      .route(
        "/admin/realms/{realm}/users/{user}/consents/{client}",
        delete(revoke_consent),
      )
      .with_state(Arc::clone(&state));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
      .await
      .expect("bind fake keycloak");
    let address = listener.local_addr().expect("local address");
    let server = tokio::spawn(async move {
      axum::serve(listener, app).await.expect("fake keycloak");
    });
    FakeKeycloak {
      base_url: format!("http://{address}"),
      address,
      state,
      server,
    }
  }

  pub fn issuer(&self) -> String {
    format!("{}/realms/{REALM}", self.base_url)
  }

  pub fn jwks_url(&self) -> String {
    format!("{}/protocol/openid-connect/certs", self.issuer())
  }

  pub fn publish(&self, key: TestKey) {
    let mut published = self.state.published.lock().expect("published keys");
    if !published.contains(&key) {
      published.push(key);
    }
  }

  pub fn grant_consent(&self, subject: &str, client_id: &str, name: &str) {
    self
      .state
      .consents
      .lock()
      .expect("consents")
      .entry(subject.to_string())
      .or_default()
      .push(json!({
        "clientId": client_id,
        "clientName": name,
        "grantedClientScopes": ["openid", "spec-store-mcp", "offline_access"],
        "createdDate": 1_760_000_000_000_i64,
        "lastUpdatedDate": 1_760_000_500_000_i64,
      }));
  }

  pub fn consent_client_ids(&self, subject: &str) -> Vec<String> {
    self
      .state
      .consents
      .lock()
      .expect("consents")
      .get(subject)
      .map(|c| {
        c.iter()
          .map(|v| v["clientId"].as_str().unwrap_or_default().to_string())
          .collect()
      })
      .unwrap_or_default()
  }

  pub fn jwks_fetches(&self) -> usize {
    self.state.jwks_fetches.load(Ordering::SeqCst)
  }

  pub fn mint(&self, spec: &TokenSpec) -> String {
    let now = now_seconds();
    let claims = json!({
      "iss": spec.issuer.clone().unwrap_or_else(|| self.issuer()),
      "sub": spec.subject,
      "aud": spec.audience,
      "exp": now + spec.ttl_seconds,
      "iat": now,
      "azp": spec.client_id,
      "sid": spec.session_id,
      "preferred_username": spec.username,
      "email": spec.email,
      "email_verified": true,
      "name": spec.name,
      "realm_access": { "roles": spec.roles },
    });
    let mut header = Header::new(Algorithm::RS256);
    header.kid = Some(spec.key.kid().to_string());
    let key = EncodingKey::from_rsa_pem(spec.key.pem().as_bytes()).expect("test key");
    jsonwebtoken::encode(&header, &claims, &key).expect("sign token")
  }
}

impl Drop for FakeKeycloak {
  fn drop(&mut self) {
    self.server.abort();
  }
}
