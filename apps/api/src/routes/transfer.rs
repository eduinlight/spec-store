use axum::Json;
use axum::extract::{Path, Query, State};
use sdd_vault_core::model::StoreError;
use sdd_vault_core::store::transfer::{ImportOptions, ImportReport, ProjectExport};
use sdd_vault_core::tree::ProjectTree;
use serde::Deserialize;

use crate::auth::Principal;
use crate::error::{ApiError, ErrorBody};
use crate::routes::projects::address;
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct ImportInput {
  pub tree: ProjectTree,
  #[serde(default)]
  pub constitution_version: Option<String>,
  #[serde(default)]
  pub dry_run: bool,
}

#[derive(Debug, Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub struct ExportQuery {
  pub spec: Option<i64>,
}

#[utoipa::path(
  post,
  operation_id = "transfer_import",
  path = "/api/v1/projects/{org}/{project}/import",
  tag = "transfer",
  params(("org" = String, Path), ("project" = String, Path)),
  request_body = Object,
  responses((status = 200, body = Object), (status = 401, body = ErrorBody), (status = 403, body = ErrorBody), (status = 404, body = ErrorBody), (status = 422, body = ErrorBody))
)]
pub async fn import(
  State(state): State<AppState>,
  principal: Principal,
  Path((org, project)): Path<(String, String)>,
  Json(body): Json<ImportInput>,
) -> Result<Json<ImportReport>, ApiError> {
  if principal.via_session {
    return Err(ApiError::Store(StoreError::Forbidden));
  }
  let caller = principal.caller;
  let target = address(&org, &project);
  let report = state
    .blocking(move |store| {
      store.import_tree(
        &caller,
        &target,
        &body.tree,
        &ImportOptions {
          constitution_version: body.constitution_version,
          dry_run: body.dry_run,
        },
      )
    })
    .await?;
  Ok(Json(report))
}

#[utoipa::path(
  get,
  operation_id = "transfer_export",
  path = "/api/v1/projects/{org}/{project}/export",
  tag = "transfer",
  params(("org" = String, Path), ("project" = String, Path), ExportQuery),
  responses((status = 200, body = Object), (status = 401, body = ErrorBody), (status = 403, body = ErrorBody), (status = 404, body = ErrorBody), (status = 422, body = ErrorBody))
)]
pub async fn export(
  State(state): State<AppState>,
  principal: Principal,
  Path((org, project)): Path<(String, String)>,
  Query(query): Query<ExportQuery>,
) -> Result<Json<ProjectExport>, ApiError> {
  if principal.via_session {
    return Err(ApiError::Store(StoreError::Forbidden));
  }
  let caller = principal.caller;
  let target = address(&org, &project);
  let export = state
    .blocking(move |store| store.export_documents(&caller, &target, query.spec))
    .await?;
  Ok(Json(export))
}
