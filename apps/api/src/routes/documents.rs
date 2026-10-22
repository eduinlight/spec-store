use axum::Json;
use axum::extract::{Path, State};
use serde::Serialize;
use spec_store_core::model::DocumentKind;
use spec_store_core::store::documents::RevisionMeta;

use crate::auth::Principal;
use crate::error::{ApiError, ErrorBody};
use crate::routes::projects::address;
use crate::state::AppState;

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct RevisionOutput {
  pub id: i64,
  pub author: String,
  pub created_at: String,
  pub message: String,
  pub version: Option<String>,
}

impl From<RevisionMeta> for RevisionOutput {
  fn from(m: RevisionMeta) -> Self {
    RevisionOutput {
      id: m.id,
      author: m.author,
      created_at: m.created_at,
      message: m.message,
      version: m.version,
    }
  }
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct DocumentOutput {
  pub body: String,
  pub revision: RevisionOutput,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ConstitutionOutput {
  pub body: String,
  pub version: String,
  pub revision: RevisionOutput,
}

#[utoipa::path(
  get,
  operation_id = "documents_read",
  path = "/api/v1/projects/{org}/{project}/specs/{number}/documents/{kind}",
  tag = "documents",
  params(("org" = String, Path), ("project" = String, Path), ("number" = i64, Path), ("kind" = String, Path)),
  responses((status = 200, body = DocumentOutput), (status = 401, body = ErrorBody), (status = 404, body = ErrorBody), (status = 422, body = ErrorBody))
)]
pub async fn read(
  State(state): State<AppState>,
  principal: Principal,
  Path((org, project, number, kind)): Path<(String, String, i64, String)>,
) -> Result<Json<DocumentOutput>, ApiError> {
  let caller = principal.caller;
  let target = address(&org, &project);
  let document = state
    .blocking(move |store| {
      let kind: DocumentKind = kind.parse()?;
      store.get_document(&caller, &target, number, kind)
    })
    .await?;
  Ok(Json(DocumentOutput {
    body: document.body,
    revision: document.revision.into(),
  }))
}

#[utoipa::path(
  get,
  operation_id = "documents_constitution",
  path = "/api/v1/projects/{org}/{project}/constitution",
  tag = "documents",
  params(("org" = String, Path), ("project" = String, Path)),
  responses((status = 200, body = ConstitutionOutput), (status = 401, body = ErrorBody), (status = 404, body = ErrorBody))
)]
pub async fn constitution(
  State(state): State<AppState>,
  principal: Principal,
  Path((org, project)): Path<(String, String)>,
) -> Result<Json<ConstitutionOutput>, ApiError> {
  let caller = principal.caller;
  let target = address(&org, &project);
  let constitution = state
    .blocking(move |store| store.get_constitution(&caller, &target))
    .await?;
  Ok(Json(ConstitutionOutput {
    body: constitution.body,
    version: constitution.version,
    revision: constitution.revision.into(),
  }))
}
