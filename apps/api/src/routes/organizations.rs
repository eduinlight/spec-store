use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};

use crate::auth::Principal;
use crate::error::{ApiError, ErrorBody};
use crate::state::AppState;

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct NewOrganization {
  pub slug: String,
  pub name: String,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct OrganizationOutput {
  pub slug: String,
  pub name: String,
  pub role: Option<String>,
  pub created_at: String,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct OrganizationSummaryOutput {
  pub slug: String,
  pub name: String,
  pub role: Option<String>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct OrganizationsOutput {
  pub organizations: Vec<OrganizationSummaryOutput>,
}

#[utoipa::path(
  post,
  path = "/api/v1/orgs",
  tag = "organizations",
  request_body = NewOrganization,
  responses((status = 201, body = OrganizationOutput), (status = 401, body = ErrorBody), (status = 403, body = ErrorBody), (status = 404, body = ErrorBody), (status = 422, body = ErrorBody))
)]
pub async fn create(
  State(state): State<AppState>,
  principal: Principal,
  Json(body): Json<NewOrganization>,
) -> Result<(StatusCode, Json<OrganizationOutput>), ApiError> {
  let caller = principal.caller;
  let created = state
    .blocking(move |store| store.create_organization(&caller, &body.slug, &body.name))
    .await?;
  Ok((
    StatusCode::CREATED,
    Json(OrganizationOutput {
      slug: created.slug,
      name: created.name,
      role: Some("owner".into()),
      created_at: created.created_at,
    }),
  ))
}

#[utoipa::path(
  get,
  path = "/api/v1/orgs",
  tag = "organizations",
  responses((status = 200, body = OrganizationsOutput), (status = 401, body = ErrorBody), (status = 403, body = ErrorBody), (status = 404, body = ErrorBody), (status = 422, body = ErrorBody))
)]
pub async fn list(
  State(state): State<AppState>,
  principal: Principal,
) -> Result<Json<OrganizationsOutput>, ApiError> {
  let caller = principal.caller;
  let organizations = state
    .blocking(move |store| store.list_organizations(&caller))
    .await?;
  Ok(Json(OrganizationsOutput {
    organizations: organizations
      .into_iter()
      .map(|o| OrganizationSummaryOutput {
        slug: o.slug,
        name: o.name,
        role: o.role.map(|r| r.as_str().to_string()),
      })
      .collect(),
  }))
}

#[utoipa::path(
  get,
  path = "/api/v1/orgs/{org}",
  tag = "organizations",
  params(("org" = String, Path)),
  responses((status = 200, body = OrganizationOutput), (status = 401, body = ErrorBody), (status = 403, body = ErrorBody), (status = 404, body = ErrorBody), (status = 422, body = ErrorBody))
)]
pub async fn read(
  State(state): State<AppState>,
  principal: Principal,
  Path(org): Path<String>,
) -> Result<Json<OrganizationOutput>, ApiError> {
  let caller = principal.caller;
  let (organization, role) = state
    .blocking(move |store| {
      let organization = store.organization(&caller, &org)?;
      let role = store
        .list_organizations(&caller)?
        .into_iter()
        .find(|o| o.slug == organization.slug)
        .and_then(|o| o.role);
      Ok((organization, role))
    })
    .await?;
  Ok(Json(OrganizationOutput {
    slug: organization.slug,
    name: organization.name,
    role: role.map(|r| r.as_str().to_string()),
    created_at: organization.created_at,
  }))
}
