use std::collections::HashMap;
use std::time::{Duration, Instant};

use jsonwebtoken::errors::ErrorKind;
use jsonwebtoken::jwk::JwkSet;
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode, decode_header};
use serde::Deserialize;
use tokio::sync::{Mutex, RwLock};

use crate::store::users::Identity;

pub const OPERATOR_ROLE: &str = "platform-operator";
pub const KEY_SET_MAX_AGE: Duration = Duration::from_secs(600);
pub const UNKNOWN_KEY_REFRESH_INTERVAL: Duration = Duration::from_secs(10);
pub const HTTP_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentityConfig {
  pub issuer: String,
  pub jwks_url: String,
}

impl IdentityConfig {
  pub fn for_issuer(issuer: &str) -> IdentityConfig {
    let issuer = issuer.trim_end_matches('/').to_string();
    IdentityConfig {
      jwks_url: format!("{issuer}/protocol/openid-connect/certs"),
      issuer,
    }
  }
}

#[derive(Debug, thiserror::Error)]
pub enum IdentityError {
  #[error("token expired")]
  Expired,
  #[error("invalid token: {0}")]
  Invalid(String),
  #[error("identity provider unavailable: {0}")]
  Unavailable(String),
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct RealmAccess {
  #[serde(default)]
  pub roles: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Claims {
  pub sub: String,
  pub exp: i64,
  #[serde(default)]
  pub preferred_username: String,
  #[serde(default)]
  pub email: String,
  #[serde(default)]
  pub email_verified: bool,
  #[serde(default)]
  pub name: String,
  #[serde(default)]
  pub sid: Option<String>,
  #[serde(default)]
  pub azp: Option<String>,
  #[serde(default)]
  pub realm_access: RealmAccess,
}

impl Claims {
  pub fn is_operator(&self) -> bool {
    self.realm_access.roles.iter().any(|r| r == OPERATOR_ROLE)
  }

  pub fn identity(&self) -> Identity {
    let handle = if self.preferred_username.is_empty() {
      self.sub.clone()
    } else {
      self.preferred_username.clone()
    };
    Identity {
      subject: self.sub.clone(),
      name: if self.name.is_empty() {
        handle.clone()
      } else {
        self.name.clone()
      },
      handle,
      email: self.email.clone(),
    }
  }
}

struct KeySet {
  keys: HashMap<String, DecodingKey>,
  loaded_at: Option<Instant>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LogoutClaims {
  #[serde(default)]
  pub sid: Option<String>,
  #[serde(default)]
  pub sub: Option<String>,
  #[serde(default)]
  pub events: serde_json::Value,
}

pub const BACKCHANNEL_LOGOUT_EVENT: &str = "http://schemas.openid.net/event/backchannel-logout";

pub struct Verifier {
  config: IdentityConfig,
  http: reqwest::Client,
  keys: RwLock<KeySet>,
  unknown_key_refreshed_at: Mutex<Option<Instant>>,
}

impl Verifier {
  pub fn new(config: IdentityConfig) -> Verifier {
    Verifier {
      config,
      http: reqwest::Client::builder()
        .timeout(HTTP_TIMEOUT)
        .build()
        .expect("http client"),
      keys: RwLock::new(KeySet {
        keys: HashMap::new(),
        loaded_at: None,
      }),
      unknown_key_refreshed_at: Mutex::new(None),
    }
  }

  pub fn config(&self) -> &IdentityConfig {
    &self.config
  }

  async fn fetch(&self) -> Result<HashMap<String, DecodingKey>, IdentityError> {
    let unavailable = |e: reqwest::Error| IdentityError::Unavailable(e.to_string());
    let set: JwkSet = self
      .http
      .get(&self.config.jwks_url)
      .send()
      .await
      .map_err(unavailable)?
      .error_for_status()
      .map_err(unavailable)?
      .json()
      .await
      .map_err(unavailable)?;
    Ok(
      set
        .keys
        .iter()
        .filter_map(|jwk| {
          let kid = jwk.common.key_id.clone()?;
          DecodingKey::from_jwk(jwk).ok().map(|key| (kid, key))
        })
        .collect(),
    )
  }

  pub async fn refresh(&self) -> Result<(), IdentityError> {
    let keys = self.fetch().await?;
    let mut set = self.keys.write().await;
    set.keys = keys;
    set.loaded_at = Some(Instant::now());
    Ok(())
  }

  pub async fn reachable(&self) -> bool {
    self.fetch().await.is_ok()
  }

  async fn cached(&self, kid: &str) -> (Option<DecodingKey>, bool) {
    let set = self.keys.read().await;
    let fresh = set
      .loaded_at
      .is_some_and(|at| at.elapsed() < KEY_SET_MAX_AGE);
    (set.keys.get(kid).cloned(), fresh)
  }

  async fn key_for(&self, kid: &str) -> Result<DecodingKey, IdentityError> {
    let (key, fresh) = self.cached(kid).await;
    if !fresh {
      match self.refresh().await {
        Ok(()) => {}
        Err(error) if key.is_none() => return Err(error),
        Err(error) => tracing::warn!(%error, "keeping cached signing keys"),
      }
      if let (Some(key), _) = self.cached(kid).await {
        return Ok(key);
      }
    } else if let Some(key) = key {
      return Ok(key);
    }
    {
      let mut last = self.unknown_key_refreshed_at.lock().await;
      if last.is_some_and(|at| at.elapsed() < UNKNOWN_KEY_REFRESH_INTERVAL) {
        return Err(IdentityError::Invalid(format!(
          "unknown signing key {kid:?}"
        )));
      }
      *last = Some(Instant::now());
    }
    self.refresh().await?;
    self
      .cached(kid)
      .await
      .0
      .ok_or_else(|| IdentityError::Invalid(format!("unknown signing key {kid:?}")))
  }

  pub async fn verify_logout_token(
    &self,
    token: &str,
    audience: &str,
  ) -> Result<LogoutClaims, IdentityError> {
    let header = decode_header(token).map_err(|e| IdentityError::Invalid(e.to_string()))?;
    let kid = header
      .kid
      .ok_or_else(|| IdentityError::Invalid("token has no key id".into()))?;
    let key = self.key_for(&kid).await?;
    let mut validation = Validation::new(Algorithm::RS256);
    validation.leeway = 0;
    validation.set_audience(&[audience]);
    validation.set_issuer(&[&self.config.issuer]);
    validation.set_required_spec_claims(&["iss", "aud"]);
    let claims = decode::<LogoutClaims>(token, &key, &validation)
      .map(|data| data.claims)
      .map_err(|e| IdentityError::Invalid(e.to_string()))?;
    if claims.events.get(BACKCHANNEL_LOGOUT_EVENT).is_none() {
      return Err(IdentityError::Invalid(
        "not a back-channel logout token".into(),
      ));
    }
    if claims.sid.is_none() && claims.sub.is_none() {
      return Err(IdentityError::Invalid(
        "logout token names no session".into(),
      ));
    }
    Ok(claims)
  }

  pub async fn verify(&self, token: &str, audience: &str) -> Result<Claims, IdentityError> {
    let header = decode_header(token).map_err(|e| IdentityError::Invalid(e.to_string()))?;
    if header.alg != Algorithm::RS256 {
      return Err(IdentityError::Invalid(format!(
        "unsupported algorithm {:?}",
        header.alg
      )));
    }
    let kid = header
      .kid
      .ok_or_else(|| IdentityError::Invalid("token has no key id".into()))?;
    let key = self.key_for(&kid).await?;
    let mut validation = Validation::new(Algorithm::RS256);
    validation.leeway = 0;
    validation.set_audience(&[audience]);
    validation.set_issuer(&[&self.config.issuer]);
    validation.set_required_spec_claims(&["exp", "sub", "aud", "iss"]);
    decode::<Claims>(token, &key, &validation)
      .map(|data| data.claims)
      .map_err(|e| match e.kind() {
        ErrorKind::ExpiredSignature => IdentityError::Expired,
        _ => IdentityError::Invalid(e.to_string()),
      })
  }
}
