use axum::Json;
use serde::Serialize;

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
