use axum::Router;
use axum::middleware;
use axum::routing::{delete, get, post, put};
use tower_http::limit::RequestBodyLimitLayer;

use crate::health::{live, ready};
use crate::hosts::check_host;
use crate::routes::clients::{list_clients, revoke_client};
use crate::routes::me::me;
use crate::routes::{invitations, members, organizations};
use crate::state::AppState;

pub const MAX_REQUEST_BODY_BYTES: usize = 2 * 1024 * 1024;

pub fn router(state: AppState) -> Router {
  Router::new()
    .route("/healthz", get(live))
    .route("/readyz", get(ready))
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
    .layer(RequestBodyLimitLayer::new(MAX_REQUEST_BODY_BYTES))
    .with_state(state)
}
