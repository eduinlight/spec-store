use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::header::{CONTENT_DISPOSITION, CONTENT_TYPE};
use axum::response::{IntoResponse, Response};
use sdd_vault_core::model::{AuditResult, StoreError};
use sdd_vault_core::store::views::audit_table::{
  AuditExportSelection, AuditFacets, AuditQuery as TableQuery, AuditScope, AuditTable, FacetCount,
  PAGE_SIZES,
};
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
  pub client: Option<String>,
  pub origin: Option<String>,
  pub result: String,
  pub detail: Option<String>,
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
      client: e.client,
      origin: e.origin,
      result: e.result.as_str().to_string(),
      detail: e.detail,
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

#[derive(Debug, Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
#[allow(dead_code)]
pub struct TableParams {
  /// Text matched against target, client and origin.
  pub q: Option<String>,
  /// Actor handle; repeat the parameter to select several.
  pub actor: Option<Vec<String>>,
  /// Client name; repeat the parameter to select several.
  pub client: Option<Vec<String>>,
  /// Action name; repeat the parameter to select several.
  pub action: Option<Vec<String>>,
  /// Result (ok, denied, conflict); repeat the parameter to select several.
  pub result: Option<Vec<String>>,
  /// day, week, month or all (default).
  pub period: Option<String>,
  /// newest (default) or oldest.
  pub sort: Option<String>,
  /// Page number starting at 1.
  pub page: Option<u32>,
  /// 10, 25 (default), 50 or 100.
  pub page_size: Option<u32>,
}

#[derive(Debug, Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
#[allow(dead_code)]
pub struct ExportParams {
  /// Comma-separated entry ids; when present, only these entries are exported.
  pub ids: Option<String>,
  pub q: Option<String>,
  pub actor: Option<Vec<String>>,
  pub client: Option<Vec<String>>,
  pub action: Option<Vec<String>>,
  pub result: Option<Vec<String>>,
  pub period: Option<String>,
  pub sort: Option<String>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct FacetCountOutput {
  pub value: String,
  pub count: i64,
}

impl From<FacetCount> for FacetCountOutput {
  fn from(f: FacetCount) -> Self {
    FacetCountOutput {
      value: f.value,
      count: f.count,
    }
  }
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct FacetsOutput {
  pub actors: Vec<FacetCountOutput>,
  pub clients: Vec<FacetCountOutput>,
  pub actions: Vec<FacetCountOutput>,
  pub results: Vec<FacetCountOutput>,
}

fn facet_outputs(values: Vec<FacetCount>) -> Vec<FacetCountOutput> {
  values.into_iter().map(FacetCountOutput::from).collect()
}

impl From<AuditFacets> for FacetsOutput {
  fn from(f: AuditFacets) -> Self {
    FacetsOutput {
      actors: facet_outputs(f.actors),
      clients: facet_outputs(f.clients),
      actions: facet_outputs(f.actions),
      results: facet_outputs(f.results),
    }
  }
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct TableOutput {
  pub entries: Vec<EntryOutput>,
  pub total: i64,
  pub page: u32,
  pub page_size: u32,
  pub facets: FacetsOutput,
}

impl From<AuditTable> for TableOutput {
  fn from(t: AuditTable) -> Self {
    TableOutput {
      entries: t.entries.into_iter().map(EntryOutput::from).collect(),
      total: t.total,
      page: t.page,
      page_size: t.page_size,
      facets: t.facets.into(),
    }
  }
}

fn validation(message: String) -> ApiError {
  ApiError::Store(StoreError::Validation(message))
}

struct ParsedParams {
  query: TableQuery,
  ids: Option<Vec<i64>>,
}

fn parse_params(pairs: Vec<(String, String)>) -> Result<ParsedParams, ApiError> {
  let mut query = TableQuery::default();
  let mut ids = None;
  for (name, value) in pairs {
    match name.as_str() {
      "q" => query.text = Some(value),
      "actor" => query.actors.push(value),
      "client" => query.clients.push(value),
      "action" => query.actions.push(value),
      "result" => query.results.push(value.parse::<AuditResult>()?),
      "period" => query.period = value.parse()?,
      "sort" => query.sort = value.parse()?,
      "page" => {
        query.page = value
          .parse()
          .map_err(|_| validation(format!("page must be a number, got {value:?}")))?
      }
      "page_size" => {
        query.page_size = value.parse().map_err(|_| {
          validation(format!(
            "page_size must be one of {PAGE_SIZES:?}, got {value:?}"
          ))
        })?
      }
      "ids" if !value.trim().is_empty() => {
        let parsed = value
          .split(',')
          .map(|id| {
            id.trim()
              .parse::<i64>()
              .map_err(|_| validation(format!("ids must be numbers, got {id:?}")))
          })
          .collect::<Result<Vec<i64>, ApiError>>()?;
        ids = Some(parsed);
      }
      _ => {}
    }
  }
  Ok(ParsedParams { query, ids })
}

fn csv_response(csv: String, scope: &str) -> Response {
  let date = chrono::Utc::now().format("%Y-%m-%d");
  (
    [
      (CONTENT_TYPE, "text/csv; charset=utf-8".to_string()),
      (
        CONTENT_DISPOSITION,
        format!("attachment; filename=\"audit-{scope}-{date}.csv\""),
      ),
    ],
    csv,
  )
    .into_response()
}

fn selection(parsed: ParsedParams) -> AuditExportSelection {
  match parsed.ids {
    Some(ids) => AuditExportSelection::Ids(ids),
    None => AuditExportSelection::Query(parsed.query),
  }
}

#[utoipa::path(
  get,
  operation_id = "audit_project_table",
  path = "/api/v1/projects/{org}/{project}/audit/table",
  tag = "audit",
  params(("org" = String, Path), ("project" = String, Path), TableParams),
  responses((status = 200, body = TableOutput), (status = 401, body = ErrorBody), (status = 403, body = ErrorBody), (status = 404, body = ErrorBody), (status = 422, body = ErrorBody))
)]
pub async fn project_table(
  State(state): State<AppState>,
  principal: Principal,
  Path((org, project)): Path<(String, String)>,
  Query(pairs): Query<Vec<(String, String)>>,
) -> Result<Json<TableOutput>, ApiError> {
  let caller = principal.caller;
  let parsed = parse_params(pairs)?;
  let target = address(&org, &project);
  let table = state
    .blocking(move |store| store.audit_table(&caller, AuditScope::Project(&target), &parsed.query))
    .await?;
  Ok(Json(table.into()))
}

#[utoipa::path(
  get,
  operation_id = "audit_organization_table",
  path = "/api/v1/orgs/{org}/audit/table",
  tag = "audit",
  params(("org" = String, Path), TableParams),
  responses((status = 200, body = TableOutput), (status = 401, body = ErrorBody), (status = 403, body = ErrorBody), (status = 404, body = ErrorBody), (status = 422, body = ErrorBody))
)]
pub async fn organization_table(
  State(state): State<AppState>,
  principal: Principal,
  Path(org): Path<String>,
  Query(pairs): Query<Vec<(String, String)>>,
) -> Result<Json<TableOutput>, ApiError> {
  let caller = principal.caller;
  let parsed = parse_params(pairs)?;
  let table = state
    .blocking(move |store| {
      store.audit_table(&caller, AuditScope::Organization(&org), &parsed.query)
    })
    .await?;
  Ok(Json(table.into()))
}

