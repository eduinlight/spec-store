use axum::extract::FromRequestParts;
use axum::http::header::AUTHORIZATION;
use axum::http::request::Parts;
use spec_store_core::model::Caller;
use spec_store_core::store::users::Identity;

use crate::error::ApiError;
use crate::state::AppState;

#[derive(Debug, Clone)]
pub struct Principal {
  pub caller: Caller,
  pub identity: Identity,
  pub email_verified: bool,
  pub via_session: bool,
}

pub fn bearer_token(parts: &Parts) -> Option<&str> {
  parts
    .headers
    .get(AUTHORIZATION)
    .and_then(|value| value.to_str().ok())
    .and_then(|value| value.strip_prefix("Bearer "))
    .map(str::trim)
    .filter(|token| !token.is_empty())
}

impl FromRequestParts<AppState> for Principal {
  type Rejection = ApiError;

  async fn from_request_parts(
    parts: &mut Parts,
    state: &AppState,
  ) -> Result<Self, Self::Rejection> {
    if let Some(principal) = parts.extensions.get::<Principal>() {
      return Ok(principal.clone());
    }
    let token = bearer_token(parts).ok_or(ApiError::Unauthenticated)?;
    let claims = state.verifier.verify(token, &state.config.audience).await?;
    let identity = claims.identity();
    let is_operator = claims.is_operator();
    let for_store = identity.clone();
    let caller = state
      .blocking(move |store| store.caller_for(&for_store, is_operator))
      .await?;
    let principal = Principal {
      caller,
      identity,
      email_verified: claims.email_verified,
      via_session: false,
    };
    parts.extensions.insert(principal.clone());
    Ok(principal)
  }
}
