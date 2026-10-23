use axum::Json;
use axum::extract::{Path, State};
use serde::Serialize;
use spec_store_core::markdown::tasks::{Malformed, Task};
use spec_store_core::model::StoreError;
use spec_store_core::store::specs::SpecSummary;
use spec_store_core::store::tasks::{ActiveStatus, SpecMarker};

use crate::auth::Principal;
use crate::error::{ApiError, ErrorBody};
use crate::routes::projects::address;
use crate::state::AppState;

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct SpecOutput {
  pub number: i64,
  pub slug: String,
  pub status: String,
  pub branch: String,
  pub kinds: Vec<String>,
}

impl From<SpecSummary> for SpecOutput {
  fn from(s: SpecSummary) -> Self {
    SpecOutput {
      number: s.number,
      slug: s.slug,
      status: s.status.as_str().to_string(),
      branch: s.branch,
      kinds: s.kinds.iter().map(|k| k.as_str().to_string()).collect(),
    }
  }
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct SpecsOutput {
  pub specs: Vec<SpecOutput>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct TaskOutput {
  pub id: String,
  pub phase: String,
  pub parallel: bool,
  pub frs: Vec<String>,
  pub description: String,
  pub done_when: String,
  pub done: bool,
  pub commit: Option<String>,
  pub line: usize,
}

impl From<Task> for TaskOutput {
  fn from(t: Task) -> Self {
    TaskOutput {
      id: t.id,
      phase: t.phase,
      parallel: t.parallel,
      frs: t.frs,
      description: t.description,
      done_when: t.done_when,
      done: t.done,
      commit: t.commit,
      line: t.line,
    }
  }
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct MalformedOutput {
  pub line: usize,
  pub text: String,
}

impl From<Malformed> for MalformedOutput {
  fn from(m: Malformed) -> Self {
    MalformedOutput {
      line: m.line,
      text: m.text,
    }
  }
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct TasksOutput {
  pub tasks: Vec<TaskOutput>,
  pub malformed: Vec<MalformedOutput>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct MarkerOutput {
  pub document_kind: String,
  pub kind: String,
  pub heading: String,
  pub text: String,
  pub line: usize,
}

impl From<SpecMarker> for MarkerOutput {
  fn from(m: SpecMarker) -> Self {
    MarkerOutput {
      document_kind: m.document_kind.as_str().to_string(),
      kind: m.kind.as_str().to_string(),
      heading: m.heading,
      text: m.text,
      line: m.line,
    }
  }
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct MarkersOutput {
  pub markers: Vec<MarkerOutput>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ActiveOutput {
  pub number: i64,
  pub slug: String,
  pub status: String,
  pub branch: String,
  pub tasks_done: usize,
  pub tasks_total: usize,
  pub current_phase: Option<String>,
  pub next_task: Option<TaskOutput>,
  pub markers: usize,
}

impl From<ActiveStatus> for ActiveOutput {
  fn from(a: ActiveStatus) -> Self {
    ActiveOutput {
      number: a.number,
      slug: a.slug,
      status: a.status.as_str().to_string(),
      branch: a.branch,
      tasks_done: a.tasks_done,
      tasks_total: a.tasks_total,
      current_phase: a.current_phase,
      next_task: a.next_task.map(TaskOutput::from),
      markers: a.markers,
    }
  }
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct StatusOutput {
  pub constitution_version: Option<String>,
  pub specs: Vec<SpecOutput>,
  pub active: Option<ActiveOutput>,
}

#[utoipa::path(
  get,
  operation_id = "specs_status",
  path = "/api/v1/projects/{org}/{project}/status",
  tag = "specs",
  params(("org" = String, Path), ("project" = String, Path)),
  responses((status = 200, body = StatusOutput), (status = 401, body = ErrorBody), (status = 404, body = ErrorBody))
)]
pub async fn status(
  State(state): State<AppState>,
  principal: Principal,
  Path((org, project)): Path<(String, String)>,
) -> Result<Json<StatusOutput>, ApiError> {
  let caller = principal.caller;
  let target = address(&org, &project);
  let status = state
    .blocking(move |store| store.project_status(&caller, &target))
    .await?;
  Ok(Json(StatusOutput {
    constitution_version: status.constitution_version,
    specs: status.specs.into_iter().map(SpecOutput::from).collect(),
    active: status.active.map(ActiveOutput::from),
  }))
}

#[utoipa::path(
  get,
  operation_id = "specs_list",
  path = "/api/v1/projects/{org}/{project}/specs",
  tag = "specs",
  params(("org" = String, Path), ("project" = String, Path)),
  responses((status = 200, body = SpecsOutput), (status = 401, body = ErrorBody), (status = 404, body = ErrorBody))
)]
pub async fn list(
  State(state): State<AppState>,
  principal: Principal,
  Path((org, project)): Path<(String, String)>,
) -> Result<Json<SpecsOutput>, ApiError> {
  let caller = principal.caller;
  let target = address(&org, &project);
  let specs = state
    .blocking(move |store| store.list_specs(&caller, &target))
    .await?;
  Ok(Json(SpecsOutput {
    specs: specs.into_iter().map(SpecOutput::from).collect(),
  }))
}

#[utoipa::path(
  get,
  operation_id = "specs_read",
  path = "/api/v1/projects/{org}/{project}/specs/{number}",
  tag = "specs",
  params(("org" = String, Path), ("project" = String, Path), ("number" = i64, Path)),
  responses((status = 200, body = SpecOutput), (status = 401, body = ErrorBody), (status = 404, body = ErrorBody))
)]
pub async fn read(
  State(state): State<AppState>,
  principal: Principal,
  Path((org, project, number)): Path<(String, String, i64)>,
) -> Result<Json<SpecOutput>, ApiError> {
  let caller = principal.caller;
  let target = address(&org, &project);
  let spec = state
    .blocking(move |store| {
      store
        .list_specs(&caller, &target)?
        .into_iter()
        .find(|s| s.number == number)
        .ok_or_else(|| StoreError::NotFound(format!("spec {number:03} in project {target:?}")))
    })
    .await?;
  Ok(Json(spec.into()))
}

#[utoipa::path(
  get,
  operation_id = "specs_tasks",
  path = "/api/v1/projects/{org}/{project}/specs/{number}/tasks",
  tag = "specs",
  params(("org" = String, Path), ("project" = String, Path), ("number" = i64, Path)),
  responses((status = 200, body = TasksOutput), (status = 401, body = ErrorBody), (status = 404, body = ErrorBody))
)]
pub async fn tasks(
  State(state): State<AppState>,
  principal: Principal,
  Path((org, project, number)): Path<(String, String, i64)>,
) -> Result<Json<TasksOutput>, ApiError> {
  let caller = principal.caller;
  let target = address(&org, &project);
  let parsed = state
    .blocking(move |store| store.list_tasks(&caller, &target, number, false))
    .await?;
  Ok(Json(TasksOutput {
    tasks: parsed.tasks.into_iter().map(TaskOutput::from).collect(),
    malformed: parsed
      .malformed
      .into_iter()
      .map(MalformedOutput::from)
      .collect(),
  }))
}

#[utoipa::path(
  get,
  operation_id = "specs_markers",
  path = "/api/v1/projects/{org}/{project}/specs/{number}/markers",
  tag = "specs",
  params(("org" = String, Path), ("project" = String, Path), ("number" = i64, Path)),
  responses((status = 200, body = MarkersOutput), (status = 401, body = ErrorBody), (status = 404, body = ErrorBody))
)]
pub async fn markers(
  State(state): State<AppState>,
  principal: Principal,
  Path((org, project, number)): Path<(String, String, i64)>,
) -> Result<Json<MarkersOutput>, ApiError> {
  let caller = principal.caller;
  let target = address(&org, &project);
  let markers = state
    .blocking(move |store| store.list_markers(&caller, &target, number))
    .await?;
  Ok(Json(MarkersOutput {
    markers: markers.into_iter().map(MarkerOutput::from).collect(),
  }))
}
