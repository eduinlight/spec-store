use axum::body::Body;
use axum::extract::{Request, State};
use axum::http::StatusCode;
use axum::http::header::{AUTHORIZATION, WWW_AUTHENTICATE};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use spec_store_core::identity::IdentityError;

use crate::state::AppState;

fn bearer(request: &Request<Body>) -> Option<String> {
  request
    .headers()
    .get(AUTHORIZATION)
    .and_then(|value| value.to_str().ok())
    .and_then(|value| value.strip_prefix("Bearer "))
    .map(str::trim)
    .filter(|token| !token.is_empty())
    .map(str::to_string)
}

pub fn challenge(state: &AppState, invalid_token: bool) -> String {
  let mut header = String::from("Bearer ");
  if invalid_token {
    header.push_str("error=\"invalid_token\", ");
  }
  header.push_str(&format!(
    "resource_metadata=\"{}\", scope=\"{}\"",
    state.config.metadata_url(),
    state.config.scopes.join(" ")
  ));
  header
}

pub fn unauthorized(state: &AppState, invalid_token: bool) -> Response {
  (
    StatusCode::UNAUTHORIZED,
    [(WWW_AUTHENTICATE, challenge(state, invalid_token))],
    "authentication required",
  )
    .into_response()
}

pub async fn require_bearer(
  State(state): State<AppState>,
  mut request: Request<Body>,
  next: Next,
) -> Response {
  let Some(token) = bearer(&request) else {
    return unauthorized(&state, false);
  };
  let claims = match state.verifier.verify(&token, &state.config.resource).await {
    Ok(claims) => claims,
    Err(IdentityError::Unavailable(detail)) => {
      tracing::warn!(%detail, "identity provider unavailable");
      return (
        StatusCode::SERVICE_UNAVAILABLE,
        "identity provider unavailable",
      )
        .into_response();
    }
    Err(error) => {
      tracing::debug!(%error, "rejected token");
      return unauthorized(&state, true);
    }
  };
  let identity = claims.identity();
  let is_operator = claims.is_operator();
  let store = state.store.clone();
  let caller = tokio::task::spawn_blocking(move || store.caller_for(&identity, is_operator)).await;
  match caller {
    Ok(Ok(caller)) => {
      request.extensions_mut().insert(caller);
      next.run(request).await
    }
    Ok(Err(error)) => {
      tracing::error!(%error, "cannot resolve caller");
      (StatusCode::INTERNAL_SERVER_ERROR, "internal error").into_response()
    }
    Err(error) => {
      tracing::error!(%error, "caller task failed");
      (StatusCode::INTERNAL_SERVER_ERROR, "internal error").into_response()
    }
  }
}
