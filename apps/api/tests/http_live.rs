mod common;

use std::net::SocketAddr;
use std::time::Duration;

use axum::http::StatusCode;
use futures_util::{SinkExt, StreamExt};
use sdd_vault_api::live::LiveSettings;
use sdd_vault_core::model::Role;
use serde_json::{Value, json};
use tokio::net::TcpStream;
use tokio::task::JoinHandle;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::{HeaderName, HeaderValue};
use tokio_tungstenite::tungstenite::{self, Message};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

pub type Socket = WebSocketStream<MaybeTlsStream<TcpStream>>;

const WAIT: Duration = Duration::from_secs(2);

struct Server {
  address: SocketAddr,
  handle: JoinHandle<()>,
}

impl Drop for Server {
  fn drop(&mut self) {
    self.handle.abort();
  }
}

fn live_config() -> sdd_vault_api::ApiConfig {
  let mut config = common::config();
  config.allowed_hosts.push("127.0.0.1".into());
  config
}

fn fast() -> LiveSettings {
  LiveSettings {
    ping_interval: Duration::from_millis(150),
    recheck_interval: Duration::from_millis(150),
  }
}

async fn serve(app: &common::TestApp) -> Server {
  let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
  let address = listener.local_addr().unwrap();
  let router = app.router.clone();
  let handle = tokio::spawn(async move {
    axum::serve(
      listener,
      router.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap();
  });
  Server { address, handle }
}

async fn seeded() -> (common::TestApp, Server) {
  let app = common::app_wired(live_config(), None, 4, |state| {
    state.with_in_process_events().with_live_settings(fast())
  })
  .await;
  seed(&app).await;
  let server = serve(&app).await;
  (app, server)
}

async fn seed(app: &common::TestApp) {
  app.organization("alice", "acme").await;
  app.admit("acme", "rita");
  for slug in ["demo", "secret"] {
    let created = app
      .json(
        "POST",
        "/api/v1/orgs/acme/projects",
        "alice",
        json!({"slug": slug, "name": slug}),
      )
      .await;
    assert_eq!(created.status(), StatusCode::CREATED);
  }
  let alice = app.person("alice");
  app
    .state
    .store
    .set_member(&alice, "acme/demo", "rita", Role::Reader)
    .unwrap();
}

fn bearer(app: &common::TestApp, user: &str) -> Vec<(&'static str, String)> {
  vec![
    ("authorization", format!("Bearer {}", app.token(user))),
    ("origin", "https://elsewhere.example".into()),
  ]
}

async fn cookie(app: &common::TestApp, user: &str) -> Vec<(&'static str, String)> {
  vec![
    ("cookie", app.browser_sign_in(user).await),
    ("origin", common::APP_URL.into()),
  ]
}

async fn connect(
  server: &Server,
  headers: Vec<(&'static str, String)>,
) -> Result<Socket, tungstenite::Error> {
  let mut request = format!("ws://{}/api/v1/live", server.address)
    .into_client_request()
    .unwrap();
  for (name, value) in headers {
    request.headers_mut().insert(
      HeaderName::from_static(name),
      HeaderValue::from_str(&value).unwrap(),
    );
  }
  tokio_tungstenite::connect_async(request)
    .await
    .map(|(socket, _)| socket)
}

async fn next_json(socket: &mut Socket) -> Value {
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

async fn send(socket: &mut Socket, value: Value) {
  socket.send(Message::text(value.to_string())).await.unwrap();
}

async fn subscribe(socket: &mut Socket, project: &str) -> Value {
  send(socket, json!({"subscribe": project})).await;
  next_json(socket).await
}

async fn silent(socket: &mut Socket) {
  let outcome = tokio::time::timeout(Duration::from_millis(500), async {
    loop {
      match socket.next().await {
        Some(Ok(Message::Ping(_) | Message::Pong(_))) => continue,
        other => return other,
      }
    }
  })
  .await;
  assert!(outcome.is_err(), "unexpected traffic {outcome:?}");
}

async fn closed(socket: &mut Socket) {
  let outcome = tokio::time::timeout(Duration::from_secs(3), async {
    loop {
      match socket.next().await {
        Some(Ok(Message::Text(text))) => panic!("unexpected message {text}"),
        Some(Ok(_)) => continue,
        Some(Err(_)) | None => return,
      }
    }
  })
  .await;
  assert!(outcome.is_ok(), "the socket stayed open");
}

fn last_audit_id(app: &common::TestApp, project: &str) -> i64 {
  app.state.store.list_audit(Some(project), 1).unwrap()[0].id
}

fn http_status(error: tungstenite::Error) -> StatusCode {
  match error {
    tungstenite::Error::Http(response) => response.status(),
    other => panic!("expected an HTTP refusal, got {other:?}"),
  }
}

#[tokio::test]
async fn an_upgrade_without_credentials_is_refused_with_401() {
  let (_app, server) = seeded().await;
  let error = connect(&server, vec![("origin", common::APP_URL.into())])
    .await
    .unwrap_err();
  assert_eq!(http_status(error), StatusCode::UNAUTHORIZED);
  let error = connect(
    &server,
    vec![("authorization", "Bearer not-a-token".into())],
  )
  .await
  .unwrap_err();
  assert_eq!(http_status(error), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn a_session_cookie_from_another_origin_is_refused_but_a_bearer_token_is_not() {
  let (app, server) = seeded().await;
  let mut headers = cookie(&app, "rita").await;
  headers[1].1 = "https://evil.example".into();
  let error = connect(&server, headers).await.unwrap_err();
  assert_eq!(http_status(error), StatusCode::FORBIDDEN);
  let mut headers = cookie(&app, "rita").await;
  headers.pop();
  let error = connect(&server, headers).await.unwrap_err();
  assert_eq!(http_status(error), StatusCode::FORBIDDEN);

  let mut socket = connect(&server, bearer(&app, "rita")).await.unwrap();
  assert_eq!(
    subscribe(&mut socket, "acme/demo").await,
    json!({"subscribed": "acme/demo"})
  );
}

#[tokio::test]
async fn a_browser_session_can_subscribe_to_its_project() {
  let (app, server) = seeded().await;
  let mut socket = connect(&server, cookie(&app, "rita").await).await.unwrap();
  assert_eq!(
    subscribe(&mut socket, "acme/demo").await,
    json!({"subscribed": "acme/demo"})
  );
}

#[tokio::test]
async fn subscribing_to_a_project_the_caller_cannot_read_is_refused_without_a_trace() {
  let (app, server) = seeded().await;
  let mut socket = connect(&server, bearer(&app, "rita")).await.unwrap();
  for project in ["acme/secret", "acme/missing", "not an address"] {
    assert_eq!(
      subscribe(&mut socket, project).await,
      json!({"error": "not_found", "project": project})
    );
  }
  let written = app
    .json(
      "PUT",
      "/api/v1/projects/acme/secret/members/rita",
      "alice",
      json!({"role": "reader"}),
    )
    .await;
  assert_eq!(written.status(), StatusCode::NO_CONTENT);
  silent(&mut socket).await;
}

#[tokio::test]
async fn an_api_write_reaches_a_subscribed_socket_within_two_seconds() {
  let (app, server) = seeded().await;
  let mut socket = connect(&server, bearer(&app, "rita")).await.unwrap();
  subscribe(&mut socket, "acme/demo").await;
  let written = app
    .json(
      "PUT",
      "/api/v1/projects/acme/demo/members/rita",
      "alice",
      json!({"role": "editor"}),
    )
    .await;
  assert_eq!(written.status(), StatusCode::NO_CONTENT);
  assert_eq!(
    next_json(&mut socket).await,
    json!({
      "project": "acme/demo",
      "audit_id": last_audit_id(&app, "acme/demo"),
      "action": "member.set",
      "result": "ok"
    })
  );
}

#[tokio::test]
async fn a_refused_write_reaches_the_socket_too() {
  let (app, server) = seeded().await;
  let mut socket = connect(&server, bearer(&app, "alice")).await.unwrap();
  subscribe(&mut socket, "acme/demo").await;
  let refused = app
    .json(
      "PUT",
      "/api/v1/projects/acme/demo/members/rita",
      "rita",
      json!({"role": "owner"}),
    )
    .await;
  assert_eq!(refused.status(), StatusCode::FORBIDDEN);
  assert_eq!(
    next_json(&mut socket).await,
    json!({
      "project": "acme/demo",
      "audit_id": last_audit_id(&app, "acme/demo"),
      "action": "member.set",
      "result": "denied"
    })
  );
}

#[tokio::test]
async fn a_socket_only_receives_the_projects_it_subscribed_to() {
  let (app, server) = seeded().await;
  let mut socket = connect(&server, bearer(&app, "alice")).await.unwrap();
  subscribe(&mut socket, "acme/demo").await;
  app
    .json(
      "PUT",
      "/api/v1/projects/acme/secret/members/rita",
      "alice",
      json!({"role": "reader"}),
    )
    .await;
  silent(&mut socket).await;
  app
    .json(
      "PUT",
      "/api/v1/projects/acme/demo/members/rita",
      "alice",
      json!({"role": "editor"}),
    )
    .await;
  assert_eq!(next_json(&mut socket).await["project"], "acme/demo");
}

#[tokio::test]
async fn unsubscribing_stops_the_events() {
  let (app, server) = seeded().await;
  let mut socket = connect(&server, bearer(&app, "alice")).await.unwrap();
  subscribe(&mut socket, "acme/demo").await;
  send(&mut socket, json!({"unsubscribe": "acme/demo"})).await;
  assert_eq!(
    next_json(&mut socket).await,
    json!({"unsubscribed": "acme/demo"})
  );
  app
    .json(
      "PUT",
      "/api/v1/projects/acme/demo/members/rita",
      "alice",
      json!({"role": "editor"}),
    )
    .await;
  silent(&mut socket).await;
}

#[tokio::test]
async fn malformed_messages_get_an_error_and_keep_the_socket_open() {
  let (app, server) = seeded().await;
  let mut socket = connect(&server, bearer(&app, "rita")).await.unwrap();
  socket.send(Message::text("not json")).await.unwrap();
  assert_eq!(
    next_json(&mut socket).await,
    json!({"error": "invalid_message"})
  );
  send(&mut socket, json!({"nothing": true})).await;
  assert_eq!(
    next_json(&mut socket).await,
    json!({"error": "invalid_message"})
  );
  assert_eq!(
    subscribe(&mut socket, "acme/demo").await,
    json!({"subscribed": "acme/demo"})
  );
}

#[tokio::test]
async fn ending_the_browser_session_closes_the_socket() {
  let (app, server) = seeded().await;
  let headers = cookie(&app, "rita").await;
  let session = headers[0].1.clone();
  let mut socket = connect(&server, headers).await.unwrap();
  subscribe(&mut socket, "acme/demo").await;
  let logout = app
    .browser("POST", "/auth/logout", &session, true, None)
    .await;
  assert_eq!(logout.status(), StatusCode::OK);
  closed(&mut socket).await;
}

#[tokio::test]
async fn losing_access_to_a_project_drops_its_subscription() {
  let (app, server) = seeded().await;
  let mut socket = connect(&server, bearer(&app, "rita")).await.unwrap();
  subscribe(&mut socket, "acme/demo").await;
  let removed = app
    .authorized("DELETE", "/api/v1/projects/acme/demo/members/rita", "alice")
    .await;
  assert_eq!(removed.status(), StatusCode::NO_CONTENT);
  assert_eq!(next_json(&mut socket).await["action"], "member.remove");
  assert_eq!(
    next_json(&mut socket).await,
    json!({"error": "not_found", "project": "acme/demo"})
  );
  app
    .json(
      "PUT",
      "/api/v1/projects/acme/demo/members/rita",
      "alice",
      json!({"role": "reader"}),
    )
    .await;
  silent(&mut socket).await;
}

#[tokio::test]
async fn a_client_that_never_answers_pings_is_dropped_and_a_responsive_one_is_kept() {
  let (app, server) = seeded().await;
  let mut lively = connect(&server, bearer(&app, "rita")).await.unwrap();
  let mut dead = connect(&server, bearer(&app, "alice")).await.unwrap();
  subscribe(&mut lively, "acme/demo").await;
  let keep_reading = tokio::spawn(async move {
    let until = tokio::time::Instant::now() + Duration::from_millis(900);
    while tokio::time::Instant::now() < until {
      let _ = tokio::time::timeout(Duration::from_millis(50), lively.next()).await;
    }
    lively
  });
  tokio::time::sleep(Duration::from_millis(900)).await;
  let mut lively = keep_reading.await.unwrap();
  assert_eq!(
    subscribe(&mut lively, "acme/demo").await,
    json!({"subscribed": "acme/demo"})
  );
  closed(&mut dead).await;
}
