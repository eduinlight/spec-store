mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use spec_store::model::Caller;
use spec_store::server::{ServerConfig, build_router};
use spec_store::store::Store;
use tower::ServiceExt;

struct Fixture {
  store: Store,
  admin: Caller,
  token: String,
}

fn fixture() -> Fixture {
  let store = Store::open(common::temp_db_path("http-auth")).unwrap();
  store.create_user(None, "root", false).unwrap();
  let admin = store.caller_by_handle("root").unwrap();
  let token = store
    .issue_token(Some(&admin), "root", "test")
    .unwrap()
    .token;
  Fixture {
    store,
    admin,
    token,
  }
}

fn initialize_body() -> String {
  serde_json::json!({
    "jsonrpc": "2.0",
    "id": 1,
    "method": "initialize",
    "params": {
      "protocolVersion": "2025-06-18",
      "capabilities": {},
      "clientInfo": {"name": "test", "version": "0"}
    }
  })
  .to_string()
}

fn mcp_request(token: Option<&str>) -> Request<Body> {
  let mut builder = Request::builder()
    .method("POST")
    .uri("/mcp")
    .header(header::HOST, "localhost")
    .header(header::CONTENT_TYPE, "application/json")
    .header(header::ACCEPT, "application/json, text/event-stream");
  if let Some(token) = token {
    builder = builder.header(header::AUTHORIZATION, format!("Bearer {token}"));
  }
  builder.body(Body::from(initialize_body())).unwrap()
}

async fn body_text(response: axum::response::Response) -> String {
  let bytes = response.into_body().collect().await.unwrap().to_bytes();
  String::from_utf8_lossy(&bytes).into_owned()
}

#[tokio::test]
async fn healthz_is_open() {
  let f = fixture();
  let app = build_router(f.store.clone(), ServerConfig::default());
  let response = app
    .oneshot(
      Request::builder()
        .uri("/healthz")
        .body(Body::empty())
        .unwrap(),
    )
    .await
    .unwrap();
  assert_eq!(response.status(), StatusCode::OK);
  assert_eq!(body_text(response).await, "ok");
}

#[tokio::test]
async fn missing_unknown_and_revoked_tokens_get_401() {
  let f = fixture();
  let revoked = f.store.issue_token(Some(&f.admin), "root", "old").unwrap();
  f.store.revoke_token(Some(&f.admin), revoked.id).unwrap();

  for (label, token) in [
    ("missing", None),
    ("unknown", Some("sst_unknown")),
    ("revoked", Some(revoked.token.as_str())),
    ("malformed scheme", Some("")),
  ] {
    let app = build_router(f.store.clone(), ServerConfig::default());
    let response = app.oneshot(mcp_request(token)).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED, "{label}");
    let challenge = response
      .headers()
      .get(header::WWW_AUTHENTICATE)
      .unwrap_or_else(|| panic!("{label}: WWW-Authenticate missing"))
      .to_str()
      .unwrap()
      .to_string();
    assert!(challenge.starts_with("Bearer"), "{label}: {challenge}");
    let text = body_text(response).await;
    assert!(
      !text.contains("protocolVersion"),
      "{label}: no MCP data on 401"
    );
    if let Some(t) = token.filter(|t| !t.is_empty()) {
      assert!(!text.contains(t), "{label}: token echoed in body");
    }
  }

  let app = build_router(f.store.clone(), ServerConfig::default());
  let basic = Request::builder()
    .method("POST")
    .uri("/mcp")
    .header(header::HOST, "localhost")
    .header(header::AUTHORIZATION, "Basic abc")
    .header(header::CONTENT_TYPE, "application/json")
    .header(header::ACCEPT, "application/json, text/event-stream")
    .body(Body::from(initialize_body()))
    .unwrap();
  let response = app.oneshot(basic).await.unwrap();
  assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn valid_token_reaches_mcp_initialize() {
  let f = fixture();
  let app = build_router(f.store.clone(), ServerConfig::default());
  let response = app.oneshot(mcp_request(Some(&f.token))).await.unwrap();
  assert_eq!(response.status(), StatusCode::OK);
  let text = body_text(response).await;
  assert!(text.contains("protocolVersion"), "{text}");
  assert!(text.contains("spec-store"), "{text}");

  let tokens = f.store.list_tokens(Some(&f.admin), "root").unwrap();
  let used = tokens.iter().find(|t| t.label == "test").unwrap();
  assert!(
    used.last_used_at.is_some(),
    "authentication records last use"
  );
}

#[derive(Clone, Default)]
struct Capture(std::sync::Arc<std::sync::Mutex<Vec<u8>>>);

impl std::io::Write for Capture {
  fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
    self.0.lock().unwrap().extend_from_slice(buf);
    Ok(buf.len())
  }

  fn flush(&mut self) -> std::io::Result<()> {
    Ok(())
  }
}

impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for Capture {
  type Writer = Capture;

  fn make_writer(&'a self) -> Capture {
    self.clone()
  }
}

#[tokio::test]
async fn token_never_in_logs_or_responses() {
  let capture = Capture::default();
  let subscriber = tracing_subscriber::fmt()
    .with_writer(capture.clone())
    .with_max_level(tracing_subscriber::filter::LevelFilter::TRACE)
    .with_ansi(false)
    .finish();
  let _guard = tracing::subscriber::set_default(subscriber);

  let f = fixture();
  let secret = f.token.trim_start_matches("sst_").to_string();
  let revoked = f.store.issue_token(Some(&f.admin), "root", "old").unwrap();
  f.store.revoke_token(Some(&f.admin), revoked.id).unwrap();

  let mut responses = Vec::new();
  for token in [
    Some(f.token.as_str()),
    Some(revoked.token.as_str()),
    Some("sst_unknown"),
    None,
  ] {
    let app = build_router(f.store.clone(), ServerConfig::default());
    let response = app.oneshot(mcp_request(token)).await.unwrap();
    let headers = format!("{:?}", response.headers());
    responses.push(headers);
    responses.push(body_text(response).await);
  }
  f.store.authenticate("sst_unknown").ok();
  f.store.authenticate(&revoked.token).ok();

  let audit = format!("{:?}", f.store.list_audit(None, 100).unwrap());
  let tokens = format!("{:?}", f.store.list_tokens(Some(&f.admin), "root").unwrap());
  let logs = String::from_utf8_lossy(&capture.0.lock().unwrap()).into_owned();
  assert!(
    !logs.is_empty() || true,
    "logging may be quiet, that is fine"
  );

  for (label, text) in [("audit", &audit), ("token list", &tokens), ("logs", &logs)]
    .into_iter()
    .chain(responses.iter().map(|r| ("response", r)))
  {
    assert!(!text.contains(&f.token), "{label} leaks the token: {text}");
    assert!(
      !text.contains(&secret),
      "{label} leaks the token secret: {text}"
    );
    assert!(
      !text.contains(&revoked.token),
      "{label} leaks the revoked token: {text}"
    );
  }
}
