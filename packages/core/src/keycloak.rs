use std::time::{Duration, Instant};

use chrono::{DateTime, SecondsFormat};
use serde::Deserialize;
use tokio::sync::Mutex;

use crate::model::StoreError;

pub const HTTP_TIMEOUT: Duration = Duration::from_secs(5);
const TOKEN_MARGIN: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdminConfig {
  pub base_url: String,
  pub realm: String,
  pub client_id: String,
  pub client_secret: String,
}

#[derive(Debug, thiserror::Error)]
pub enum KeycloakError {
  #[error("{0} not found")]
  NotFound(String),
  #[error("the identity provider refused the grant")]
  InvalidGrant,
  #[error("identity provider unavailable: {0}")]
  Unavailable(String),
  #[error("identity provider refused the request: {0}")]
  Rejected(String),
}

impl From<KeycloakError> for StoreError {
  fn from(error: KeycloakError) -> StoreError {
    match error {
      KeycloakError::NotFound(what) => StoreError::NotFound(what),
      KeycloakError::InvalidGrant => StoreError::Unauthenticated,
      KeycloakError::Unavailable(detail) | KeycloakError::Rejected(detail) => {
        tracing::warn!(%detail, "identity provider call failed");
        StoreError::Unavailable("the identity provider".into())
      }
    }
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Consent {
  pub client_id: String,
  pub name: Option<String>,
  pub scopes: Vec<String>,
  pub granted_at: String,
  pub updated_at: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ConsentRepresentation {
  client_id: String,
  #[serde(default)]
  client_name: Option<String>,
  #[serde(default)]
  granted_client_scopes: Vec<String>,
  #[serde(default)]
  created_date: i64,
  #[serde(default)]
  last_updated_date: i64,
}

#[derive(Deserialize)]
struct TokenResponse {
  access_token: String,
  expires_in: u64,
}

pub fn iso_millis(millis: i64) -> String {
  DateTime::from_timestamp_millis(millis)
    .unwrap_or_default()
    .to_rfc3339_opts(SecondsFormat::Millis, true)
}

pub fn encode_segment(value: &str) -> String {
  value
    .bytes()
    .map(|b| match b {
      b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
        (b as char).to_string()
      }
      _ => format!("%{b:02X}"),
    })
    .collect()
}

pub struct KeycloakAdmin {
  config: AdminConfig,
  http: reqwest::Client,
  token: Mutex<Option<(String, Instant)>>,
}

fn unavailable(error: reqwest::Error) -> KeycloakError {
  KeycloakError::Unavailable(error.to_string())
}

impl KeycloakAdmin {
  pub fn new(config: AdminConfig) -> KeycloakAdmin {
    KeycloakAdmin {
      config,
      http: reqwest::Client::builder()
        .timeout(HTTP_TIMEOUT)
        .build()
        .expect("http client"),
      token: Mutex::new(None),
    }
  }

  pub fn config(&self) -> &AdminConfig {
    &self.config
  }

  fn realm_url(&self, path: &str) -> String {
    format!(
      "{}/realms/{}{path}",
      self.config.base_url.trim_end_matches('/'),
      self.config.realm
    )
  }

  fn admin_url(&self, path: &str) -> String {
    format!(
      "{}/admin/realms/{}{path}",
      self.config.base_url.trim_end_matches('/'),
      self.config.realm
    )
  }

  async fn admin_token(&self) -> Result<String, KeycloakError> {
    let mut cached = self.token.lock().await;
    if let Some((token, until)) = cached.as_ref()
      && Instant::now() < *until
    {
      return Ok(token.clone());
    }
    let response = self
      .http
      .post(self.realm_url("/protocol/openid-connect/token"))
      .form(&[
        ("grant_type", "client_credentials"),
        ("client_id", self.config.client_id.as_str()),
        ("client_secret", self.config.client_secret.as_str()),
      ])
      .send()
      .await
      .map_err(unavailable)?;
    if !response.status().is_success() {
      return Err(KeycloakError::Rejected(format!(
        "service account token request answered {}",
        response.status()
      )));
    }
    let token: TokenResponse = response.json().await.map_err(unavailable)?;
    let lifetime = Duration::from_secs(token.expires_in).saturating_sub(TOKEN_MARGIN);
    *cached = Some((token.access_token.clone(), Instant::now() + lifetime));
    Ok(token.access_token)
  }

  pub async fn set_locale(&self, user_id: &str, locale: &str) -> Result<(), KeycloakError> {
    let token = self.admin_token().await?;
    let url = self.admin_url(&format!("/users/{}", encode_segment(user_id)));
    let response = self
      .http
      .get(&url)
      .bearer_auth(&token)
      .send()
      .await
      .map_err(unavailable)?;
    if response.status() == reqwest::StatusCode::NOT_FOUND {
      return Err(KeycloakError::NotFound(format!("identity {user_id:?}")));
    }
    if !response.status().is_success() {
      return Err(KeycloakError::Rejected(format!(
        "reading the user answered {}",
        response.status()
      )));
    }
    let mut user: serde_json::Value = response.json().await.map_err(unavailable)?;
    if !user["attributes"].is_object() {
      user["attributes"] = serde_json::json!({});
    }
    user["attributes"]["locale"] = serde_json::json!([locale]);
    let response = self
      .http
      .put(&url)
      .bearer_auth(&token)
      .json(&user)
      .send()
      .await
      .map_err(unavailable)?;
    if !response.status().is_success() {
      return Err(KeycloakError::Rejected(format!(
        "updating the user answered {}",
        response.status()
      )));
    }
    Ok(())
  }

  pub async fn list_consents(&self, user_id: &str) -> Result<Vec<Consent>, KeycloakError> {
    let token = self.admin_token().await?;
    let response = self
      .http
      .get(self.admin_url(&format!("/users/{}/consents", encode_segment(user_id))))
      .bearer_auth(token)
      .send()
      .await
      .map_err(unavailable)?;
    if response.status() == reqwest::StatusCode::NOT_FOUND {
      return Ok(Vec::new());
    }
    if !response.status().is_success() {
      return Err(KeycloakError::Rejected(format!(
        "listing consents answered {}",
        response.status()
      )));
    }
    let consents: Vec<ConsentRepresentation> = response.json().await.map_err(unavailable)?;
    Ok(
      consents
        .into_iter()
        .map(|c| Consent {
          client_id: c.client_id,
          name: c.client_name,
          scopes: c.granted_client_scopes,
          granted_at: iso_millis(c.created_date),
          updated_at: iso_millis(c.last_updated_date),
        })
        .collect(),
    )
  }

  pub async fn revoke_consent(&self, user_id: &str, client_id: &str) -> Result<(), KeycloakError> {
    let token = self.admin_token().await?;
    let response = self
      .http
      .delete(self.admin_url(&format!(
        "/users/{}/consents/{}",
        encode_segment(user_id),
        encode_segment(client_id)
      )))
      .bearer_auth(token)
      .send()
      .await
      .map_err(unavailable)?;
    match response.status() {
      status if status.is_success() => Ok(()),
      reqwest::StatusCode::NOT_FOUND => Err(KeycloakError::NotFound(format!(
        "authorized client {client_id:?}"
      ))),
      status => Err(KeycloakError::Rejected(format!(
        "revoking consent answered {status}"
      ))),
    }
  }
}

#[derive(Debug, Clone, Deserialize)]
pub struct OidcTokens {
  pub access_token: String,
  #[serde(default)]
  pub refresh_token: Option<String>,
  #[serde(default)]
  pub id_token: Option<String>,
  #[serde(default)]
  pub expires_in: i64,
}

pub fn pkce_pair() -> (String, String) {
  use base64::Engine;
  use sha2::Digest;
  let verifier = crate::secrets::random_secret("");
  let challenge = base64::engine::general_purpose::URL_SAFE_NO_PAD
    .encode(sha2::Sha256::digest(verifier.as_bytes()));
  (verifier, challenge)
}

fn query(pairs: &[(&str, &str)]) -> String {
  pairs
    .iter()
    .map(|(k, v)| format!("{k}={}", encode_segment(v)))
    .collect::<Vec<_>>()
    .join("&")
}

pub struct OidcClient {
  issuer: String,
  client_id: String,
  client_secret: String,
  http: reqwest::Client,
}

pub struct AuthorizeRequest<'a> {
  pub redirect_uri: &'a str,
  pub state: &'a str,
  pub code_challenge: &'a str,
  pub ui_locales: &'a str,
  pub register: bool,
}

impl OidcClient {
  pub fn new(issuer: &str, client_id: &str, client_secret: &str) -> OidcClient {
    OidcClient {
      issuer: issuer.trim_end_matches('/').to_string(),
      client_id: client_id.to_string(),
      client_secret: client_secret.to_string(),
      http: reqwest::Client::builder()
        .timeout(HTTP_TIMEOUT)
        .build()
        .expect("http client"),
    }
  }

