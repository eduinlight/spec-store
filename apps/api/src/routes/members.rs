use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use spec_store_core::model::OrgRole;

use crate::auth::Principal;
use crate::error::ApiError;
use crate::state::AppState;

#[derive(Debug, Serialize)]
pub struct MemberOutput {
  pub handle: String,
  pub name: String,
  pub role: String,
}

#[derive(Debug, Serialize)]
pub struct MembersOutput {
  pub members: Vec<MemberOutput>,
}

#[derive(Debug, Deserialize)]
pub struct RoleInput {
  pub role: String,
}

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
