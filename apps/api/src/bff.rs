use axum::Form;
use axum::Json;
use axum::extract::{Query, State};
use axum::http::header::{LOCATION, SET_COOKIE};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use serde::Deserialize;
use serde_json::json;
use spec_store_core::keycloak::{AuthorizeRequest, KeycloakError, pkce_pair};
use spec_store_core::model::StoreError;
use spec_store_core::secrets::random_secret;
use spec_store_core::store::sessions::NewSession;

use crate::cookies::{
  REQUEST_HEADER, SESSION_COOKIE, cleared_session_cookie, cookie, language, session_cookie,
};
use crate::error::ApiError;
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct LoginQuery {
  pub return_to: Option<String>,
  pub action: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CallbackQuery {
  pub code: Option<String>,
  pub state: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct BackchannelForm {
  pub logout_token: String,
}

pub fn safe_return_to(target: Option<&str>) -> String {
  match target {
    Some(path)
      if path.starts_with('/')
        && !path.starts_with("//")
        && !path.starts_with("/\\")
        && !path.chars().any(|c| c.is_control()) =>
    {
      path.to_string()
    }
    _ => "/".to_string(),
  }
}

fn redirect(location: &str) -> Response {
  (StatusCode::FOUND, [(LOCATION, location.to_string())]).into_response()
}

fn identity_failure(error: KeycloakError) -> ApiError {
  match error {
    KeycloakError::InvalidGrant => ApiError::Unauthenticated,
    other => ApiError::Store(StoreError::from(other)),
  }
}

pub async fn login(
  State(state): State<AppState>,
  headers: HeaderMap,
  Query(query): Query<LoginQuery>,
) -> Result<Response, ApiError> {
  let return_to = safe_return_to(query.return_to.as_deref());
  let login_state = random_secret("");
  let (verifier, challenge) = pkce_pair();
  let sealed = state.sealer.seal(verifier.as_bytes());
  let stored_state = login_state.clone();
  state
    .blocking(move |store| store.begin_login(&stored_state, &sealed, &return_to))
    .await?;
  let callback = state.config.callback_url();
  let url = state.oidc.authorize_url(&AuthorizeRequest {
    redirect_uri: &callback,
    state: &login_state,
    code_challenge: &challenge,
    ui_locales: language(&headers),
    register: query.action.as_deref() == Some("register"),
  });
  Ok(redirect(&url))
}

pub async fn callback(
  State(state): State<AppState>,
  Query(query): Query<CallbackQuery>,
) -> Result<Response, ApiError> {
  let (Some(code), Some(login_state)) = (query.code, query.state) else {
    return Err(ApiError::Unauthenticated);
  };
  let attempt = state
    .blocking(move |store| store.finish_login(&login_state))
    .await
    .map_err(|_| ApiError::Unauthenticated)?;
  let verifier = state
    .sealer
    .open(&attempt.pkce_verifier_sealed)
    .map_err(|_| ApiError::Unauthenticated)?;
  let verifier = String::from_utf8(verifier).map_err(|_| ApiError::Unauthenticated)?;
  let tokens = state
    .oidc
    .exchange_code(&code, &state.config.callback_url(), &verifier)
    .await
    .map_err(identity_failure)?;
  let claims = state
    .verifier
    .verify(&tokens.access_token, &state.config.audience)
    .await?;
  let refresh_token = tokens.refresh_token.ok_or(ApiError::Unauthenticated)?;
  let id_token = tokens.id_token.unwrap_or_default();
  let identity = claims.identity();
  let is_operator = claims.is_operator();
  let session = NewSession {
    user_id: 0,
    identity_session_id: claims.sid.clone(),
    refresh_token_sealed: state.sealer.seal(refresh_token.as_bytes()),
    id_token_sealed: state.sealer.seal(id_token.as_bytes()),
    access_expires_in_seconds: tokens.expires_in,
    is_operator,
  };
  let secret = state
    .blocking(move |store| {
      let caller = store.caller_for(&identity, is_operator)?;
      store.create_session(&NewSession {
        user_id: caller.user_id,
        ..session
      })
    })
    .await?;
  Ok(
    (
      StatusCode::FOUND,
      [
        (LOCATION, attempt.return_to),
        (SET_COOKIE, session_cookie(&secret)),
      ],
    )
      .into_response(),
  )
}

pub async fn logout(
  State(state): State<AppState>,
  headers: HeaderMap,
) -> Result<Response, ApiError> {
  if headers.get(REQUEST_HEADER).and_then(|v| v.to_str().ok()) != Some("1") {
    return Err(ApiError::Store(StoreError::Forbidden));
  }
  let secret = cookie(&headers, SESSION_COOKIE).ok_or(ApiError::Unauthenticated)?;
  let lookup = secret.clone();
  let session = state
    .blocking(move |store| store.session(&lookup))
    .await
    .map_err(|_| ApiError::Unauthenticated)?;
  state
    .blocking(move |store| store.end_session(&secret))
    .await?;
  let id_token = state
    .sealer
    .open(&session.id_token_sealed)
    .ok()
    .and_then(|bytes| String::from_utf8(bytes).ok())
    .unwrap_or_default();
  let url = state
    .oidc
    .end_session_url(&id_token, &state.config.landing_url, language(&headers));
  Ok(
    (
      StatusCode::OK,
      [(SET_COOKIE, cleared_session_cookie())],
      Json(json!({ "logout_url": url })),
    )
      .into_response(),
  )
}

pub async fn backchannel_logout(
  State(state): State<AppState>,
  Form(form): Form<BackchannelForm>,
) -> Response {
  let claims = match state
    .verifier
    .verify_logout_token(&form.logout_token, state.oidc.client_id())
    .await
  {
    Ok(claims) => claims,
    Err(error) => {
      tracing::warn!(%error, "refused back-channel logout token");
      return (
        StatusCode::BAD_REQUEST,
        Json(json!({ "error": "invalid_request" })),
      )
        .into_response();
    }
  };
  let Some(sid) = claims.sid else {
    return StatusCode::OK.into_response();
  };
  match state
    .blocking(move |store| store.end_identity_session(&sid))
    .await
  {
    Ok(ended) => {
      tracing::info!(ended, "back-channel logout ended sessions");
      StatusCode::OK.into_response()
    }
    Err(error) => error.into_response(),
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn return_targets_stay_on_the_app() {
    assert_eq!(safe_return_to(Some("/projects/acme")), "/projects/acme");
    for unsafe_target in ["https://evil", "//evil", "/\\evil", "javascript:x", ""] {
      assert_eq!(safe_return_to(Some(unsafe_target)), "/", "{unsafe_target}");
    }
    assert_eq!(safe_return_to(None), "/");
  }
}
