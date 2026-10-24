use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use sdd_vault_core::model::Role;
use sdd_vault_core::store::invitations::ProjectInvitation;
use serde::{Deserialize, Serialize};

use crate::auth::Principal;
use crate::error::{ApiError, ErrorBody};
use crate::routes::invitations::{InvitationOutput, InvitationsOutput};
use crate::routes::projects::address;
use crate::state::AppState;

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct ProjectInvitationInput {
  pub email: String,
  pub role: String,
  #[serde(default)]
  pub language: Option<String>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ProjectInviteOutput {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub added: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub invited: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub id: Option<i64>,
}

fn language(input: Option<String>) -> String {
  match input.as_deref() {
    Some("es") => "es".into(),
    _ => "en".into(),
  }
}

#[utoipa::path(
  post,
  operation_id = "project_invitations_create",
  path = "/api/v1/projects/{org}/{project}/members/invitations",
  tag = "projects",
  params(("org" = String, Path), ("project" = String, Path)),
  request_body = ProjectInvitationInput,
  responses((status = 200, description = "An organization member received the role at once", body = ProjectInviteOutput), (status = 201, description = "A pending invitation was created", body = ProjectInviteOutput), (status = 401, body = ErrorBody), (status = 403, body = ErrorBody), (status = 404, body = ErrorBody), (status = 422, body = ErrorBody))
)]
pub async fn create(
  State(state): State<AppState>,
  principal: Principal,
  Path((org, project)): Path<(String, String)>,
  Json(body): Json<ProjectInvitationInput>,
) -> Result<(StatusCode, Json<ProjectInviteOutput>), ApiError> {
  let caller = principal.caller;
  let target = address(&org, &project);
  let base = state.config.invitation_base_url();
  let outcome = state
    .blocking(move |store| {
      let role: Role = body.role.parse()?;
      store.invite_to_project(
        &caller,
        &target,
        &body.email,
        role,
        &language(body.language),
        &base,
      )
    })
    .await?;
  Ok(match outcome {
    ProjectInvitation::Added { handle } => (
      StatusCode::OK,
      Json(ProjectInviteOutput {
        added: Some(handle),
        invited: None,
        id: None,
      }),
    ),
    ProjectInvitation::Invited(created) => (
      StatusCode::CREATED,
      Json(ProjectInviteOutput {
        added: None,
        invited: Some(created.email),
        id: Some(created.id),
      }),
    ),
  })
}

#[utoipa::path(
  get,
  operation_id = "project_invitations_list",
  path = "/api/v1/projects/{org}/{project}/members/invitations",
  tag = "projects",
  params(("org" = String, Path), ("project" = String, Path)),
  responses((status = 200, body = InvitationsOutput), (status = 401, body = ErrorBody), (status = 403, body = ErrorBody), (status = 404, body = ErrorBody), (status = 422, body = ErrorBody))
)]
pub async fn list(
  State(state): State<AppState>,
  principal: Principal,
  Path((org, project)): Path<(String, String)>,
) -> Result<Json<InvitationsOutput>, ApiError> {
  let caller = principal.caller;
  let target = address(&org, &project);
  let invitations = state
    .blocking(move |store| store.list_project_invitations(&caller, &target))
    .await?;
  Ok(Json(InvitationsOutput {
    invitations: invitations
      .into_iter()
      .map(InvitationOutput::from)
      .collect(),
  }))
}

#[utoipa::path(
  post,
  operation_id = "project_invitations_resend",
  path = "/api/v1/projects/{org}/{project}/members/invitations/{id}/resend",
  tag = "projects",
  params(("org" = String, Path), ("project" = String, Path), ("id" = i64, Path)),
  responses((status = 204), (status = 401, body = ErrorBody), (status = 403, body = ErrorBody), (status = 404, body = ErrorBody), (status = 422, body = ErrorBody))
)]
pub async fn resend(
  State(state): State<AppState>,
  principal: Principal,
  Path((org, project, id)): Path<(String, String, i64)>,
) -> Result<StatusCode, ApiError> {
  let caller = principal.caller;
  let target = address(&org, &project);
  let base = state.config.invitation_base_url();
  state
    .blocking(move |store| store.resend_project_invitation(&caller, &target, id, &base))
    .await?;
  Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
  delete,
  operation_id = "project_invitations_revoke",
  path = "/api/v1/projects/{org}/{project}/members/invitations/{id}",
  tag = "projects",
  params(("org" = String, Path), ("project" = String, Path), ("id" = i64, Path)),
  responses((status = 204), (status = 401, body = ErrorBody), (status = 403, body = ErrorBody), (status = 404, body = ErrorBody), (status = 422, body = ErrorBody))
)]
pub async fn revoke(
  State(state): State<AppState>,
  principal: Principal,
  Path((org, project, id)): Path<(String, String, i64)>,
) -> Result<StatusCode, ApiError> {
  let caller = principal.caller;
  let target = address(&org, &project);
  state
    .blocking(move |store| store.revoke_project_invitation(&caller, &target, id))
    .await?;
  Ok(StatusCode::NO_CONTENT)
}
