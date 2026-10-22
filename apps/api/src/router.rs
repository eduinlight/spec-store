use axum::Router;
use axum::middleware;
use axum::routing::{delete, get};
use tower_http::limit::RequestBodyLimitLayer;

use crate::health::{live, ready};
use crate::hosts::check_host;
use crate::routes::clients::{list_clients, revoke_client};
use crate::routes::me::me;
use crate::state::AppState;

pub const MAX_REQUEST_BODY_BYTES: usize = 2 * 1024 * 1024;

pub fn router(state: AppState) -> Router {
  Router::new()
    .route("/healthz", get(live))
    .route("/readyz", get(ready))
    .route("/api/v1/me", get(me))
    .route("/api/v1/me/clients", get(list_clients))
    .route("/api/v1/me/clients/{client_id}", delete(revoke_client))
    .layer(middleware::from_fn_with_state(state.clone(), check_host))
    .layer(RequestBodyLimitLayer::new(MAX_REQUEST_BODY_BYTES))
    .with_state(state)
}
