#![allow(dead_code)]

use std::sync::Arc;

use diesel::connection::SimpleConnection;
use rmcp::ServiceExt;
use rmcp::service::{RoleClient, RunningService};
use rmcp::transport::StreamableHttpClientTransport;
use rmcp::transport::streamable_http_client::StreamableHttpClientTransportConfig;
use spec_store_core::identity::{IdentityConfig, Verifier};
use spec_store_core::model::{Caller, OrgRole};
use spec_store_core::store::Store;
use spec_store_core::store::users::Identity;
use spec_store_mcp::{AppState, McpConfig, router};
use spec_store_testkit::{FakeKeycloak, TestDb, TokenSpec};
use tokio::task::JoinHandle;

pub type Client = RunningService<RoleClient, ()>;

pub const RESOURCE: &str = "https://mcp.example.test/mcp";

pub fn migrate(url: &str) {
  spec_store_core::db::open_with_size(url, 1).expect("migrate template");
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
    scopes: vec!["openid".into(), "spec-store-mcp".into()],
  }
}

pub async fn serve(store: Store, keycloak: &FakeKeycloak) -> (String, JoinHandle<()>) {
  let verifier = Verifier::new(IdentityConfig {
    issuer: keycloak.issuer(),
    jwks_url: keycloak.jwks_url(),
  });
  let state = AppState::new(store, Arc::new(verifier), config(keycloak));
  let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
    .await
    .expect("bind");
  let address = listener.local_addr().expect("address");
  let server = tokio::spawn(async move {
    axum::serve(listener, router(state))
      .await
      .expect("serve mcp");
  });
  (format!("http://{address}"), server)
}

pub async fn start_on(db: TestDb) -> TestMcp {
  let keycloak = FakeKeycloak::start().await;
  let store = Store::open_with_size(&db.url, 4).expect("store");
  let (base, server) = serve(store.clone(), &keycloak).await;
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

pub fn join(store: &Store, owner: &Caller, org: &str, member: &Caller) {
  store
    .set_organization_member(owner, org, &member.handle, OrgRole::Member)
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
