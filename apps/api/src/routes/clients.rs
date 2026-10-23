use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use serde::Serialize;
use spec_store_core::model::StoreError;

use crate::auth::Principal;
use crate::error::{ApiError, ErrorBody};
use crate::state::AppState;

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct AuthorizedClient {
  pub client_id: String,
  pub name: String,
  pub scopes: Vec<String>,
  pub granted_at: String,
  pub updated_at: String,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct AuthorizedClients {
  pub clients: Vec<AuthorizedClient>,
}

#[utoipa::path(
  get,
  operation_id = "clients_list_clients",
  path = "/api/v1/me/clients",
  tag = "me",
  responses((status = 200, body = AuthorizedClients), (status = 401, body = ErrorBody), (status = 403, body = ErrorBody), (status = 404, body = ErrorBody), (status = 422, body = ErrorBody))
)]
pub async fn list_clients(
  State(state): State<AppState>,
  principal: Principal,
) -> Result<Json<AuthorizedClients>, ApiError> {
  let consents = state
    .admin()?
    .list_consents(&principal.identity.subject)
    .await
    .map_err(StoreError::from)?;
  Ok(Json(AuthorizedClients {
    clients: consents
      .into_iter()
      .map(|c| AuthorizedClient {
        name: c.name.clone().unwrap_or_else(|| c.client_id.clone()),
        client_id: c.client_id,
        scopes: c.scopes,
        granted_at: c.granted_at,
        updated_at: c.updated_at,
      })
      .collect(),
  }))
}

#[utoipa::path(
  delete,
  operation_id = "clients_revoke_client",
  path = "/api/v1/me/clients/{client_id}",
  tag = "me",
  params(("client_id" = String, Path)),
  responses((status = 204), (status = 401, body = ErrorBody), (status = 403, body = ErrorBody), (status = 404, body = ErrorBody), (status = 422, body = ErrorBody))
)]
pub async fn revoke_client(
  State(state): State<AppState>,
  principal: Principal,
  Path(client_id): Path<String>,
) -> Result<StatusCode, ApiError> {
  state
    .admin()?
    .revoke_consent(&principal.identity.subject, &client_id)
    .await
    .map_err(StoreError::from)?;
  Ok(StatusCode::NO_CONTENT)
}
