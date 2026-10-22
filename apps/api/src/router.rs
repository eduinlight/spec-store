use axum::Router;
use axum::middleware;
use axum::routing::get;
use tower_http::limit::RequestBodyLimitLayer;

use crate::health::{live, ready};
use crate::hosts::check_host;
use crate::routes::me::me;
use crate::state::AppState;

pub const MAX_REQUEST_BODY_BYTES: usize = 2 * 1024 * 1024;

pub fn router(state: AppState) -> Router {
  Router::new()
    .route("/healthz", get(live))
    .route("/readyz", get(ready))
    .route("/api/v1/me", get(me))
    .layer(middleware::from_fn_with_state(state.clone(), check_host))
    .layer(RequestBodyLimitLayer::new(MAX_REQUEST_BODY_BYTES))
    .with_state(state)
}
