mod common;

use axum::body::Body;
use axum::http::{HeaderMap, Request, StatusCode, header};
use http_body_util::BodyExt;
use spec_store::model::{Caller, Role};
use spec_store::server::{ServerConfig, build_router};
use spec_store::store::Store;
use tower::ServiceExt;

struct Fixture {
  store: Store,
  admin: Caller,
  token: String,
  token_id: i64,
}

fn fixture() -> Fixture {
  let store = Store::open(common::temp_db_path("ui-session")).unwrap();
  store.create_user(None, "root", false).unwrap();
  let admin = store.caller_by_handle("root").unwrap();
  store.create_user(Some(&admin), "bob", false).unwrap();
  store
    .create_project(&admin, "demo", "Demo project")
    .unwrap();
  store
    .create_project(&admin, "secret", "Secret project")
    .unwrap();
  store
    .set_member(&admin, "demo", "bob", Role::Editor)
    .unwrap();
  let issued = store.issue_token(Some(&admin), "bob", "laptop").unwrap();
  store.issue_token(Some(&admin), "bob", "phone").unwrap();
  store.issue_token(Some(&admin), "root", "root-cli").unwrap();
  Fixture {
    store,
    admin,
    token: issued.token,
    token_id: issued.id,
  }
}

fn app(f: &Fixture) -> axum::Router {
  build_router(f.store.clone(), ServerConfig::default())
}

fn get(path: &str, cookie: Option<&str>) -> Request<Body> {
  let mut builder = Request::builder()
    .uri(path)
    .header(header::HOST, "localhost");
  if let Some(cookie) = cookie {
    builder = builder.header(header::COOKIE, format!("spec_store_session={cookie}"));
  }
  builder.body(Body::empty()).unwrap()
}

fn post_form(path: &str, body: &str, cookie: Option<&str>) -> Request<Body> {
  let mut builder = Request::builder()
    .method("POST")
    .uri(path)
    .header(header::HOST, "localhost")
    .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded");
  if let Some(cookie) = cookie {
    builder = builder.header(header::COOKIE, format!("spec_store_session={cookie}"));
  }
  builder.body(Body::from(body.to_string())).unwrap()
}

async fn body_text(response: axum::response::Response) -> String {
  let bytes = response.into_body().collect().await.unwrap().to_bytes();
  String::from_utf8_lossy(&bytes).into_owned()
}

fn location(headers: &HeaderMap) -> String {
  headers
    .get(header::LOCATION)
    .map(|v| v.to_str().unwrap().to_string())
    .unwrap_or_default()
}

fn set_cookie(headers: &HeaderMap) -> Option<String> {
  headers
    .get(header::SET_COOKIE)
    .map(|v| v.to_str().unwrap().to_string())
}

fn session_secret(set_cookie: &str) -> String {
  set_cookie
    .strip_prefix("spec_store_session=")
    .unwrap()
    .split(';')
    .next()
    .unwrap()
    .to_string()
}

async fn login(f: &Fixture, token: &str, next: &str) -> axum::response::Response {
  let body = format!("token={token}&next={next}");
  app(f)
    .oneshot(post_form("/ui/login", &body, None))
    .await
    .unwrap()
}

async fn session_for(f: &Fixture) -> String {
  let response = login(f, &f.token, "/ui/").await;
  session_secret(&set_cookie(response.headers()).expect("cookie"))
}

#[tokio::test]
async fn visitors_are_sent_to_the_login_page_with_the_requested_path() {
  let f = fixture();
  let root = app(&f).oneshot(get("/", None)).await.unwrap();
  assert_eq!(root.status(), StatusCode::SEE_OTHER);
  assert_eq!(location(root.headers()), "/ui/");

  for (path, encoded) in [
    ("/ui/", "%2Fui%2F"),
    ("/ui/profile", "%2Fui%2Fprofile"),
    ("/ui/p/demo?view=raw", "%2Fui%2Fp%2Fdemo%3Fview%3Draw"),
  ] {
    let response = app(&f).oneshot(get(path, None)).await.unwrap();
    assert_eq!(response.status(), StatusCode::SEE_OTHER, "{path}");
    assert_eq!(
      location(response.headers()),
      format!("/ui/login?next={encoded}")
    );
    assert!(set_cookie(response.headers()).is_none());
  }

  let stale = app(&f)
    .oneshot(get("/ui/", Some("sss_stale")))
    .await
    .unwrap();
  assert_eq!(stale.status(), StatusCode::SEE_OTHER);
  assert!(location(stale.headers()).starts_with("/ui/login"));
  assert!(
    set_cookie(stale.headers()).unwrap().contains("Max-Age=0"),
    "a stale cookie is cleared"
  );
}

