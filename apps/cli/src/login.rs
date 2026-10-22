use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::Deserialize;

pub const DEVICE_GRANT: &str = "urn:ietf:params:oauth:grant-type:device_code";
pub const SCOPES: &str = "openid offline_access";

#[derive(Debug, thiserror::Error)]
pub enum LoginError {
  #[error("cannot reach the identity provider: {0}")]
  Unreachable(String),
  #[error("the identity provider refused the sign-in: {0}")]
  Refused(String),
  #[error("sign-in timed out after {0} seconds without approval; run spec-store login again")]
  TimedOut(u64),
  #[error("sign-in was denied in the browser")]
  Denied,
  #[error("the session ended; run spec-store login again")]
  SessionEnded,
}

#[derive(Debug, Deserialize)]
pub struct DeviceAuthorization {
  pub device_code: String,
  pub user_code: String,
  pub verification_uri: String,
  #[serde(default)]
  pub verification_uri_complete: Option<String>,
  pub expires_in: u64,
  #[serde(default = "default_interval")]
  pub interval: u64,
}

fn default_interval() -> u64 {
  5
}

#[derive(Debug, Clone, Deserialize)]
pub struct TokenSet {
  pub access_token: String,
  #[serde(default)]
  pub refresh_token: Option<String>,
  #[serde(default)]
  pub expires_in: i64,
}

#[derive(Debug, Deserialize)]
struct OAuthError {
  error: String,
  #[serde(default)]
  error_description: Option<String>,
}

pub fn now() -> i64 {
  SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .map(|d| d.as_secs() as i64)
    .unwrap_or_default()
}

pub struct Endpoints {
  pub issuer: String,
  pub client_id: String,
}

impl Endpoints {
  fn url(&self, path: &str) -> String {
    format!(
      "{}/protocol/openid-connect{path}",
      self.issuer.trim_end_matches('/')
    )
  }
}

fn unreachable(e: reqwest::Error) -> LoginError {
  LoginError::Unreachable(e.to_string())
}

pub async fn start(
  http: &reqwest::Client,
  endpoints: &Endpoints,
) -> Result<DeviceAuthorization, LoginError> {
  let response = http
    .post(endpoints.url("/auth/device"))
    .form(&[
      ("client_id", endpoints.client_id.as_str()),
      ("scope", SCOPES),
    ])
    .send()
    .await
    .map_err(unreachable)?;
  if !response.status().is_success() {
    let error: Option<OAuthError> = response.json().await.ok();
    return Err(LoginError::Refused(
      error.map(|e| e.error).unwrap_or_default(),
    ));
  }
  response.json().await.map_err(unreachable)
}

pub async fn poll(
  http: &reqwest::Client,
  endpoints: &Endpoints,
  device: &DeviceAuthorization,
) -> Result<TokenSet, LoginError> {
  let deadline = Instant::now() + Duration::from_secs(device.expires_in);
  let mut interval = Duration::from_secs(device.interval.max(1));
  loop {
    if Instant::now() >= deadline {
      return Err(LoginError::TimedOut(device.expires_in));
    }
    tokio::time::sleep(interval).await;
    let response = http
      .post(endpoints.url("/token"))
      .form(&[
        ("grant_type", DEVICE_GRANT),
        ("device_code", device.device_code.as_str()),
        ("client_id", endpoints.client_id.as_str()),
      ])
      .send()
      .await
      .map_err(unreachable)?;
    if response.status().is_success() {
      return response.json().await.map_err(unreachable);
    }
    let error: OAuthError = response.json().await.map_err(unreachable)?;
    match error.error.as_str() {
      "authorization_pending" => {}
      "slow_down" => interval += Duration::from_secs(5),
      "access_denied" => return Err(LoginError::Denied),
      "expired_token" => return Err(LoginError::TimedOut(device.expires_in)),
      other => {
        return Err(LoginError::Refused(
          error.error_description.unwrap_or_else(|| other.to_string()),
        ));
      }
    }
  }
}

pub async fn refresh(
  http: &reqwest::Client,
  endpoints: &Endpoints,
  refresh_token: &str,
) -> Result<TokenSet, LoginError> {
  let response = http
    .post(endpoints.url("/token"))
    .form(&[
      ("grant_type", "refresh_token"),
      ("refresh_token", refresh_token),
      ("client_id", endpoints.client_id.as_str()),
    ])
    .send()
    .await
    .map_err(unreachable)?;
  if response.status().is_success() {
    return response.json().await.map_err(unreachable);
  }
  Err(LoginError::SessionEnded)
}

pub async fn logout(
  http: &reqwest::Client,
  endpoints: &Endpoints,
  refresh_token: &str,
) -> Result<(), LoginError> {
  let response = http
    .post(endpoints.url("/logout"))
    .form(&[
      ("client_id", endpoints.client_id.as_str()),
      ("refresh_token", refresh_token),
    ])
    .send()
    .await
    .map_err(unreachable)?;
  if response.status().is_success() || response.status() == reqwest::StatusCode::BAD_REQUEST {
    return Ok(());
  }
  Err(LoginError::Refused(format!(
    "logout answered {}",
    response.status()
  )))
}
