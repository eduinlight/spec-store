use axum::Router;
use axum::extract::DefaultBodyLimit;
use axum::middleware;
use axum::routing::{delete, get, post, put};

use crate::bff;
use crate::health::{live, ready};
use crate::hosts::check_host;
use crate::routes::clients::{list_clients, revoke_client};
use crate::routes::me::me;
use crate::routes::{audit, invitations, members, organizations, projects, transfer};
use crate::state::AppState;

pub const MAX_REQUEST_BODY_BYTES: usize = 2 * 1024 * 1024;
pub const MAX_IMPORT_BODY_BYTES: usize = 128 * 1024 * 1024;

pub fn router(state: AppState) -> Router {
  Router::new()
    .route("/healthz", get(live))
    .route("/readyz", get(ready))
    .route("/auth/login", get(bff::login))
    .route("/auth/callback", get(bff::callback))
    .route("/auth/logout", post(bff::logout))
    .route("/auth/backchannel-logout", post(bff::backchannel_logout))
    .route("/api/v1/me", get(me))
    .route("/api/v1/me/clients", get(list_clients))
    .route("/api/v1/me/clients/{client_id}", delete(revoke_client))
    .route(
      "/api/v1/me/invitations/{id}/accept",
      post(invitations::accept_pending),
    )
    .route(
      "/api/v1/me/invitations/{id}/decline",
      post(invitations::decline_pending),
    )
    .route(
      "/api/v1/orgs",
      get(organizations::list).post(organizations::create),
    )
    .route("/api/v1/orgs/{org}", get(organizations::read))
    .route("/api/v1/orgs/{org}/members", get(members::list))
    .route(
      "/api/v1/orgs/{org}/members/{handle}",
      put(members::set_role).delete(members::remove),
    )
    .route(
      "/api/v1/orgs/{org}/invitations",
      get(invitations::list).post(invitations::create),
    )
    .route(
      "/api/v1/orgs/{org}/invitations/{id}",
      delete(invitations::revoke),
    )
    .route(
      "/api/v1/orgs/{org}/invitations/{id}/resend",
      post(invitations::resend),
    )
    .route(
      "/api/v1/orgs/{org}/projects",
      get(projects::list).post(projects::create),
    )
    .route("/api/v1/orgs/{org}/audit", get(audit::organization))
    .route("/api/v1/audit", get(audit::global))
    .route("/api/v1/projects/{org}/{project}", get(projects::read))
    .route(
      "/api/v1/projects/{org}/{project}/members",
      get(projects::members),
    )
    .route(
      "/api/v1/projects/{org}/{project}/members/{handle}",
      put(projects::set_member).delete(projects::remove_member),
    )
    .route(
      "/api/v1/projects/{org}/{project}/repository-url",
      put(projects::set_repository).delete(projects::clear_repository),
    )
    .route(
      "/api/v1/projects/{org}/{project}/audit",
      get(audit::project),
    )
    .route(
      "/api/v1/projects/{org}/{project}/import",
      post(transfer::import).layer(DefaultBodyLimit::max(MAX_IMPORT_BODY_BYTES)),
    )
    .route(
      "/api/v1/projects/{org}/{project}/export",
      get(transfer::export),
    )
    .route("/api/v1/invitations", get(invitations::mine))
    .route("/api/v1/invitations/{token}", get(invitations::preview))
    .route(
      "/api/v1/invitations/{token}/accept",
      post(invitations::accept),
    )
    .route(
      "/api/v1/invitations/{token}/decline",
      post(invitations::decline),
    )
    .layer(middleware::from_fn_with_state(state.clone(), check_host))
    .layer(DefaultBodyLimit::max(MAX_REQUEST_BODY_BYTES))
    .with_state(state)
}
