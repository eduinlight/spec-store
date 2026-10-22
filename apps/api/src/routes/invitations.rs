use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use spec_store_core::store::invitations::{InvitationInfo, InvitationPreview};

use crate::auth::Principal;
use crate::error::ApiError;
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct NewInvitationInput {
  pub email: String,
  #[serde(default)]
  pub language: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CreatedInvitationOutput {
  pub id: i64,
  pub email: String,
  pub expires_at: String,
}

#[derive(Debug, Serialize)]
pub struct InvitationOutput {
  pub id: i64,
  pub email: String,
  pub state: String,
  pub inviter: String,
  pub expires_at: String,
  pub created_at: String,
}

impl From<InvitationInfo> for InvitationOutput {
  fn from(i: InvitationInfo) -> Self {
    InvitationOutput {
      id: i.id,
      email: i.email,
      state: i.state.as_str().to_string(),
      inviter: i.inviter,
      expires_at: i.expires_at,
      created_at: i.created_at,
    }
  }
}

#[derive(Debug, Serialize)]
pub struct InvitationsOutput {
  pub invitations: Vec<InvitationOutput>,
}

#[derive(Debug, Serialize)]
pub struct PreviewOutput {
  pub id: i64,
  pub organization: String,
  pub organization_name: String,
  pub inviter: String,
  pub email: String,
  pub state: String,
  pub expires_at: String,
}

impl From<InvitationPreview> for PreviewOutput {
  fn from(p: InvitationPreview) -> Self {
    PreviewOutput {
      id: p.id,
      organization: p.organization,
      organization_name: p.organization_name,
      inviter: p.inviter,
      email: p.email,
      state: p.state.as_str().to_string(),
      expires_at: p.expires_at,
    }
  }
}

#[derive(Debug, Serialize)]
pub struct PreviewsOutput {
  pub invitations: Vec<PreviewOutput>,
}

fn language(input: Option<String>) -> String {
  match input.as_deref() {
    Some("es") => "es".into(),
    _ => "en".into(),
  }
}

pub async fn create(
  State(state): State<AppState>,
  principal: Principal,
  Path(org): Path<String>,
  Json(body): Json<NewInvitationInput>,
) -> Result<(StatusCode, Json<CreatedInvitationOutput>), ApiError> {
  let caller = principal.caller;
  let base = state.config.invitation_base_url();
  let created = state
    .blocking(move |store| {
      store.create_invitation(&caller, &org, &body.email, &language(body.language), &base)
    })
    .await?;
  Ok((
    StatusCode::CREATED,
    Json(CreatedInvitationOutput {
      id: created.id,
      email: created.email,
      expires_at: created.expires_at,
    }),
  ))
}

pub async fn list(
  State(state): State<AppState>,
  principal: Principal,
  Path(org): Path<String>,
) -> Result<Json<InvitationsOutput>, ApiError> {
  let caller = principal.caller;
  let invitations = state
    .blocking(move |store| store.list_invitations(&caller, &org))
    .await?;
  Ok(Json(InvitationsOutput {
    invitations: invitations
      .into_iter()
      .map(InvitationOutput::from)
      .collect(),
  }))
}

pub async fn resend(
  State(state): State<AppState>,
  principal: Principal,
  Path((org, id)): Path<(String, i64)>,
) -> Result<StatusCode, ApiError> {
  let caller = principal.caller;
  let base = state.config.invitation_base_url();
  state
    .blocking(move |store| store.resend_invitation(&caller, &org, id, &base))
    .await?;
  Ok(StatusCode::NO_CONTENT)
}

pub async fn revoke(
  State(state): State<AppState>,
  principal: Principal,
  Path((org, id)): Path<(String, i64)>,
) -> Result<StatusCode, ApiError> {
  let caller = principal.caller;
  state
    .blocking(move |store| store.revoke_invitation(&caller, &org, id))
    .await?;
  Ok(StatusCode::NO_CONTENT)
}

pub async fn mine(
  State(state): State<AppState>,
  principal: Principal,
) -> Result<Json<PreviewsOutput>, ApiError> {
  let caller = principal.caller;
  let previews = state
    .blocking(move |store| store.my_invitations(&caller))
    .await?;
  Ok(Json(PreviewsOutput {
    invitations: previews.into_iter().map(PreviewOutput::from).collect(),
  }))
}

pub async fn preview(
  State(state): State<AppState>,
  _principal: Principal,
  Path(token): Path<String>,
) -> Result<Json<PreviewOutput>, ApiError> {
  let preview = state
    .blocking(move |store| store.invitation_preview(&token))
    .await?;
  Ok(Json(preview.into()))
}

pub async fn accept(
  State(state): State<AppState>,
  principal: Principal,
  Path(token): Path<String>,
) -> Result<StatusCode, ApiError> {
  let caller = principal.caller;
  let verified = principal.email_verified;
  state
    .blocking(move |store| store.accept_invitation(&caller, &token, verified))
    .await?;
  Ok(StatusCode::NO_CONTENT)
}

pub async fn decline(
  State(state): State<AppState>,
  principal: Principal,
  Path(token): Path<String>,
) -> Result<StatusCode, ApiError> {
  let caller = principal.caller;
  state
    .blocking(move |store| store.decline_invitation(&caller, &token))
    .await?;
  Ok(StatusCode::NO_CONTENT)
}

pub async fn accept_pending(
  State(state): State<AppState>,
  principal: Principal,
  Path(id): Path<i64>,
) -> Result<StatusCode, ApiError> {
  let caller = principal.caller;
  let verified = principal.email_verified;
  state
    .blocking(move |store| store.accept_pending_invitation(&caller, id, verified))
    .await?;
  Ok(StatusCode::NO_CONTENT)
}

pub async fn decline_pending(
  State(state): State<AppState>,
  principal: Principal,
  Path(id): Path<i64>,
) -> Result<StatusCode, ApiError> {
  let caller = principal.caller;
  state
    .blocking(move |store| store.decline_pending_invitation(&caller, id))
    .await?;
  Ok(StatusCode::NO_CONTENT)
}
