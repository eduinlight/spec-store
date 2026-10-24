use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::Context;
use sdd_vault_api::{ApiConfig, AppState, router};
use sdd_vault_core::config::{
  DatabaseSettings, Env, SmtpSettings, admin_from_env, identity_from_env,
};
use sdd_vault_core::identity::Verifier;
use sdd_vault_core::keycloak::KeycloakAdmin;
use sdd_vault_core::store::Store;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
  if std::env::args().nth(1).as_deref() == Some("openapi") {
    println!("{}", sdd_vault_api::router::openapi().to_pretty_json()?);
    return Ok(());
  }
  tracing_subscriber::fmt()
    .json()
    .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
    .init();
  let env = Env::system();
  let database = DatabaseSettings::from_env(&env)?;
  let identity = identity_from_env(&env)?;
  let config = ApiConfig {
    allowed_hosts: env.list("ALLOWED_HOSTS"),
    audience: env
      .optional("API_AUDIENCE")
      .unwrap_or_else(|| "sdd-vault-api".into()),
    app_url: env.required("APP_URL")?,
    landing_url: env.required("LANDING_URL")?,
    app_client_id: env
      .optional("KEYCLOAK_APP_CLIENT_ID")
      .unwrap_or_else(|| "sdd-vault-app".into()),
    app_client_secret: env.required("KEYCLOAK_APP_CLIENT_SECRET")?,
    session_key: env.required("SESSION_SEALING_KEY")?,
    trusted_proxy_hops: env.parsed("TRUSTED_PROXY_HOPS", 0)?,
    mcp_url: env.optional("MCP_RESOURCE"),
  };
  let smtp = SmtpSettings::from_env(&env)?;
  let bind = env
    .optional("API_BIND")
    .unwrap_or_else(|| "127.0.0.1:4000".into());
  let store =
    tokio::task::spawn_blocking(move || Store::open_with_size(&database.url, database.pool_size))
      .await?
      .context("opening the database and applying migrations")?;
  let mut state = AppState::try_new(store, Arc::new(Verifier::new(identity)), config)
    .context("SESSION_SEALING_KEY must be 32 bytes in base64")?;
  match admin_from_env(&env)? {
    Some(admin) => state = state.with_keycloak_admin(Arc::new(KeycloakAdmin::new(admin))),
    None => tracing::warn!(
      "KEYCLOAK_ADMIN_CLIENT_SECRET is not set: authorized clients cannot be listed or revoked"
    ),
  }
  let _sender = sdd_vault_api::outbox::Sender::new(state.store.clone(), smtp)
    .spawn(sdd_vault_api::outbox::INTERVAL);
  let listener = tokio::net::TcpListener::bind(&bind)
    .await
    .with_context(|| format!("binding {bind}"))?;
  tracing::info!(%bind, "SDD Vault API listening");
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