  pub fn client_id(&self) -> &str {
    &self.client_id
  }

  fn endpoint(&self, path: &str) -> String {
    format!("{}/protocol/openid-connect{path}", self.issuer)
  }

  pub fn authorize_url(&self, request: &AuthorizeRequest<'_>) -> String {
    let path = if request.register {
      "/registrations"
    } else {
      "/auth"
    };
    format!(
      "{}?{}",
      self.endpoint(path),
      query(&[
        ("response_type", "code"),
        ("client_id", &self.client_id),
        ("redirect_uri", request.redirect_uri),
        ("scope", "openid profile email"),
        ("state", request.state),
        ("code_challenge", request.code_challenge),
        ("code_challenge_method", "S256"),
        ("ui_locales", request.ui_locales),
      ])
    )
  }

  pub fn end_session_url(
    &self,
    id_token: &str,
    post_logout_redirect_uri: &str,
    ui_locales: &str,
  ) -> String {
    format!(
      "{}?{}",
      self.endpoint("/logout"),
      query(&[
        ("client_id", &self.client_id),
        ("id_token_hint", id_token),
        ("post_logout_redirect_uri", post_logout_redirect_uri),
        ("ui_locales", ui_locales),
      ])
    )
  }

  async fn token_request(&self, form: &[(&str, &str)]) -> Result<OidcTokens, KeycloakError> {
    let response = self
      .http
      .post(self.endpoint("/token"))
      .form(form)
      .send()
      .await
      .map_err(unavailable)?;
    if response.status().is_success() {
      return response.json().await.map_err(unavailable);
    }
    if response.status() == reqwest::StatusCode::BAD_REQUEST {
      return Err(KeycloakError::InvalidGrant);
    }
    Err(KeycloakError::Rejected(format!(
      "token request answered {}",
      response.status()
    )))
  }

