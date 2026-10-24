use axum::Json;
use axum::extract::{Path, State};
use sdd_vault_core::store::views::readiness::{Readiness, ReadinessItem};
use sdd_vault_core::store::views::spec_list::SpecCard;
use sdd_vault_core::store::views::traceability::{TraceRequirement, TraceTask, Traceability};
use serde::Serialize;

use crate::auth::Principal;
use crate::error::{ApiError, ErrorBody};
use crate::routes::projects::address;
use crate::state::AppState;

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct SpecCardOutput {
  pub number: i64,
  pub slug: String,
  pub title: String,
  pub summary: String,
  pub status: String,
  pub branch: String,
  pub created_at: String,
  pub updated_at: String,
  pub contributors: Vec<String>,
  pub documents: Vec<String>,
  pub requirement_count: usize,
  pub tasks_done: usize,
  pub tasks_total: usize,
  pub open_markers: usize,
}

impl From<SpecCard> for SpecCardOutput {
  fn from(card: SpecCard) -> Self {
    SpecCardOutput {
      number: card.number,
      slug: card.slug,
      title: card.title,
      summary: card.summary,
      status: card.status.as_str().to_string(),
      branch: card.branch,
      created_at: card.created_at,
      updated_at: card.updated_at,
      contributors: card.contributors,
      documents: card
        .documents
        .iter()
        .map(|kind| kind.as_str().to_string())
        .collect(),
      requirement_count: card.requirement_count,
      tasks_done: card.tasks_done,
      tasks_total: card.tasks_total,
      open_markers: card.open_markers,
    }
  }
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct SpecCardsOutput {
  pub specs: Vec<SpecCardOutput>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct TraceTaskOutput {
  pub id: String,
  pub description: String,
  pub done: bool,
  pub commit: Option<String>,
}

impl From<TraceTask> for TraceTaskOutput {
  fn from(task: TraceTask) -> Self {
    TraceTaskOutput {
      id: task.id,
      description: task.description,
      done: task.done,
      commit: task.commit,
    }
  }
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct TraceRequirementOutput {
  pub id: String,
  pub text: String,
  pub status: String,
  pub tasks: Vec<TraceTaskOutput>,
}

impl From<TraceRequirement> for TraceRequirementOutput {
  fn from(requirement: TraceRequirement) -> Self {
    TraceRequirementOutput {
      id: requirement.id,
      text: requirement.text,
      status: requirement.status.as_str().to_string(),
      tasks: requirement
        .tasks
        .into_iter()
        .map(TraceTaskOutput::from)
        .collect(),
    }
  }
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct TraceabilityOutput {
  pub requirements: Vec<TraceRequirementOutput>,
  pub tasks_without_requirements: Vec<TraceTaskOutput>,
}

impl From<Traceability> for TraceabilityOutput {
  fn from(trace: Traceability) -> Self {
    TraceabilityOutput {
      requirements: trace
        .requirements
        .into_iter()
        .map(TraceRequirementOutput::from)
        .collect(),
      tasks_without_requirements: trace
        .tasks_without_requirements
        .into_iter()
        .map(TraceTaskOutput::from)
        .collect(),
    }
  }
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ReadinessItemOutput {
  pub key: String,
  pub ok: bool,
  pub count: Option<usize>,
  pub total: Option<usize>,
}

impl From<ReadinessItem> for ReadinessItemOutput {
  fn from(item: ReadinessItem) -> Self {
    ReadinessItemOutput {
      key: item.key.as_str().to_string(),
      ok: item.ok,
      count: item.count,
      total: item.total,
    }
  }
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ReadinessOutput {
  pub ready: bool,
  pub items: Vec<ReadinessItemOutput>,
}

impl From<Readiness> for ReadinessOutput {
  fn from(readiness: Readiness) -> Self {
    ReadinessOutput {
      ready: readiness.ready,
      items: readiness
        .items
        .into_iter()
        .map(ReadinessItemOutput::from)
        .collect(),
    }
  }
}

#[utoipa::path(
  get,
  operation_id = "specs_cards",
  path = "/api/v1/projects/{org}/{project}/specs/cards",
  tag = "specs",
  params(("org" = String, Path), ("project" = String, Path)),
  responses((status = 200, body = SpecCardsOutput), (status = 401, body = ErrorBody), (status = 404, body = ErrorBody))
)]
pub async fn cards(
  State(state): State<AppState>,
  principal: Principal,
  Path((org, project)): Path<(String, String)>,
) -> Result<Json<SpecCardsOutput>, ApiError> {
  let caller = principal.caller;
  let target = address(&org, &project);
  let cards = state
    .blocking(move |store| store.spec_cards(&caller, &target))
    .await?;
  Ok(Json(SpecCardsOutput {
    specs: cards.into_iter().map(SpecCardOutput::from).collect(),
  }))
}

#[utoipa::path(
  get,
  operation_id = "specs_traceability",
  path = "/api/v1/projects/{org}/{project}/specs/{number}/traceability",
  tag = "specs",
  params(("org" = String, Path), ("project" = String, Path), ("number" = i64, Path)),
  responses((status = 200, body = TraceabilityOutput), (status = 401, body = ErrorBody), (status = 404, body = ErrorBody))
)]
pub async fn traceability(
  State(state): State<AppState>,
  principal: Principal,
  Path((org, project, number)): Path<(String, String, i64)>,
) -> Result<Json<TraceabilityOutput>, ApiError> {
  let caller = principal.caller;
  let target = address(&org, &project);
  let trace = state
    .blocking(move |store| store.spec_traceability(&caller, &target, number))
    .await?;
  Ok(Json(trace.into()))
}

#[utoipa::path(
  get,
  operation_id = "specs_readiness",
  path = "/api/v1/projects/{org}/{project}/specs/{number}/readiness",
  tag = "specs",
  params(("org" = String, Path), ("project" = String, Path), ("number" = i64, Path)),
  responses((status = 200, body = ReadinessOutput), (status = 401, body = ErrorBody), (status = 404, body = ErrorBody))
)]
pub async fn readiness(
  State(state): State<AppState>,
  principal: Principal,
  Path((org, project, number)): Path<(String, String, i64)>,
) -> Result<Json<ReadinessOutput>, ApiError> {
  let caller = principal.caller;
  let target = address(&org, &project);
  let readiness = state
    .blocking(move |store| store.spec_readiness(&caller, &target, number))
    .await?;
  Ok(Json(readiness.into()))
}
