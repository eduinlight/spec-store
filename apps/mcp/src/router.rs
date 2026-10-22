use axum::Router;
use axum::middleware;
use axum::routing::get;
use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
use rmcp::transport::streamable_http_server::{StreamableHttpServerConfig, StreamableHttpService};

use crate::auth::require_bearer;
use crate::health::{live, ready};
use crate::state::AppState;
use crate::tools::SpecStoreMcp;

pub const MAX_REQUEST_BODY_BYTES: usize = 2 * 1024 * 1024;

pub fn router(state: AppState) -> Router {
  let mut config = StreamableHttpServerConfig::default()
    .with_legacy_session_mode(false)
    .with_json_response(true)
    .with_max_request_body_bytes(MAX_REQUEST_BODY_BYTES);
  if !state.config.allowed_hosts.is_empty() {
    config = config.with_allowed_hosts(state.config.allowed_hosts.clone());
  }
  let store = state.store.clone();
  let service = StreamableHttpService::new(
    move || Ok(SpecStoreMcp::new(store.clone())),
    LocalSessionManager::default().into(),
    config,
  );
  let protected =
    Router::new()
      .nest_service("/mcp", service)
      .route_layer(middleware::from_fn_with_state(
        state.clone(),
        require_bearer,
      ));
  Router::new()
    .route("/healthz", get(live))
    .route("/readyz", get(ready))
    .merge(protected)
    .with_state(state)
}
