mod common;

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use sdd_vault_api::live::LiveHub;
use serde_json::{Value, json};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::{HeaderName, HeaderValue};

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

#[tokio::test]
async fn a_socket_is_told_to_resync_when_the_hub_reconnects_to_redis() {
  let Some(url) = redis_url() else { return };
  let prefix = unique_prefix();
  let hub = LiveHub::redis_with_prefix(&url, &prefix).unwrap();
  let mut config = common::config();
  config.allowed_hosts.push("127.0.0.1".into());
  let app = common::app_wired(config, None, 4, {
    let hub = hub.clone();
    move |state| state.with_events(Arc::new(sdd_vault_core::events::NoopPublisher), hub)
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
  assert_eq!(
    next_json(&mut socket).await,
    json!({"subscribed": "acme/demo"})
  );

  let client = redis::Client::open(url.as_str()).unwrap();
  let mut connection = client.get_multiplexed_async_connection().await.unwrap();
  let _: redis::Value = redis::cmd("CLIENT")
    .arg("KILL")
    .arg("TYPE")
    .arg("pubsub")
    .query_async(&mut connection)
    .await
    .unwrap();

  let deadline = tokio::time::Instant::now() + Duration::from_secs(15);
  loop {
    let message = tokio::time::timeout_at(deadline, socket.next())
      .await
      .expect("a resync message within fifteen seconds")
      .expect("an open socket")
      .expect("a readable frame");
    if let Message::Text(text) = message {
      let value: Value = serde_json::from_str(text.as_str()).unwrap();
      if value == json!({"resync": true}) {
        break;
      }
    }
  }
  api_server.abort();
}
