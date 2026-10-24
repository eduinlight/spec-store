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

use crate::keys::{GeneratedKey, key_a, key_b};

pub const REALM: &str = "sdd-vault";

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

  fn generated(self) -> &'static GeneratedKey {
    match self {
      TestKey::A => key_a(),
      TestKey::B => key_b(),
    }
  }

  fn pem(self) -> &'static str {
    &self.generated().pem
  }

  fn jwk(self) -> Value {
    json!({
      "kid": self.kid(),
      "kty": "RSA",
      "alg": "RS256",
      "use": "sig",
      "n": self.generated().modulus,
      "e": self.generated().exponent,
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
      client_id: "sdd-vault-test".to_string(),
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

pub const ADMIN_CLIENT_ID: &str = "sdd-vault-api-admin";
pub const ADMIN_CLIENT_SECRET: &str = "admin-secret";
const ADMIN_TOKEN: &str = "fake-admin-token";

pub const CLI_CLIENT_ID: &str = "sdd-vault-cli";
pub const CLI_AUDIENCE: &str = "sdd-vault-api";

#[derive(Debug, Clone)]
pub struct DeviceGrant {
  pub device_code: String,
  pub user_code: String,
  pub expires_at: i64,
  pub approved_for: Option<String>,
}

pub struct KeycloakState {
  pub published: Mutex<Vec<TestKey>>,
  pub jwks_fetches: AtomicUsize,
  pub consents: Mutex<HashMap<String, Vec<Value>>>,
  pub admin_token_requests: AtomicUsize,
  pub issuer: Mutex<String>,
  pub devices: Mutex<Vec<DeviceGrant>>,
  pub device_lifespan: Mutex<i64>,
  pub refresh_tokens: Mutex<HashMap<String, String>>,
  pub revoked_refresh_tokens: Mutex<Vec<String>>,
  pub disabled: Mutex<Vec<String>>,
  pub operators: Mutex<Vec<String>>,
  pub codes: Mutex<HashMap<String, (String, String, String)>>,
  pub refresh_grants: AtomicUsize,
  pub user_attributes: Mutex<HashMap<String, Value>>,
  pub issued_secrets: Mutex<Vec<String>>,
}

fn record_secret(state: &KeycloakState, secret: &str) {
  state
    .issued_secrets
    .lock()
    .expect("issued secrets")
    .push(secret.to_string());
}

fn mint_with(spec: &TokenSpec) -> String {
  let now = now_seconds();
  let claims = json!({
    "iss": spec.issuer.clone().unwrap_or_default(),
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

pub const APP_CLIENT_ID: &str = "sdd-vault-app";
pub const APP_CLIENT_SECRET: &str = "app-secret";

fn session_id(username: &str) -> String {
  format!("sid-{username}")
}

fn id_token(state: &KeycloakState, username: &str) -> String {
  let issuer = state.issuer.lock().expect("issuer").clone();
  let mut spec = TokenSpec::new(username, APP_CLIENT_ID).issuer(&issuer);
  spec.client_id = APP_CLIENT_ID.into();
  spec.session_id = Some(session_id(username));
  mint_with(&spec)
}

fn token_pair(state: &KeycloakState, username: &str) -> Value {
  let refresh = format!("refresh-{username}-{}", now_seconds_nanos());
  state
    .refresh_tokens
    .lock()
    .expect("refresh tokens")
    .insert(refresh.clone(), username.to_string());
  let issuer = state.issuer.lock().expect("issuer").clone();
  let mut spec = TokenSpec::new(username, CLI_AUDIENCE).issuer(&issuer);
  spec.session_id = Some(session_id(username));
  if state
    .operators
    .lock()
    .expect("operators")
    .contains(&username.to_string())
  {
    spec = spec.operator();
  }
  let access = mint_with(&spec);
  record_secret(state, &access);
  record_secret(state, &refresh);
  json!({
    "access_token": access,
    "refresh_token": refresh,
    "id_token": id_token(state, username),
    "expires_in": 300,
    "refresh_expires_in": 0,
    "token_type": "Bearer",
  })
}

fn pkce_challenge(verifier: &str) -> String {
  use base64::Engine;
  use sha2::Digest;
  base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(sha2::Sha256::digest(verifier.as_bytes()))
}

fn now_seconds_nanos() -> u128 {
  SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .expect("clock")
    .as_nanos()
}

fn oauth_error(error: &str) -> Response {
  (StatusCode::BAD_REQUEST, Json(json!({ "error": error }))).into_response()
}

async fn device_authorization(
  State(state): State<Arc<KeycloakState>>,
  Form(form): Form<HashMap<String, String>>,
) -> Response {
  if form.get("client_id").map(String::as_str) != Some(CLI_CLIENT_ID) {
    return oauth_error("invalid_client");
  }
  let n = now_seconds_nanos();
  let lifespan = *state.device_lifespan.lock().expect("lifespan");
  let grant = DeviceGrant {
    device_code: format!("device-{n}"),
    user_code: format!("CODE-{}", n % 10_000),
    expires_at: now_seconds() + lifespan,
    approved_for: None,
  };
  state.devices.lock().expect("devices").push(grant.clone());
  record_secret(&state, &grant.device_code);
  let issuer = state.issuer.lock().expect("issuer").clone();
  Json(json!({
    "device_code": grant.device_code,
    "user_code": grant.user_code,
    "verification_uri": format!("{issuer}/device"),
    "verification_uri_complete": format!("{issuer}/device?user_code={}", grant.user_code),
    "expires_in": lifespan,
    "interval": 1,
  }))
  .into_response()
}

async fn logout(
  State(state): State<Arc<KeycloakState>>,
  Form(form): Form<HashMap<String, String>>,
) -> Response {
  if let Some(refresh) = form.get("refresh_token") {
    state
      .revoked_refresh_tokens
      .lock()
      .expect("revoked")
      .push(refresh.clone());
    state
      .refresh_tokens
      .lock()
      .expect("refresh tokens")
      .remove(refresh);
  }
  StatusCode::NO_CONTENT.into_response()
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
  if grant == Some("urn:ietf:params:oauth:grant-type:device_code") {
    let code = form.get("device_code").cloned().unwrap_or_default();
    let found = state
      .devices
      .lock()
      .expect("devices")
      .iter()
      .find(|d| d.device_code == code)
      .cloned();
    return match found {
      None => oauth_error("invalid_grant"),
      Some(grant) if grant.expires_at <= now_seconds() => oauth_error("expired_token"),
      Some(DeviceGrant {
        approved_for: None, ..
      }) => oauth_error("authorization_pending"),
      Some(DeviceGrant {
        approved_for: Some(username),
        ..
      }) => {
        state
          .devices
          .lock()
          .expect("devices")
          .retain(|d| d.device_code != code);
        Json(token_pair(&state, &username)).into_response()
      }
    };
  }
  if grant == Some("authorization_code") {
    if form.get("client_id").map(String::as_str) != Some(APP_CLIENT_ID)
      || form.get("client_secret").map(String::as_str) != Some(APP_CLIENT_SECRET)
    {
      return oauth_error("unauthorized_client");
    }
    let code = form.get("code").cloned().unwrap_or_default();
    let found = state.codes.lock().expect("codes").remove(&code);
    let verifier = form.get("code_verifier").cloned().unwrap_or_default();
    return match found {
      Some((username, challenge, redirect_uri))
        if challenge == pkce_challenge(&verifier)
          && form.get("redirect_uri").map(String::as_str) == Some(redirect_uri.as_str()) =>
      {
        Json(token_pair(&state, &username)).into_response()
      }
      _ => oauth_error("invalid_grant"),
    };
  }
  if grant == Some("refresh_token") {
    state.refresh_grants.fetch_add(1, Ordering::SeqCst);
    let refresh = form.get("refresh_token").cloned().unwrap_or_default();
    let username = state
      .refresh_tokens
      .lock()
      .expect("refresh tokens")
      .remove(&refresh);
    return match username {
      Some(username) if state.disabled.lock().expect("disabled").contains(&username) => (
        StatusCode::BAD_REQUEST,
        Json(json!({ "error": "invalid_grant", "error_description": "User disabled" })),
      )
        .into_response(),
      Some(username) => Json(token_pair(&state, &username)).into_response(),
      None => oauth_error("invalid_grant"),
    };
  }
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

async fn read_user(
  State(state): State<Arc<KeycloakState>>,
  Path((_realm, user)): Path<(String, String)>,
  headers: HeaderMap,
) -> Response {
  if !admin_authorized(&headers) {
    return StatusCode::UNAUTHORIZED.into_response();
  }
  let attributes = state
    .user_attributes
    .lock()
    .expect("attributes")
    .get(&user)
    .cloned()
    .unwrap_or_else(|| json!({ "keep": ["this"] }));
  Json(json!({ "id": user, "username": user, "enabled": true, "attributes": attributes }))
    .into_response()
}

async fn update_user(
  State(state): State<Arc<KeycloakState>>,
  Path((_realm, user)): Path<(String, String)>,
  headers: HeaderMap,
  Json(body): Json<Value>,
) -> Response {
  if !admin_authorized(&headers) {
    return StatusCode::UNAUTHORIZED.into_response();
  }
  state
    .user_attributes
    .lock()
    .expect("attributes")
    .insert(user, body["attributes"].clone());
  StatusCode::NO_CONTENT.into_response()
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
      issuer: Mutex::new(String::new()),
      devices: Mutex::new(Vec::new()),
      device_lifespan: Mutex::new(600),
      refresh_tokens: Mutex::new(HashMap::new()),
      revoked_refresh_tokens: Mutex::new(Vec::new()),
      disabled: Mutex::new(Vec::new()),
      operators: Mutex::new(Vec::new()),
      codes: Mutex::new(HashMap::new()),
      refresh_grants: AtomicUsize::new(0),
      user_attributes: Mutex::new(HashMap::new()),
      issued_secrets: Mutex::new(Vec::new()),
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
        &format!("/realms/{REALM}/protocol/openid-connect/auth/device"),
        post(device_authorization),
      )
      .route(
        &format!("/realms/{REALM}/protocol/openid-connect/logout"),
        post(logout),
      )
      .route(
        "/admin/realms/{realm}/users/{user}/consents",
        get(list_consents),
      )
      .route(
        "/admin/realms/{realm}/users/{user}",
        get(read_user).put(update_user),
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
    *state.issuer.lock().expect("issuer") = format!("http://{address}/realms/{REALM}");
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
        "grantedClientScopes": ["openid", "sdd-vault-mcp", "offline_access"],
        "createdDate": 1_760_000_000_000_i64,
        "lastUpdatedDate": 1_760_000_500_000_i64,
      }));
  }

  pub fn clear_consents(&self) {
    self.state.consents.lock().expect("consents").clear();
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

  pub fn set_device_lifespan(&self, seconds: i64) {
    *self.state.device_lifespan.lock().expect("lifespan") = seconds;
  }

  pub fn approve_device(&self, user_code: &str, username: &str) -> bool {
    let mut devices = self.state.devices.lock().expect("devices");
    match devices.iter_mut().find(|d| d.user_code == user_code) {
      Some(grant) => {
        grant.approved_for = Some(username.to_string());
        true
      }
      None => false,
    }
  }

  pub fn issue_refresh_token(&self, username: &str) -> String {
    token_pair(&self.state, username)["refresh_token"]
      .as_str()
      .unwrap()
      .to_string()
  }

  pub fn revoked_refresh_tokens(&self) -> Vec<String> {
    self
      .state
      .revoked_refresh_tokens
      .lock()
      .expect("revoked")
      .clone()
  }

  pub fn authorize(&self, username: &str, code_challenge: &str, redirect_uri: &str) -> String {
    let code = format!("code-{username}-{}", now_seconds_nanos());
    self.state.codes.lock().expect("codes").insert(
      code.clone(),
      (
        username.to_string(),
        code_challenge.to_string(),
        redirect_uri.to_string(),
      ),
    );
    code
  }

  pub fn make_operator(&self, username: &str) {
    self
      .state
      .operators
      .lock()
      .expect("operators")
      .push(username.to_string());
  }

  pub fn user_attributes(&self, subject: &str) -> Option<Value> {
    self
      .state
      .user_attributes
      .lock()
      .expect("attributes")
      .get(subject)
      .cloned()
  }

  pub fn refresh_grants(&self) -> usize {
    self.state.refresh_grants.load(Ordering::SeqCst)
  }

  pub fn mint_logout_token(&self, username: &str) -> String {
    let now = now_seconds();
    let claims = json!({
      "iss": self.issuer(),
      "aud": APP_CLIENT_ID,
      "iat": now,
      "exp": now + 60,
      "jti": format!("logout-{}", now_seconds_nanos()),
      "sub": format!("subject-{username}"),
      "sid": session_id(username),
      "events": { "http://schemas.openid.net/event/backchannel-logout": {} },
    });
    let mut header = Header::new(Algorithm::RS256);
    header.kid = Some(TestKey::A.kid().to_string());
    let key = EncodingKey::from_rsa_pem(TestKey::A.pem().as_bytes()).expect("test key");
    jsonwebtoken::encode(&header, &claims, &key).expect("sign logout token")
  }

  pub fn disable(&self, username: &str) {
    self
      .state
      .disabled
      .lock()
      .expect("disabled")
      .push(username.to_string());
  }

  pub fn jwks_fetches(&self) -> usize {
    self.state.jwks_fetches.load(Ordering::SeqCst)
  }

  pub fn mint(&self, spec: &TokenSpec) -> String {
    let mut spec = spec.clone();
    if spec.issuer.is_none() {
      spec.issuer = Some(self.issuer());
    }
    let token = mint_with(&spec);
    record_secret(&self.state, &token);
    token
  }

  pub fn issued_secrets(&self) -> Vec<String> {
    self
      .state
      .issued_secrets
      .lock()
      .expect("issued secrets")
      .clone()
  }
}

impl Drop for FakeKeycloak {
  fn drop(&mut self) {
    self.server.abort();
  }
}
