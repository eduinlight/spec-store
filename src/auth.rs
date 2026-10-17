use base64::Engine;
use sha2::{Digest, Sha256};

pub const TOKEN_PREFIX: &str = "sst_";

pub fn generate_token() -> String {
  let mut bytes = [0u8; 32];
  rand::fill(&mut bytes);
  format!(
    "{TOKEN_PREFIX}{}",
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
  )
}

pub fn hash_token(token: &str) -> String {
  Sha256::digest(token.as_bytes())
    .iter()
    .map(|b| format!("{b:02x}"))
    .collect()
}

use axum::body::Body;
use axum::extract::{Request, State};
use axum::http::StatusCode;
use axum::http::header::{AUTHORIZATION, WWW_AUTHENTICATE};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

use crate::model::Caller;
use crate::store::Store;

pub async fn require_bearer(
  State(store): State<Store>,
  mut request: Request<Body>,
  next: Next,
) -> Response {
  let token = request
    .headers()
    .get(AUTHORIZATION)
    .and_then(|value| value.to_str().ok())
    .and_then(|value| value.strip_prefix("Bearer "))
    .map(str::trim)
    .filter(|token| !token.is_empty());
  let caller: Option<Caller> = token.and_then(|token| store.authenticate(token).ok());
  match caller {
    Some(caller) => {
      request.extensions_mut().insert(caller);
      next.run(request).await
    }
    None => unauthorized(),
  }
}

fn unauthorized() -> Response {
  (
    StatusCode::UNAUTHORIZED,
    [(WWW_AUTHENTICATE, "Bearer realm=\"spec-store\"")],
    "authentication required",
  )
    .into_response()
}
