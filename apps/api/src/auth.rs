use axum::extract::FromRequestParts;
use axum::http::Method;
use axum::http::header::AUTHORIZATION;
use axum::http::request::Parts;
use sdd_vault_core::keycloak::KeycloakError;
use sdd_vault_core::model::{Caller, StoreError};
use sdd_vault_core::store::sessions::BrowserSession;
use sdd_vault_core::store::users::Identity;

use crate::cookies::{REQUEST_HEADER, SESSION_COOKIE, cookie};
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

fn changes_state(method: &Method) -> bool {
  !matches!(*method, Method::GET | Method::HEAD | Method::OPTIONS)
}

async fn from_bearer(token: &str, state: &AppState) -> Result<Principal, ApiError> {
  let claims = state.verifier.verify(token, &state.config.audience).await?;
  let identity = claims.identity();
  let is_operator = claims.is_operator();
  let for_store = identity.clone();
  let caller = state
    .blocking(move |store| store.caller_for(&for_store, is_operator))
    .await?;
  Ok(Principal {
    caller,
    identity,
    email_verified: claims.email_verified,
    via_session: false,
  })
}

async fn renew(state: &AppState, session: &BrowserSession) -> Result<(), ApiError> {
  let id = session.id;
  if !state
    .blocking(move |store| store.claim_session_refresh(id))
    .await?
  {
    return Ok(());
  }
  let refresh_token = state
    .sealer
    .open(&session.refresh_token_sealed)
    .ok()
    .and_then(|bytes| String::from_utf8(bytes).ok())
    .ok_or(ApiError::Unauthenticated)?;
  match state.oidc.refresh(&refresh_token).await {
    Ok(tokens) => {
      let refresh_sealed = state.sealer.seal(
        tokens
          .refresh_token
          .as_deref()
          .unwrap_or(&refresh_token)
          .as_bytes(),
      );
      let id_sealed = tokens
        .id_token
        .as_deref()
        .map(|t| state.sealer.seal(t.as_bytes()));
      let expires_in = tokens.expires_in;
      state
        .blocking(move |store| {
          store.refresh_session(id, &refresh_sealed, id_sealed.as_deref(), expires_in)
        })
        .await
    }
    Err(KeycloakError::InvalidGrant) => {
      state
        .blocking(move |store| store.end_session_by_id(id))
        .await?;
      Err(ApiError::Unauthenticated)
    }
    Err(other) => Err(ApiError::Store(StoreError::from(other))),
  }
}

async fn from_session(
  parts: &Parts,
  secret: String,
  state: &AppState,
) -> Result<Principal, ApiError> {
  let session = state
    .blocking(move |store| store.session(&secret))
    .await
    .map_err(|_| ApiError::Unauthenticated)?;
  if changes_state(&parts.method)
    && parts
      .headers
      .get(REQUEST_HEADER)
      .and_then(|v| v.to_str().ok())
      != Some("1")
  {
    return Err(ApiError::Store(StoreError::Forbidden));
  }
  if session.access_expired {
    renew(state, &session).await?;
  }
  let user_id = session.caller.user_id;
  let user = state
    .blocking(move |store| store.user_by_id(user_id))
    .await?;
  Ok(Principal {
    identity: Identity {
      subject: user.subject,
      handle: user.handle,
      email: user.email,
      name: user.name,
    },
    caller: session.caller,
    email_verified: true,
    via_session: true,
  })
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
    let principal = if let Some(token) = bearer_token(parts) {
      from_bearer(token, state).await?
    } else if let Some(secret) = cookie(&parts.headers, SESSION_COOKIE) {
      from_session(parts, secret, state).await?
    } else {
      return Err(ApiError::Unauthenticated);
    };
    parts.extensions.insert(principal.clone());
    Ok(principal)
  }
}
