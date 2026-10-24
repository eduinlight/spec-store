use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::Context;
use sdd_vault_core::config::{DatabaseSettings, Env, admin_from_env, identity_from_env};
use sdd_vault_core::identity::Verifier;
use sdd_vault_core::keycloak::KeycloakAdmin;
use sdd_vault_core::redis_events::RedisPublisher;
use sdd_vault_core::store::Store;
use sdd_vault_mcp::{AppState, McpConfig, router};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
  tracing_subscriber::fmt()
    .json()
    .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
    .init();
  let env = Env::system();
  let database = DatabaseSettings::from_env(&env)?;
  let identity = identity_from_env(&env)?;
  let config = McpConfig {
    allowed_hosts: env.list("ALLOWED_HOSTS"),
    resource: env.required("MCP_RESOURCE")?,
    authorization_server: identity.issuer.clone(),
    scopes: env
      .optional("MCP_SCOPES")
      .unwrap_or_else(|| "openid sdd-vault-mcp".into())
      .split_whitespace()
      .map(str::to_string)
      .collect(),
    trusted_proxy_hops: env.parsed("TRUSTED_PROXY_HOPS", 0)?,
  };
  let bind = env
    .optional("MCP_BIND")
    .unwrap_or_else(|| "127.0.0.1:4100".into());
  let store =
    tokio::task::spawn_blocking(move || Store::open_with_size(&database.url, database.pool_size))
      .await?
      .context("opening the database and applying migrations")?;
  let store = match env.optional("REDIS_URL") {
    Some(url) => store.with_publisher(Arc::new(
      RedisPublisher::spawn(&url).context("REDIS_URL must be a Redis connection URL")?,
    )),
    None => {
      tracing::warn!("REDIS_URL is not set: writes through MCP do not reach live pages");
      store
    }
  };
  let mut state = AppState::new(store, Arc::new(Verifier::new(identity)), config);
  match admin_from_env(&env)? {
    Some(admin) => state = state.with_keycloak_admin(Arc::new(KeycloakAdmin::new(admin))),
    None => tracing::warn!(
      "KEYCLOAK_ADMIN_CLIENT_SECRET is not set: clients that send no name are recorded as MCP client"
    ),
  }
  let listener = tokio::net::TcpListener::bind(&bind)
    .await
    .with_context(|| format!("binding {bind}"))?;
  tracing::info!(%bind, "SDD Vault MCP server listening");
  axum::serve(
    listener,
    router(state).into_make_service_with_connect_info::<SocketAddr>(),
  )
  .with_graceful_shutdown(shutdown_signal())
  .await
  .context("serving HTTP")
}

async fn shutdown_signal() {
  let terminate = async {
    match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
      Ok(mut signal) => {
        signal.recv().await;
      }
      Err(_) => std::future::pending::<()>().await,
    }
  };
  tokio::select! {
    _ = tokio::signal::ctrl_c() => {}
    _ = terminate => {}
  }
}
