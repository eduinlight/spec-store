use axum::Json;
use axum::extract::{Path, Query, State};
use serde::{Deserialize, Serialize};
use spec_store_core::store::{AuditEntry, AuditPage};

use crate::auth::Principal;
use crate::error::ApiError;
use crate::routes::projects::address;
use crate::state::AppState;

pub const PAGE_SIZE: usize = 50;

#[derive(Debug, Deserialize, Default)]
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

#[derive(Debug, Serialize)]
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

#[derive(Debug, Serialize)]
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