#[tokio::test]
async fn login_page_shows_a_form_and_no_data() {
  let f = fixture();
  let response = app(&f)
    .oneshot(get("/ui/login?next=%2Fui%2Fp%2Fdemo", None))
    .await
    .unwrap();
  assert_eq!(response.status(), StatusCode::OK);
  let text = body_text(response).await;
  assert!(text.contains("<form method=\"post\" action=\"/ui/login\""));
  assert!(text.contains("name=\"token\""));
  assert!(text.contains("type=\"password\""));
  assert!(text.contains("value=\"/ui/p/demo\""));
  assert!(!text.contains("Demo project"));
  assert!(!text.contains("/ui/logout"));

  let evil = app(&f)
    .oneshot(get("/ui/login?next=https%3A%2F%2Fevil.example", None))
    .await
    .unwrap();
  let text = body_text(evil).await;
  assert!(!text.contains("evil.example"));
}

#[tokio::test]
async fn valid_token_creates_a_session_and_redirects_to_next() {
  let f = fixture();
  let response = login(&f, &f.token, "%2Fui%2Fp%2Fdemo").await;
  assert_eq!(response.status(), StatusCode::SEE_OTHER);
  assert_eq!(location(response.headers()), "/ui/p/demo");
  let cookie = set_cookie(response.headers()).expect("Set-Cookie");
  assert!(cookie.starts_with("spec_store_session=sss_"), "{cookie}");
  for attribute in ["Path=/", "HttpOnly", "SameSite=Lax", "Max-Age=2592000"] {
    assert!(cookie.contains(attribute), "{cookie} lacks {attribute}");
  }
  assert!(!cookie.contains("Secure"));
  assert!(!cookie.contains(&f.token));
  assert!(body_text(response).await.is_empty());

  let secure_app = build_router(
    f.store.clone(),
    ServerConfig {
      secure_cookies: true,
      ..ServerConfig::default()
    },
  );
  let response = secure_app
    .oneshot(post_form(
      "/ui/login",
      &format!("token={}&next=", f.token),
      None,
    ))
    .await
    .unwrap();
  assert_eq!(location(response.headers()), "/ui/");
  assert!(set_cookie(response.headers()).unwrap().contains("; Secure"));

  let outside = login(&f, &f.token, "https%3A%2F%2Fevil.example%2F").await;
  assert_eq!(location(outside.headers()), "/ui/");

  let tokens = f.store.list_tokens(Some(&f.admin), "bob").unwrap();
  let used = tokens.iter().find(|t| t.id == f.token_id).unwrap();
  assert!(used.last_used_at.is_some(), "login marks the token as used");
}

#[tokio::test]
async fn invalid_tokens_get_an_error_without_session_or_echo() {
  let f = fixture();
  let revoked = f.store.issue_token(Some(&f.admin), "bob", "old").unwrap();
  f.store.revoke_token(Some(&f.admin), revoked.id).unwrap();
  for token in ["sst_unknown_value_123", revoked.token.as_str(), ""] {
    let response = login(&f, token, "%2Fui%2F").await;
    assert_eq!(response.status(), StatusCode::OK);
    assert!(set_cookie(response.headers()).is_none());
    let text = body_text(response).await;
    assert!(text.contains("not valid"), "{text}");
    assert!(text.contains("name=\"token\""));
    if !token.is_empty() {
      assert!(!text.contains(token), "submitted token echoed");
    }
  }
  let sessions: i64 = f
    .store
    .with_conn(|c| Ok(c.query_row("SELECT count(*) FROM sessions", [], |r| r.get(0))?))
    .unwrap();
  assert_eq!(sessions, 0);
}

#[tokio::test]
async fn signed_in_pages_show_the_viewer_and_end_on_revocation_or_logout() {
  let f = fixture();
  let secret = session_for(&f).await;

  let page = app(&f).oneshot(get("/ui/", Some(&secret))).await.unwrap();
  assert_eq!(page.status(), StatusCode::OK);
  let text = body_text(page).await;
  assert!(text.contains("bob"));
  assert!(text.contains("Demo project"));
  assert!(!text.contains("Secret project"));
  assert!(text.contains("action=\"/ui/logout\""));
  assert!(text.contains("href=\"/ui/profile\""));
  assert!(!text.contains("admin"));

  let mcp = app(&f)
    .oneshot(
      Request::builder()
        .method("POST")
        .uri("/mcp")
        .header(header::HOST, "localhost")
        .header(header::AUTHORIZATION, format!("Bearer {}", f.token))
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::ACCEPT, "application/json, text/event-stream")
        .body(Body::from(
          r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"t","version":"0"}}}"#,
        ))
        .unwrap(),
    )
    .await
    .unwrap();
  assert_eq!(
    mcp.status(),
    StatusCode::OK,
    "MCP keeps working beside the UI"
  );

  let other = session_for(&f).await;
  let logout = app(&f)
    .oneshot(post_form("/ui/logout", "", Some(&other)))
    .await
    .unwrap();
  assert_eq!(logout.status(), StatusCode::SEE_OTHER);
  assert_eq!(location(logout.headers()), "/ui/login");
  assert!(set_cookie(logout.headers()).unwrap().contains("Max-Age=0"));
  let after = app(&f).oneshot(get("/ui/", Some(&other))).await.unwrap();
  assert_eq!(after.status(), StatusCode::SEE_OTHER);
  let still = app(&f).oneshot(get("/ui/", Some(&secret))).await.unwrap();
  assert_eq!(
    still.status(),
    StatusCode::OK,
    "other sessions are untouched"
  );

  f.store.revoke_token(Some(&f.admin), f.token_id).unwrap();
  let revoked = app(&f).oneshot(get("/ui/", Some(&secret))).await.unwrap();
  assert_eq!(revoked.status(), StatusCode::SEE_OTHER);
  assert!(location(revoked.headers()).starts_with("/ui/login"));
}

