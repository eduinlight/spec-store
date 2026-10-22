use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use spec_store_core::model::Role;

use crate::auth::Principal;
use crate::error::{ApiError, ErrorBody};
use crate::state::AppState;

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct NewProject {
  pub slug: String,
  pub name: String,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ProjectSummaryOutput {
  pub address: String,
  pub slug: String,
  pub name: String,
  pub role: Option<String>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ProjectsOutput {
  pub projects: Vec<ProjectSummaryOutput>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ProjectOutput {
  pub address: String,
  pub organization: String,
  pub slug: String,
  pub name: String,
  pub repository_url: Option<String>,
  pub role: Option<String>,
  pub created_at: String,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ProjectMemberOutput {
  pub handle: String,
  pub role: String,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ProjectMembersOutput {
  pub members: Vec<ProjectMemberOutput>,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct RoleInput {
  pub role: String,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct RepositoryInput {
  pub url: String,
}

pub fn address(org: &str, project: &str) -> String {
  format!("{org}/{project}")
}

#[utoipa::path(
  get,
  operation_id = "projects_list",
  path = "/api/v1/orgs/{org}/projects",
  tag = "projects",
  params(("org" = String, Path)),
  responses((status = 200, body = ProjectsOutput), (status = 401, body = ErrorBody), (status = 403, body = ErrorBody), (status = 404, body = ErrorBody), (status = 422, body = ErrorBody))
)]
pub async fn list(
  State(state): State<AppState>,
  principal: Principal,
  Path(org): Path<String>,
) -> Result<Json<ProjectsOutput>, ApiError> {
  let caller = principal.caller;
  let projects = state
    .blocking(move |store| store.list_organization_projects(&caller, &org))
    .await?;
  Ok(Json(ProjectsOutput {
    projects: projects
      .into_iter()
      .map(|p| ProjectSummaryOutput {
        address: p.address.to_string(),
        slug: p.address.project.clone(),
        name: p.name,
        role: p.role.map(|r| r.as_str().to_string()),
      })
      .collect(),
  }))
}

async fn read_project(
  state: &AppState,
  principal: Principal,
  address: String,
) -> Result<ProjectOutput, ApiError> {
  let caller = principal.caller;
  let (project, role) = state
    .blocking(move |store| {
      let access = store.access(&caller, &address)?;
      Ok((store.project(&caller, &address)?, access.role))
    })
    .await?;
  Ok(ProjectOutput {
    address: project.address.to_string(),
    organization: project.address.organization.clone(),
    slug: project.slug,
    name: project.name,
    repository_url: project.repository_url,
    role: role.map(|r| r.as_str().to_string()),
    created_at: project.created_at,
  })
}

#[utoipa::path(
  post,
  operation_id = "projects_create",
  path = "/api/v1/orgs/{org}/projects",
  tag = "projects",
  params(("org" = String, Path)),
  request_body = NewProject,
  responses((status = 201, body = ProjectOutput), (status = 401, body = ErrorBody), (status = 403, body = ErrorBody), (status = 404, body = ErrorBody), (status = 422, body = ErrorBody))
)]
pub async fn create(
  State(state): State<AppState>,
  principal: Principal,
  Path(org): Path<String>,
  Json(body): Json<NewProject>,
) -> Result<(StatusCode, Json<ProjectOutput>), ApiError> {
  let caller = principal.caller.clone();
  let target = address(&org, &body.slug);
  let created_address = target.clone();
  state
    .blocking(move |store| store.create_project(&caller, &target, &body.name))
    .await?;
  Ok((
    StatusCode::CREATED,
    Json(read_project(&state, principal, created_address).await?),
  ))
}

#[utoipa::path(
  get,
  operation_id = "projects_read",
  path = "/api/v1/projects/{org}/{project}",
  tag = "projects",
  params(("org" = String, Path), ("project" = String, Path)),
  responses((status = 200, body = ProjectOutput), (status = 401, body = ErrorBody), (status = 403, body = ErrorBody), (status = 404, body = ErrorBody), (status = 422, body = ErrorBody))
)]
pub async fn read(
  State(state): State<AppState>,
  principal: Principal,
  Path((org, project)): Path<(String, String)>,
) -> Result<Json<ProjectOutput>, ApiError> {
  Ok(Json(
    read_project(&state, principal, address(&org, &project)).await?,
  ))
}

#[utoipa::path(
  get,
  operation_id = "projects_members",
  path = "/api/v1/projects/{org}/{project}/members",
  tag = "projects",
  params(("org" = String, Path), ("project" = String, Path)),
  responses((status = 200, body = ProjectMembersOutput), (status = 401, body = ErrorBody), (status = 403, body = ErrorBody), (status = 404, body = ErrorBody), (status = 422, body = ErrorBody))
)]
pub async fn members(
  State(state): State<AppState>,
  principal: Principal,
  Path((org, project)): Path<(String, String)>,
) -> Result<Json<ProjectMembersOutput>, ApiError> {
  let caller = principal.caller;
  let target = address(&org, &project);
  let members = state
    .blocking(move |store| store.list_members(&caller, &target))
    .await?;
  Ok(Json(ProjectMembersOutput {
    members: members
      .into_iter()
      .map(|m| ProjectMemberOutput {
        handle: m.handle,
        role: m.role.as_str().to_string(),
      })
      .collect(),
  }))
}

