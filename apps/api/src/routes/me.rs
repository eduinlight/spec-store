use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::http::header::SET_COOKIE;
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};
use spec_store_core::model::StoreError;

use crate::cookies::language_cookie;
use crate::error::ApiError;
use crate::state::AppState;

use crate::auth::Principal;
use crate::error::ErrorBody;

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct Me {
  pub handle: String,
  pub email: String,
  pub name: String,
  pub is_operator: bool,
}

#[utoipa::path(
  get,
  path = "/api/v1/me",
  tag = "me",
  responses((status = 200, body = Me), (status = 401, body = ErrorBody), (status = 403, body = ErrorBody), (status = 404, body = ErrorBody), (status = 422, body = ErrorBody))
)]
pub async fn me(principal: Principal) -> Json<Me> {
  Json(Me {
    handle: principal.caller.handle,
    email: principal.identity.email,
    name: principal.identity.name,
    is_operator: principal.caller.is_operator,
  })
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct LanguageInput {
  pub language: String,
}

#[utoipa::path(
  put,
  path = "/api/v1/me/language",
  tag = "me",
  request_body = LanguageInput,
  responses((status = 204), (status = 401, body = ErrorBody), (status = 422, body = ErrorBody))
)]
pub async fn set_language(
  State(state): State<AppState>,
  principal: Principal,
  Json(body): Json<LanguageInput>,
) -> Result<Response, ApiError> {
  if !matches!(body.language.as_str(), "en" | "es") {
    return Err(ApiError::Store(StoreError::Validation(format!(
      "unsupported language {:?}; valid languages: en, es",
      body.language
    ))));
  }
  if let (Some(admin), false) = (&state.keycloak_admin, principal.identity.subject.is_empty())
    && let Err(error) = admin
      .set_locale(&principal.identity.subject, &body.language)
      .await
  {
    tracing::warn!(%error, "could not store the language in the identity provider");
  }
  Ok(
    (
      StatusCode::NO_CONTENT,
      [(
        SET_COOKIE,
        language_cookie(&body.language, &state.config.app_url),
      )],
    )
      .into_response(),
  )
}
