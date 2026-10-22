use reqwest::{Method, StatusCode};
use serde_json::Value;

use crate::credentials::{CredentialStore, Credentials};
use crate::login::{Endpoints, LoginError, now, refresh};

pub const EXPIRY_MARGIN_SECONDS: i64 = 30;

#[derive(Debug, thiserror::Error)]
pub enum ClientError {
  #[error("not signed in; run spec-store login")]
  NotSignedIn,
  #[error("{0}")]
  Login(#[from] LoginError),
  #[error("{0}")]
  Credentials(#[from] crate::credentials::CredentialsError),
  #[error("cannot reach the spec-store API: {0}")]
  Unreachable(String),
  #[error("{message}")]
  Api {
    status: u16,
    code: String,
    message: String,
  },
}

pub struct ApiClient {
  http: reqwest::Client,
  api: String,
  endpoints: Endpoints,
  store: CredentialStore,
  credentials: Credentials,
}

impl ApiClient {
  pub fn http() -> reqwest::Client {
    reqwest::Client::builder()
      .timeout(std::time::Duration::from_secs(120))
      .build()
      .expect("http client")
  }

  pub fn connect(
    api: &str,
    endpoints: Endpoints,
    store: CredentialStore,
  ) -> Result<ApiClient, ClientError> {
    let credentials = store
      .load(&endpoints.issuer)?
      .ok_or(ClientError::NotSignedIn)?;
    Ok(ApiClient {
      http: Self::http(),
      api: api.trim_end_matches('/').to_string(),
      endpoints,
      store,
      credentials,
    })
  }

  async fn renew(&mut self) -> Result<(), ClientError> {
    let tokens = match refresh(&self.http, &self.endpoints, &self.credentials.refresh_token).await {
      Ok(tokens) => tokens,
      Err(LoginError::SessionEnded) => {
        self.store.remove(&self.endpoints.issuer)?;
        return Err(ClientError::Login(LoginError::SessionEnded));
      }
      Err(error) => return Err(error.into()),
    };
    self.credentials.access_token = tokens.access_token;
    if let Some(refresh_token) = tokens.refresh_token {
      self.credentials.refresh_token = refresh_token;
    }
    self.credentials.expires_at = now() + tokens.expires_in;
    self.store.save(&self.credentials)?;
    Ok(())
  }

  async fn send_once(
    &self,
    method: Method,
    path: &str,
    body: Option<&Value>,
  ) -> Result<reqwest::Response, ClientError> {
    let mut request = self
      .http
      .request(method, format!("{}{path}", self.api))
      .bearer_auth(&self.credentials.access_token);
    if let Some(body) = body {
      request = request.json(body);
    }
    request
      .send()
      .await
      .map_err(|e| ClientError::Unreachable(e.to_string()))
  }

  pub async fn request(
    &mut self,
    method: Method,
    path: &str,
    body: Option<&Value>,
  ) -> Result<Value, ClientError> {
    if self.credentials.expires_at - EXPIRY_MARGIN_SECONDS <= now() {
      self.renew().await?;
    }
    let mut response = self.send_once(method.clone(), path, body).await?;
    if response.status() == StatusCode::UNAUTHORIZED {
      self.renew().await?;
      response = self.send_once(method, path, body).await?;
    }
    let status = response.status();
    let text = response
      .text()
      .await
      .map_err(|e| ClientError::Unreachable(e.to_string()))?;
    if status.is_success() {
      if text.is_empty() {
        return Ok(Value::Null);
      }
      return serde_json::from_str(&text).map_err(|e| ClientError::Unreachable(e.to_string()));
    }
    let parsed: Value = serde_json::from_str(&text).unwrap_or(Value::Null);
    Err(ClientError::Api {
      status: status.as_u16(),
      code: parsed["code"].as_str().unwrap_or("error").to_string(),
      message: parsed["message"]
        .as_str()
        .map(str::to_string)
        .unwrap_or_else(|| format!("the API answered {status}")),
    })
  }

  pub async fn get(&mut self, path: &str) -> Result<Value, ClientError> {
    self.request(Method::GET, path, None).await
  }

  pub async fn post(&mut self, path: &str, body: &Value) -> Result<Value, ClientError> {
    self.request(Method::POST, path, Some(body)).await
  }

  pub async fn put(&mut self, path: &str, body: &Value) -> Result<Value, ClientError> {
    self.request(Method::PUT, path, Some(body)).await
  }

  pub async fn delete(&mut self, path: &str) -> Result<Value, ClientError> {
    self.request(Method::DELETE, path, None).await
  }
}
