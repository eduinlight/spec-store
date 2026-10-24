use axum::Router;
use axum::middleware;
use axum::routing::get;
use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
use rmcp::transport::streamable_http_server::{StreamableHttpServerConfig, StreamableHttpService};

use crate::auth::require_bearer;
use crate::health::{live, ready};
use crate::metadata::protected_resource;
use crate::state::AppState;
use crate::tools::SddVaultMcp;

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
  let client_names = state.client_names.clone();
  let service = StreamableHttpService::new(
    move || Ok(SddVaultMcp::new(store.clone(), client_names.clone())),
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
  let metadata_path = format!(
    "/.well-known/oauth-protected-resource{}",
    state.config.resource_path()
  );
  let mut public = Router::new()
    .route("/healthz", get(live))
    .route("/readyz", get(ready))
    .route(
      "/.well-known/oauth-protected-resource",
      get(protected_resource),
    );
  if !state.config.resource_path().is_empty() {
    public = public.route(&metadata_path, get(protected_resource));
  }
  public.merge(protected).with_state(state)
}
