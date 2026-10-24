mod common;

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use rmcp::ServiceExt;
use rmcp::model::CallToolRequestParams;
use rmcp::transport::StreamableHttpClientTransport;
use rmcp::transport::streamable_http_client::StreamableHttpClientTransportConfig;
use sdd_vault_api::live::LiveHub;
use sdd_vault_core::events::{EventPublisher, ProjectEvent};
use sdd_vault_core::identity::{IdentityConfig, Verifier};
use sdd_vault_core::model::AuditResult;
use sdd_vault_core::redis_events::RedisPublisher;
use sdd_vault_core::store::Store;
use sdd_vault_mcp::{AppState as McpState, McpConfig};
use sdd_vault_testkit::TokenSpec;
use serde_json::{Value, json};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::{HeaderName, HeaderValue};

const MCP_RESOURCE: &str = "https://mcp.example.test/mcp";
const WAIT: Duration = Duration::from_secs(2);

fn redis_url() -> Option<String> {
  match std::env::var("REDIS_URL") {
    Ok(url) if !url.is_empty() => Some(url),
    _ => {
      eprintln!("skipped: REDIS_URL is not set; run `make test-services` and load .env");
      None
    }
  }
}

fn unique_prefix() -> String {
  let nanos = std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)
    .unwrap()
    .as_nanos();
  format!("sddvault-test-{}-{nanos}:project:", std::process::id())
}

fn sample(audit_id: i64) -> ProjectEvent {
  ProjectEvent {
    project_id: 77,
    organization_id: 1,
    project_address: "acme/demo".into(),
    audit_id,
    action: "task.done".into(),
    result: AuditResult::Ok,
  }
}

#[tokio::test]
async fn two_hubs_on_the_same_redis_both_deliver_an_event_published_by_one_publisher() {
  let Some(url) = redis_url() else { return };
  let prefix = unique_prefix();
  let first = LiveHub::redis_with_prefix(&url, &prefix).unwrap();
  let second = LiveHub::redis_with_prefix(&url, &prefix).unwrap();
  assert!(first.wait_ready(WAIT).await);
  assert!(second.wait_ready(WAIT).await);
  let mut seen_first = first.subscribe();
  let mut seen_second = second.subscribe();
  let publisher = RedisPublisher::spawn_with_prefix(&url, &prefix).unwrap();
  publisher.publish(&sample(5));
  let a = tokio::time::timeout(WAIT, seen_first.recv())
    .await
    .unwrap()
    .unwrap();
  let b = tokio::time::timeout(WAIT, seen_second.recv())
    .await
    .unwrap()
    .unwrap();
  assert_eq!(a, sample(5));
  assert_eq!(b, sample(5));
}

#[tokio::test]
async fn a_hub_ignores_other_prefixes() {
  let Some(url) = redis_url() else { return };
  let hub = LiveHub::redis_with_prefix(&url, &unique_prefix()).unwrap();
  assert!(hub.wait_ready(WAIT).await);
  let mut seen = hub.subscribe();
  let stranger = RedisPublisher::spawn_with_prefix(&url, &unique_prefix()).unwrap();
  stranger.publish(&sample(6));
  assert!(
    tokio::time::timeout(Duration::from_millis(500), seen.recv())
      .await
      .is_err()
  );
}

#[tokio::test]
async fn an_unreachable_redis_never_fails_a_write() {
  let publisher = RedisPublisher::spawn("redis://127.0.0.1:1").unwrap();
  for id in 0..10 {
    publisher.publish(&sample(id));
  }
}

