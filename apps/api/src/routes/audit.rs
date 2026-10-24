use axum::Json;
use axum::extract::{Path, Query, State};
use sdd_vault_core::store::{AuditEntry, AuditPage};
use serde::{Deserialize, Serialize};

use crate::auth::Principal;
use crate::error::{ApiError, ErrorBody};
use crate::routes::projects::address;
use crate::state::AppState;

pub const PAGE_SIZE: usize = 50;

#[derive(Debug, Deserialize, Default, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub struct AuditQuery {
  pub project: Option<String>,
  pub org: Option<String>,
  pub before: Option<i64>,
  pub limit: Option<usize>,
}

impl AuditQuery {
  fn limit(&self) -> usize {
    self.limit.unwrap_or(PAGE_SIZE).clamp(1, PAGE_SIZE)
  }
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct EntryOutput {
  pub id: i64,
  pub actor: String,
  pub organization: Option<String>,
  pub project: Option<String>,
  pub action: String,
  pub target: String,
  pub created_at: String,
}

impl From<AuditEntry> for EntryOutput {
  fn from(e: AuditEntry) -> Self {
    EntryOutput {
      id: e.id,
      actor: e.actor,
      organization: e.organization,
      project: e.project,
      action: e.action,
      target: e.target,
      created_at: e.created_at,
    }
  }
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct PageOutput {
  pub entries: Vec<EntryOutput>,
  pub has_more: bool,
}

impl From<AuditPage> for PageOutput {
  fn from(p: AuditPage) -> Self {
    PageOutput {
      entries: p.entries.into_iter().map(EntryOutput::from).collect(),
      has_more: p.has_more,
    }
  }
}

#[utoipa::path(
  get,
  operation_id = "audit_organization",
  path = "/api/v1/orgs/{org}/audit",
  tag = "audit",
  params(("org" = String, Path), AuditQuery),
  responses((status = 200, body = PageOutput), (status = 401, body = ErrorBody), (status = 403, body = ErrorBody), (status = 404, body = ErrorBody), (status = 422, body = ErrorBody))
)]
pub async fn organization(
  State(state): State<AppState>,
  principal: Principal,
  Path(org): Path<String>,
  Query(query): Query<AuditQuery>,
) -> Result<Json<PageOutput>, ApiError> {
  let caller = principal.caller;
  let limit = query.limit();
  let page = state
    .blocking(move |store| {
      store.list_organization_audit(&caller, &org, query.project.as_deref(), query.before, limit)
    })
    .await?;
  Ok(Json(page.into()))
}

#[utoipa::path(
  get,
  operation_id = "audit_global",
  path = "/api/v1/audit",
  tag = "audit",
  params(AuditQuery),
  responses((status = 200, body = PageOutput), (status = 401, body = ErrorBody), (status = 403, body = ErrorBody), (status = 404, body = ErrorBody), (status = 422, body = ErrorBody))
)]
pub async fn global(
  State(state): State<AppState>,
  principal: Principal,
  Query(query): Query<AuditQuery>,
) -> Result<Json<PageOutput>, ApiError> {
  let caller = principal.caller;
  let limit = query.limit();
  let page = state
    .blocking(move |store| {
      store.list_global_audit(&caller, query.org.as_deref(), query.before, limit)
    })
    .await?;
  Ok(Json(page.into()))
}

#[utoipa::path(
  get,
  operation_id = "audit_project",
  path = "/api/v1/projects/{org}/{project}/audit",
  tag = "audit",
  params(("org" = String, Path), ("project" = String, Path), AuditQuery),
  responses((status = 200, body = PageOutput), (status = 401, body = ErrorBody), (status = 403, body = ErrorBody), (status = 404, body = ErrorBody), (status = 422, body = ErrorBody))
)]
pub async fn project(
  State(state): State<AppState>,
  principal: Principal,
  Path((org, project)): Path<(String, String)>,
  Query(query): Query<AuditQuery>,
) -> Result<Json<PageOutput>, ApiError> {
  let caller = principal.caller;
  let limit = query.limit();
  let target = address(&org, &project);
  let page = state
    .blocking(move |store| store.list_audit_page(&caller, Some(&target), query.before, limit))
    .await?;
  Ok(Json(page.into()))
}