  pub async fn exchange_code(
    &self,
    code: &str,
    redirect_uri: &str,
    verifier: &str,
  ) -> Result<OidcTokens, KeycloakError> {
    self
      .token_request(&[
        ("grant_type", "authorization_code"),
        ("client_id", &self.client_id),
        ("client_secret", &self.client_secret),
        ("code", code),
        ("redirect_uri", redirect_uri),
        ("code_verifier", verifier),
      ])
      .await
  }

  pub async fn refresh(&self, refresh_token: &str) -> Result<OidcTokens, KeycloakError> {
    self
      .token_request(&[
        ("grant_type", "refresh_token"),
        ("client_id", &self.client_id),
        ("client_secret", &self.client_secret),
        ("refresh_token", refresh_token),
      ])
      .await
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn pkce_challenges_match_their_verifiers() {
    use base64::Engine;
    use sha2::Digest;
    let (verifier, challenge) = pkce_pair();
    assert!(verifier.len() >= 43);
    assert_eq!(
      challenge,
      base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(sha2::Sha256::digest(verifier.as_bytes()))
    );
  }

  #[test]
  fn encodes_path_segments_and_formats_times() {
    assert_eq!(encode_segment("claude-code_1.x"), "claude-code_1.x");
    assert_eq!(
      encode_segment("https://claude.ai/oauth/client.json"),
      "https%3A%2F%2Fclaude.ai%2Foauth%2Fclient.json"
    );
    assert_eq!(iso_millis(1_760_000_000_000), "2025-10-09T08:53:20.000Z");
  }
}
