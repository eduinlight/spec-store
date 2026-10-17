use anyhow::Context;
use axum::Router;
use axum::middleware;
use axum::routing::get;
use rmcp::ServiceExt;
use rmcp::transport::stdio;
use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
use rmcp::transport::streamable_http_server::{StreamableHttpServerConfig, StreamableHttpService};

use crate::auth::require_bearer;
use crate::mcp::SpecStoreMcp;
use crate::model::Caller;
use crate::store::Store;
use crate::ui::{UiConfig, ui_router};

pub const MAX_REQUEST_BODY_BYTES: usize = 2 * 1024 * 1024;

#[derive(Debug, Clone, Default)]
pub struct ServerConfig {
  pub allowed_hosts: Vec<String>,
  pub secure_cookies: bool,
}

pub fn build_router(store: Store, config: ServerConfig) -> Router {
  let mut http_config =
    StreamableHttpServerConfig::default().with_max_request_body_bytes(MAX_REQUEST_BODY_BYTES);
  if !config.allowed_hosts.is_empty() {
    http_config = http_config.with_allowed_hosts(config.allowed_hosts.clone());
  }
  let ui = ui_router(
    store.clone(),
    UiConfig {
      secure_cookies: config.secure_cookies,
      allowed_hosts: config.allowed_hosts,
    },
  );
  let mcp_store = store.clone();
  let service = StreamableHttpService::new(
    move || Ok(SpecStoreMcp::new(mcp_store.clone())),
    LocalSessionManager::default().into(),
    http_config,
  );
  let protected = Router::new()
    .nest_service("/mcp", service)
    .route_layer(middleware::from_fn_with_state(store, require_bearer));
  Router::new()
    .route("/healthz", get(|| async { "ok" }))
    .merge(protected)
    .merge(ui)
}

pub async fn serve(store: Store, bind: &str, config: ServerConfig) -> anyhow::Result<()> {
  let listener = tokio::net::TcpListener::bind(bind)
    .await
    .with_context(|| format!("binding {bind}"))?;
  tracing::info!(%bind, "serving MCP at /mcp and the UI at /ui/");
  axum::serve(listener, build_router(store, config))
    .with_graceful_shutdown(async {
      let _ = tokio::signal::ctrl_c().await;
    })
    .await
    .context("serving HTTP")
}

pub async fn run_stdio(store: Store, caller: Caller) -> anyhow::Result<()> {
  tracing::info!(handle = %caller.handle, "serving MCP over stdio");
  let service = SpecStoreMcp::acting_for(store, caller)
    .serve(stdio())
    .await
    .context("starting stdio transport")?;
  service.waiting().await.context("stdio transport")?;
  Ok(())
}
