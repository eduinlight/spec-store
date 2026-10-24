#![allow(dead_code)]

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use axum::http::{HeaderName, HeaderValue};
use diesel::connection::SimpleConnection;
use rmcp::ServiceExt;
use rmcp::service::{RoleClient, RunningService};
use rmcp::transport::StreamableHttpClientTransport;
use rmcp::transport::streamable_http_client::StreamableHttpClientTransportConfig;
use sdd_vault_core::identity::{IdentityConfig, Verifier};
use sdd_vault_core::keycloak::{AdminConfig, KeycloakAdmin};
use sdd_vault_core::model::{Caller, OrgRole};
use sdd_vault_core::store::Store;
use sdd_vault_core::store::users::Identity;
use sdd_vault_mcp::{AppState, McpConfig, router};
use sdd_vault_testkit::keycloak::{ADMIN_CLIENT_ID, ADMIN_CLIENT_SECRET, REALM};
use sdd_vault_testkit::{FakeKeycloak, TestDb, TokenSpec};
use tokio::task::JoinHandle;

pub type Client = RunningService<RoleClient, ()>;

pub const RESOURCE: &str = "https://mcp.example.test/mcp";

pub fn migrate(url: &str) {
  sdd_vault_core::db::open_with_size(url, 1).expect("migrate template");
}

pub struct TestMcp {
  pub url: String,
  pub base: String,
  pub keycloak: FakeKeycloak,
  pub store: Store,
  pub db: TestDb,
  server: JoinHandle<()>,
}

impl Drop for TestMcp {
  fn drop(&mut self) {
    self.server.abort();
  }
}

pub fn config(keycloak: &FakeKeycloak) -> McpConfig {
  McpConfig {
    allowed_hosts: Vec::new(),
    resource: RESOURCE.into(),
    authorization_server: keycloak.issuer(),
    scopes: vec!["openid".into(), "sdd-vault-mcp".into()],
    trusted_proxy_hops: 0,
  }
}

pub async fn serve(store: Store, keycloak: &FakeKeycloak) -> (String, JoinHandle<()>) {
  serve_with(store, keycloak, config(keycloak)).await
}

pub async fn serve_with(
  store: Store,
  keycloak: &FakeKeycloak,
  config: McpConfig,
) -> (String, JoinHandle<()>) {
  let verifier = Verifier::new(IdentityConfig {
    issuer: keycloak.issuer(),
    jwks_url: keycloak.jwks_url(),
  });
  let admin = KeycloakAdmin::new(AdminConfig {
    base_url: keycloak.base_url.clone(),
    realm: REALM.into(),
    client_id: ADMIN_CLIENT_ID.into(),
    client_secret: ADMIN_CLIENT_SECRET.into(),
  });
  let state = AppState::new(store, Arc::new(verifier), config).with_keycloak_admin(Arc::new(admin));
  let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
    .await
    .expect("bind");
  let address = listener.local_addr().expect("address");
  let server = tokio::spawn(async move {
    axum::serve(
      listener,
      router(state).into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .expect("serve mcp");
  });
  (format!("http://{address}"), server)
}

pub async fn start_on(db: TestDb) -> TestMcp {
  start_with_hops(db, 0).await
}

pub async fn start_with_hops(db: TestDb, trusted_proxy_hops: usize) -> TestMcp {
  let keycloak = FakeKeycloak::start().await;
  let store = Store::open_with_size(&db.url, 4).expect("store");
  let config = McpConfig {
    trusted_proxy_hops,
    ..config(&keycloak)
  };
  let (base, server) = serve_with(store.clone(), &keycloak, config).await;
  TestMcp {
    url: format!("{base}/mcp"),
    base,
    keycloak,
    store,
    db,
    server,
  }
}

pub async fn start() -> TestMcp {
  start_on(TestDb::new(migrate)).await
}

pub fn identity(handle: &str) -> Identity {
  Identity {
    subject: format!("subject-{handle}"),
    handle: handle.to_string(),
    email: format!("{handle}@example.com"),
    name: handle.to_string(),
  }
}

pub fn person(store: &Store, handle: &str) -> Caller {
  store.caller_for(&identity(handle), false).expect("person")
}

pub fn join(store: &Store, _owner: &Caller, org: &str, member: &Caller) {
  store
    .admit_organization_member(org, member.user_id, OrgRole::Member)
    .expect("join organization");
}

pub fn exec(store: &Store, statements: &str) {
  store.with_tx(|c| Ok(c.batch_execute(statements)?)).unwrap()
}

impl TestMcp {
  pub fn token(&self, handle: &str) -> String {
    let spec = TokenSpec::new(handle, RESOURCE);
    let spec = if handle == "ops" {
      spec.operator()
    } else {
      spec
    };
    self.keycloak.mint(&spec)
  }

  pub fn token_for_client(&self, handle: &str, client_id: &str) -> String {
    let mut spec = TokenSpec::new(handle, RESOURCE);
    spec.client_id = client_id.to_string();
    self.keycloak.mint(&spec)
  }

  pub async fn connect(&self, handle: &str) -> Client {
    connect_with_token(&self.url, &self.token(handle)).await
  }
}

pub async fn connect_with_token(url: &str, token: &str) -> Client {
  let transport = StreamableHttpClientTransport::from_config(
    StreamableHttpClientTransportConfig::with_uri(url.to_string()).auth_header(token.to_string()),
  );
  ().serve(transport).await.expect("initialize")
}

pub async fn connect_with_headers(url: &str, token: &str, headers: &[(&str, &str)]) -> Client {
  let mut config =
    StreamableHttpClientTransportConfig::with_uri(url.to_string()).auth_header(token.to_string());
  config.custom_headers = headers
    .iter()
    .map(|(name, value)| {
      (
        HeaderName::from_bytes(name.as_bytes()).unwrap(),
        HeaderValue::from_str(value).unwrap(),
      )
    })
    .collect::<HashMap<_, _>>();
  ().serve(StreamableHttpClientTransport::from_config(config))
    .await
    .expect("initialize")
}