#[utoipa::path(
  get,
  operation_id = "audit_project_export",
  path = "/api/v1/projects/{org}/{project}/audit/export.csv",
  tag = "audit",
  params(("org" = String, Path), ("project" = String, Path), ExportParams),
  responses((status = 200, content_type = "text/csv", body = String), (status = 401, body = ErrorBody), (status = 403, body = ErrorBody), (status = 404, body = ErrorBody), (status = 422, body = ErrorBody))
)]
pub async fn project_export(
  State(state): State<AppState>,
  principal: Principal,
  Path((org, project)): Path<(String, String)>,
  Query(pairs): Query<Vec<(String, String)>>,
) -> Result<Response, ApiError> {
  let caller = principal.caller;
  let selection = selection(parse_params(pairs)?);
  let target = address(&org, &project);
  let csv = state
    .blocking(move |store| store.audit_export(&caller, AuditScope::Project(&target), selection))
    .await?;
  Ok(csv_response(csv, &format!("{org}-{project}")))
}

#[utoipa::path(
  get,
  operation_id = "audit_organization_export",
  path = "/api/v1/orgs/{org}/audit/export.csv",
  tag = "audit",
  params(("org" = String, Path), ExportParams),
  responses((status = 200, content_type = "text/csv", body = String), (status = 401, body = ErrorBody), (status = 403, body = ErrorBody), (status = 404, body = ErrorBody), (status = 422, body = ErrorBody))
)]
pub async fn organization_export(
  State(state): State<AppState>,
  principal: Principal,
  Path(org): Path<String>,
  Query(pairs): Query<Vec<(String, String)>>,
) -> Result<Response, ApiError> {
  let caller = principal.caller;
  let selection = selection(parse_params(pairs)?);
  let scope = org.clone();
  let csv = state
    .blocking(move |store| store.audit_export(&caller, AuditScope::Organization(&org), selection))
    .await?;
  Ok(csv_response(csv, &scope))
}
