use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use spec_store_core::model::OrgRole;

use crate::auth::Principal;
use crate::error::{ApiError, ErrorBody};
use crate::state::AppState;

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct MemberOutput {
  pub handle: String,
  pub name: String,
  pub role: String,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct MembersOutput {
  pub members: Vec<MemberOutput>,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct RoleInput {
  pub role: String,
}

#[utoipa::path(
  get,
  operation_id = "members_list",
  path = "/api/v1/orgs/{org}/members",
  tag = "organizations",
  params(("org" = String, Path)),
  responses((status = 200, body = MembersOutput), (status = 401, body = ErrorBody), (status = 403, body = ErrorBody), (status = 404, body = ErrorBody), (status = 422, body = ErrorBody))
)]
pub async fn list(
  State(state): State<AppState>,
  principal: Principal,
  Path(org): Path<String>,
) -> Result<Json<MembersOutput>, ApiError> {
  let caller = principal.caller;
  let members = state
    .blocking(move |store| store.list_organization_members(&caller, &org))
    .await?;
  Ok(Json(MembersOutput {
    members: members
      .into_iter()
      .map(|m| MemberOutput {
        handle: m.handle,
        name: m.name,
        role: m.role.as_str().to_string(),
      })
      .collect(),
  }))
}

#[utoipa::path(
  put,
  operation_id = "members_set_role",
  path = "/api/v1/orgs/{org}/members/{handle}",
  tag = "organizations",
  params(("org" = String, Path), ("handle" = String, Path)),
  request_body = RoleInput,
  responses((status = 204), (status = 401, body = ErrorBody), (status = 403, body = ErrorBody), (status = 404, body = ErrorBody), (status = 422, body = ErrorBody))
)]
pub async fn set_role(
  State(state): State<AppState>,
  principal: Principal,
  Path((org, handle)): Path<(String, String)>,
  Json(body): Json<RoleInput>,
) -> Result<StatusCode, ApiError> {
  let caller = principal.caller;
  state
    .blocking(move |store| {
      let role: OrgRole = body.role.parse()?;
      store.set_organization_member(&caller, &org, &handle, role)
    })
    .await?;
  Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
  delete,
  operation_id = "members_remove",
  path = "/api/v1/orgs/{org}/members/{handle}",
  tag = "organizations",
  params(("org" = String, Path), ("handle" = String, Path)),
  responses((status = 204), (status = 401, body = ErrorBody), (status = 403, body = ErrorBody), (status = 404, body = ErrorBody), (status = 422, body = ErrorBody))
)]
pub async fn remove(
  State(state): State<AppState>,
  principal: Principal,
  Path((org, handle)): Path<(String, String)>,
) -> Result<StatusCode, ApiError> {
  let caller = principal.caller;
  state
    .blocking(move |store| store.remove_organization_member(&caller, &org, &handle))
    .await?;
  Ok(StatusCode::NO_CONTENT)
}
