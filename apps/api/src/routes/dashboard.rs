use axum::Json;
use axum::extract::{Path, Query, State};
use sdd_vault_core::store::tasks::SpecMarker;
use sdd_vault_core::store::views::active::{ActiveSpec, ActiveSpecs, DraftSpec, NextTask};
use sdd_vault_core::store::views::activity::{
  ActivityFilter, ActivityPage, ActivityRow, DEFAULT_ACTIVITY_LIMIT,
};
use sdd_vault_core::store::views::overview::{
  AttentionItem, ClientCount, DayCompleted, Overview, RequirementCoverage,
};
use serde::{Deserialize, Serialize};

use crate::auth::Principal;
use crate::error::{ApiError, ErrorBody};
use crate::routes::projects::address;
use crate::state::AppState;

pub const DEFAULT_PERIOD_DAYS: i64 = 7;
pub const MAX_ACTIVITY_LIMIT: usize = 100;

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ClientCountOutput {
  pub client: String,
  pub count: i64,
}

impl From<ClientCount> for ClientCountOutput {
  fn from(item: ClientCount) -> Self {
    ClientCountOutput {
      client: item.client,
      count: item.count,
    }
  }
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct DayCompletedOutput {
  pub day: String,
  pub by_client: Vec<ClientCountOutput>,
}

impl From<DayCompleted> for DayCompletedOutput {
  fn from(day: DayCompleted) -> Self {
    DayCompletedOutput {
      day: day.day,
      by_client: day.by_client.into_iter().map(Into::into).collect(),
    }
  }
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct CoverageOutput {
  pub spec_number: i64,
  pub covered: usize,
  pub total: usize,
}

impl From<RequirementCoverage> for CoverageOutput {
  fn from(coverage: RequirementCoverage) -> Self {
    CoverageOutput {
      spec_number: coverage.spec_number,
      covered: coverage.covered,
      total: coverage.total,
    }
  }
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct AttentionOutput {
  pub kind: String,
  pub spec_number: i64,
  pub spec_slug: String,
  pub count: usize,
  pub detail: Vec<String>,
}

impl From<AttentionItem> for AttentionOutput {
  fn from(item: AttentionItem) -> Self {
    AttentionOutput {
      kind: item.kind.as_str().to_string(),
      spec_number: item.spec_number,
      spec_slug: item.spec_slug,
      count: item.count,
      detail: item.detail,
    }
  }
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct OverviewOutput {
  pub days: i64,
  pub active_specs: usize,
  pub tasks_completed: i64,
  pub tasks_completed_previous: i64,
  pub coverage: Option<CoverageOutput>,
  pub open_markers: usize,
  pub agents_active: usize,
  pub completed_per_day: Vec<DayCompletedOutput>,
  pub attention: Vec<AttentionOutput>,
}

impl From<Overview> for OverviewOutput {
  fn from(overview: Overview) -> Self {
    OverviewOutput {
      days: overview.days,
      active_specs: overview.active_specs,
      tasks_completed: overview.tasks_completed,
      tasks_completed_previous: overview.tasks_completed_previous,
      coverage: overview.coverage.map(Into::into),
      open_markers: overview.open_markers,
      agents_active: overview.agents_active,
      completed_per_day: overview
        .completed_per_day
        .into_iter()
        .map(Into::into)
        .collect(),
      attention: overview.attention.into_iter().map(Into::into).collect(),
    }
  }
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct NextTaskOutput {
  pub id: String,
  pub parallel: bool,
  pub frs: Vec<String>,
  pub description: String,
  pub done_when: String,
}

impl From<NextTask> for NextTaskOutput {
  fn from(task: NextTask) -> Self {
    NextTaskOutput {
      id: task.id,
      parallel: task.parallel,
      frs: task.frs,
      description: task.description,
      done_when: task.done_when,
    }
  }
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ActiveSpecOutput {
  pub number: i64,
  pub slug: String,
  pub title: String,
  pub status: String,
  pub branch: String,
  pub tasks_done: usize,
  pub tasks_total: usize,
  pub current_phase: Option<String>,
  pub open_markers: usize,
  pub next_task: Option<NextTaskOutput>,
  pub agent_prompt: Option<String>,
}

impl From<ActiveSpec> for ActiveSpecOutput {
  fn from(spec: ActiveSpec) -> Self {
    ActiveSpecOutput {
      number: spec.number,
      slug: spec.slug,
      title: spec.title,
      status: spec.status.as_str().to_string(),
      branch: spec.branch,
      tasks_done: spec.tasks_done,
      tasks_total: spec.tasks_total,
      current_phase: spec.current_phase,
      open_markers: spec.open_markers,
      next_task: spec.next_task.map(Into::into),
      agent_prompt: spec.agent_prompt,
    }
  }
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct DraftMarkerOutput {
  pub kind: String,
  pub document_kind: String,
  pub heading: String,
  pub text: String,
  pub line: usize,
}

impl From<SpecMarker> for DraftMarkerOutput {
  fn from(marker: SpecMarker) -> Self {
    DraftMarkerOutput {
      kind: marker.kind.as_str().to_string(),
      document_kind: marker.document_kind.as_str().to_string(),
      heading: marker.heading,
      text: marker.text,
      line: marker.line,
    }
  }
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct DraftSpecOutput {
  pub number: i64,
  pub slug: String,
  pub title: String,
  pub summary: String,
  pub markers: Vec<DraftMarkerOutput>,
}

impl From<DraftSpec> for DraftSpecOutput {
  fn from(draft: DraftSpec) -> Self {
    DraftSpecOutput {
      number: draft.number,
      slug: draft.slug,
      title: draft.title,
      summary: draft.summary,
      markers: draft.markers.into_iter().map(Into::into).collect(),
    }
  }
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ActiveSpecsOutput {
  pub active: Option<ActiveSpecOutput>,
  pub drafts: Vec<DraftSpecOutput>,
}

impl From<ActiveSpecs> for ActiveSpecsOutput {
  fn from(view: ActiveSpecs) -> Self {
    ActiveSpecsOutput {
      active: view.active.map(Into::into),
      drafts: view.drafts.into_iter().map(Into::into).collect(),
    }
  }
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ActivityRowOutput {
  pub audit_id: i64,
  pub actor: String,
  pub client: Option<String>,
  pub action: String,
  pub target: String,
  pub result: String,
  pub detail: Option<String>,
  pub at: String,
  pub summary_key: String,
  pub resolved_by_audit_id: Option<i64>,
}

impl From<ActivityRow> for ActivityRowOutput {
  fn from(row: ActivityRow) -> Self {
    ActivityRowOutput {
      audit_id: row.audit_id,
      actor: row.actor,
      client: row.client,
      action: row.action,
      target: row.target,
      result: row.result.as_str().to_string(),
      detail: row.detail,
      at: row.at,
      summary_key: row.summary_key,
      resolved_by_audit_id: row.resolved_by_audit_id,
    }
  }
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ActivityOutput {
  pub rows: Vec<ActivityRowOutput>,
  pub has_more: bool,
}

impl From<ActivityPage> for ActivityOutput {
  fn from(page: ActivityPage) -> Self {
    ActivityOutput {
      rows: page.rows.into_iter().map(Into::into).collect(),
      has_more: page.has_more,
    }
  }
}

#[derive(Debug, Deserialize, Default, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub struct OverviewQuery {
  pub days: Option<i64>,
}

#[derive(Debug, Deserialize, Default, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub struct ActivityQuery {
  pub filter: Option<String>,
  pub before: Option<i64>,
  pub limit: Option<usize>,
}

#[utoipa::path(
  get,
  operation_id = "project_overview",
  path = "/api/v1/projects/{org}/{project}/overview",
  tag = "dashboard",
  params(("org" = String, Path), ("project" = String, Path), OverviewQuery),
  responses((status = 200, body = OverviewOutput), (status = 401, body = ErrorBody), (status = 404, body = ErrorBody), (status = 422, body = ErrorBody))
)]
pub async fn overview(
  State(state): State<AppState>,
  principal: Principal,
  Path((org, project)): Path<(String, String)>,
  Query(query): Query<OverviewQuery>,
) -> Result<Json<OverviewOutput>, ApiError> {
  let caller = principal.caller;
  let target = address(&org, &project);
  let days = query.days.unwrap_or(DEFAULT_PERIOD_DAYS);
  let overview = state
    .blocking(move |store| store.project_dashboard(&caller, &target, days))
    .await?;
  Ok(Json(overview.into()))
}

#[utoipa::path(
  get,
  operation_id = "project_active_specs",
  path = "/api/v1/projects/{org}/{project}/active",
  tag = "dashboard",
  params(("org" = String, Path), ("project" = String, Path)),
  responses((status = 200, body = ActiveSpecsOutput), (status = 401, body = ErrorBody), (status = 404, body = ErrorBody))
)]
pub async fn active(
  State(state): State<AppState>,
  principal: Principal,
  Path((org, project)): Path<(String, String)>,
) -> Result<Json<ActiveSpecsOutput>, ApiError> {
  let caller = principal.caller;
  let target = address(&org, &project);
  let view = state
    .blocking(move |store| store.active_specs(&caller, &target))
    .await?;
  Ok(Json(view.into()))
}

#[utoipa::path(
  get,
  operation_id = "project_activity",
  path = "/api/v1/projects/{org}/{project}/activity",
  tag = "dashboard",
  params(("org" = String, Path), ("project" = String, Path), ActivityQuery),
  responses((status = 200, body = ActivityOutput), (status = 401, body = ErrorBody), (status = 404, body = ErrorBody), (status = 422, body = ErrorBody))
)]
pub async fn activity(
  State(state): State<AppState>,
  principal: Principal,
  Path((org, project)): Path<(String, String)>,
  Query(query): Query<ActivityQuery>,
) -> Result<Json<ActivityOutput>, ApiError> {
  let caller = principal.caller;
  let target = address(&org, &project);
  let limit = query
    .limit
    .unwrap_or(DEFAULT_ACTIVITY_LIMIT)
    .clamp(1, MAX_ACTIVITY_LIMIT);
  let before = query.before;
  let filter = query.filter;
  let page = state
    .blocking(move |store| {
      let filter: ActivityFilter = match filter {
        Some(name) => name.parse()?,
        None => ActivityFilter::All,
      };
      store.project_activity(&caller, &target, filter, before, limit)
    })
    .await?;
  Ok(Json(page.into()))
}