#[tokio::test]
async fn profile_lists_own_tokens_without_secrets() {
  let f = fixture();
  let secret = session_for(&f).await;
  let page = app(&f)
    .oneshot(get("/ui/profile", Some(&secret)))
    .await
    .unwrap();
  assert_eq!(page.status(), StatusCode::OK);
  let text = body_text(page).await;
  assert!(text.contains("laptop"));
  assert!(text.contains("phone"));
  assert!(!text.contains("root-cli"));
  assert!(text.contains("current session"));
  assert!(!text.contains(&f.token));
  assert!(!text.contains("sst_"));
  assert!(!text.contains(&secret));
  let laptop_row = text
    .split("<tr")
    .find(|row| row.contains("laptop"))
    .expect("laptop row");
  assert!(laptop_row.contains("current session"));
  let phone_row = text
    .split("<tr")
    .find(|row| row.contains("phone"))
    .expect("phone row");
  assert!(!phone_row.contains("current session"));
}

#[tokio::test]
async fn unexpected_hosts_are_rejected_for_the_ui() {
  let f = fixture();
  let evil = Request::builder()
    .uri("/ui/login")
    .header(header::HOST, "evil.example")
    .body(Body::empty())
    .unwrap();
  let response = app(&f).oneshot(evil).await.unwrap();
  assert_eq!(response.status(), StatusCode::MISDIRECTED_REQUEST);

  let configured = build_router(
    f.store.clone(),
    ServerConfig {
      allowed_hosts: vec!["specs.example.com:7391".to_string()],
      ..ServerConfig::default()
    },
  );
  let ok = Request::builder()
    .uri("/ui/login")
    .header(header::HOST, "specs.example.com:7391")
    .body(Body::empty())
    .unwrap();
  assert_eq!(
    configured.oneshot(ok).await.unwrap().status(),
    StatusCode::OK
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
async fn token_and_session_secret_never_leak() {
  let capture = Capture::default();
  let subscriber = tracing_subscriber::fmt()
    .with_writer(capture.clone())
    .with_max_level(tracing_subscriber::filter::LevelFilter::TRACE)
    .with_ansi(false)
    .finish();
  let _guard = tracing::subscriber::set_default(subscriber);

  let f = fixture();
  let bare = f.token.trim_start_matches("sst_").to_string();
  let mut texts = Vec::new();

  let response = login(&f, &f.token, "%2Fui%2F").await;
  let cookie = set_cookie(response.headers()).unwrap();
  let secret = session_secret(&cookie);
  texts.push(format!("{:?}", response.headers()));
  texts.push(body_text(response).await);
  for path in ["/ui/", "/ui/profile", "/ui/login"] {
    let response = app(&f).oneshot(get(path, Some(&secret))).await.unwrap();
    texts.push(format!("{:?}", response.headers()));
    texts.push(body_text(response).await);
  }
  let bad = login(&f, "sst_badtoken", "%2Fui%2F").await;
  texts.push(body_text(bad).await);

  let audit = format!("{:?}", f.store.list_audit(None, 100).unwrap());
  let rows: Vec<String> = f
    .store
    .with_conn(|c| {
      let mut stmt = c.prepare("SELECT secret_hash FROM sessions")?;
      let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
      Ok(rows.collect::<Result<Vec<_>, _>>()?)
    })
    .unwrap();
  let logs = String::from_utf8_lossy(&capture.0.lock().unwrap()).into_owned();

  for (label, text) in [("audit", &audit), ("logs", &logs)]
    .into_iter()
    .chain(texts.iter().map(|t| ("page", t)))
  {
    assert!(!text.contains(&f.token), "{label} leaks the token: {text}");
    assert!(
      !text.contains(&bare),
      "{label} leaks the token secret: {text}"
    );
    assert!(
      !text.contains("sst_badtoken"),
      "{label} echoes a submitted token"
    );
  }
  for (label, text) in [("audit", &audit), ("logs", &logs)] {
    assert!(!text.contains(&secret), "{label} leaks the session secret");
  }
  assert!(rows.iter().all(|h| !h.contains(&secret)));
}