#[tokio::test]
async fn an_mcp_write_reaches_a_socket_of_another_instance_through_redis() {
  let Some(url) = redis_url() else { return };
  let prefix = unique_prefix();
  let hub = LiveHub::redis_with_prefix(&url, &prefix).unwrap();
  let api_publisher = RedisPublisher::spawn_with_prefix(&url, &prefix).unwrap();
  let mut config = common::config();
  config.allowed_hosts.push("127.0.0.1".into());
  let app = common::app_wired(config, None, 4, {
    let hub = hub.clone();
    move |state| state.with_events(Arc::new(api_publisher), hub)
  })
  .await;
  assert!(app.state.live.wait_ready(WAIT).await);
  app.organization("alice", "acme").await;
  app
    .json(
      "POST",
      "/api/v1/orgs/acme/projects",
      "alice",
      json!({"slug": "demo", "name": "Demo"}),
    )
    .await;
  let api = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
  let api_address = api.local_addr().unwrap();
  let router = app.router.clone();
  let api_server = tokio::spawn(async move {
    axum::serve(
      api,
      router.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap();
  });

  let database = app.db.as_ref().unwrap().url.clone();
  let mcp_store = Store::open_with_size(&database, 4)
    .unwrap()
    .with_publisher(Arc::new(
      RedisPublisher::spawn_with_prefix(&url, &prefix).unwrap(),
    ));
  let verifier = Verifier::new(IdentityConfig {
    issuer: app.keycloak.issuer(),
    jwks_url: app.keycloak.jwks_url(),
  });
  let mcp_state = McpState::new(
    mcp_store,
    Arc::new(verifier),
    McpConfig {
      allowed_hosts: Vec::new(),
      resource: MCP_RESOURCE.into(),
      authorization_server: app.keycloak.issuer(),
      scopes: vec!["openid".into()],
      trusted_proxy_hops: 0,
    },
  );
  let mcp = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
  let mcp_address = mcp.local_addr().unwrap();
  let mcp_server = tokio::spawn(async move {
    axum::serve(mcp, sdd_vault_mcp::router(mcp_state))
      .await
      .unwrap();
  });

  let mut request = format!("ws://{api_address}/api/v1/live")
    .into_client_request()
    .unwrap();
  request.headers_mut().insert(
    HeaderName::from_static("authorization"),
    HeaderValue::from_str(&format!("Bearer {}", app.token("alice"))).unwrap(),
  );
  let (mut socket, _) = tokio_tungstenite::connect_async(request).await.unwrap();
  socket
    .send(Message::text(json!({"subscribe": "acme/demo"}).to_string()))
    .await
    .unwrap();
  let ack = next_json(&mut socket).await;
  assert_eq!(ack, json!({"subscribed": "acme/demo"}));

  let token = app.keycloak.mint(&TokenSpec::new("alice", MCP_RESOURCE));
  let client = ()
    .serve(StreamableHttpClientTransport::from_config(
      StreamableHttpClientTransportConfig::with_uri(format!("http://{mcp_address}/mcp"))
        .auth_header(token),
    ))
    .await
    .unwrap();
  let mut params = CallToolRequestParams::new("create_spec");
  params.arguments = json!({"project": "acme/demo", "slug": "billing"})
    .as_object()
    .cloned();
  client.call_tool(params).await.unwrap();

  let audit_id = app.state.store.list_audit(Some("acme/demo"), 1).unwrap()[0].id;
  assert_eq!(
    next_json(&mut socket).await,
    json!({
      "project": "acme/demo",
      "audit_id": audit_id,
      "action": "spec.create",
      "result": "ok"
    })
  );
  client.cancel().await.unwrap();
  api_server.abort();
  mcp_server.abort();
}

async fn next_json(
  socket: &mut tokio_tungstenite::WebSocketStream<
    tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
  >,
) -> Value {
  loop {
    let message = tokio::time::timeout(WAIT, socket.next())
      .await
      .expect("a message within two seconds")
      .expect("an open socket")
      .expect("a readable frame");
    match message {
      Message::Text(text) => return serde_json::from_str(text.as_str()).unwrap(),
      Message::Ping(_) | Message::Pong(_) => continue,
      other => panic!("unexpected frame {other:?}"),
    }
  }
}