#[utoipa::path(
  put,
  operation_id = "projects_set_member",
  path = "/api/v1/projects/{org}/{project}/members/{handle}",
  tag = "projects",
  params(("org" = String, Path), ("project" = String, Path), ("handle" = String, Path)),
  request_body = RoleInput,
  responses((status = 204), (status = 401, body = ErrorBody), (status = 403, body = ErrorBody), (status = 404, body = ErrorBody), (status = 422, body = ErrorBody))
)]
pub async fn set_member(
  State(state): State<AppState>,
  principal: Principal,
  Path((org, project, handle)): Path<(String, String, String)>,
  Json(body): Json<RoleInput>,
) -> Result<StatusCode, ApiError> {
  let caller = principal.caller;
  let target = address(&org, &project);
  state
    .blocking(move |store| {
      let role: Role = body.role.parse()?;
      store.set_member(&caller, &target, &handle, role)
    })
    .await?;
  Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
  delete,
  operation_id = "projects_remove_member",
  path = "/api/v1/projects/{org}/{project}/members/{handle}",
  tag = "projects",
  params(("org" = String, Path), ("project" = String, Path), ("handle" = String, Path)),
  responses((status = 204), (status = 401, body = ErrorBody), (status = 403, body = ErrorBody), (status = 404, body = ErrorBody), (status = 422, body = ErrorBody))
)]
pub async fn remove_member(
  State(state): State<AppState>,
  principal: Principal,
  Path((org, project, handle)): Path<(String, String, String)>,
) -> Result<StatusCode, ApiError> {
  let caller = principal.caller;
  let target = address(&org, &project);
  state
    .blocking(move |store| store.remove_member(&caller, &target, &handle))
    .await?;
  Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
  put,
  operation_id = "projects_set_repository",
  path = "/api/v1/projects/{org}/{project}/repository-url",
  tag = "projects",
  params(("org" = String, Path), ("project" = String, Path)),
  request_body = RepositoryInput,
  responses((status = 204), (status = 401, body = ErrorBody), (status = 403, body = ErrorBody), (status = 404, body = ErrorBody), (status = 422, body = ErrorBody))
)]
pub async fn set_repository(
  State(state): State<AppState>,
  principal: Principal,
  Path((org, project)): Path<(String, String)>,
  Json(body): Json<RepositoryInput>,
) -> Result<StatusCode, ApiError> {
  let caller = principal.caller;
  let target = address(&org, &project);
  state
    .blocking(move |store| store.set_repository_url(&caller, &target, Some(&body.url)))
    .await?;
  Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
  delete,
  operation_id = "projects_clear_repository",
  path = "/api/v1/projects/{org}/{project}/repository-url",
  tag = "projects",
  params(("org" = String, Path), ("project" = String, Path)),
  responses((status = 204), (status = 401, body = ErrorBody), (status = 403, body = ErrorBody), (status = 404, body = ErrorBody), (status = 422, body = ErrorBody))
)]
pub async fn clear_repository(
  State(state): State<AppState>,
  principal: Principal,
  Path((org, project)): Path<(String, String)>,
) -> Result<StatusCode, ApiError> {
  let caller = principal.caller;
  let target = address(&org, &project);
  state
    .blocking(move |store| store.set_repository_url(&caller, &target, None))
    .await?;
  Ok(StatusCode::NO_CONTENT)
}
