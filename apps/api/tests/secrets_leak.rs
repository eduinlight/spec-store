mod common;

use std::io::Write;
use std::sync::{Arc, Mutex};

use axum::body::Body;
use axum::http::{Request, StatusCode};
use diesel::QueryableByName;
use diesel::RunQueryDsl;
use diesel::sql_types::Text;
use rmcp::ServiceExt;
use rmcp::model::CallToolRequestParams;
use rmcp::transport::StreamableHttpClientTransport;
use rmcp::transport::streamable_http_client::StreamableHttpClientTransportConfig;
use sdd_vault_core::identity::{IdentityConfig, Verifier};
use sdd_vault_mcp::{AppState as McpState, McpConfig};
use sdd_vault_testkit::TokenSpec;
use sdd_vault_testkit::keycloak::{ADMIN_CLIENT_SECRET, APP_CLIENT_SECRET, CLI_CLIENT_ID, REALM};
use serde_json::{Value, json};

const MCP_RESOURCE: &str = "https://mcp.example.test/mcp";

#[derive(Clone, Default)]
struct CapturedLogs(Arc<Mutex<Vec<u8>>>);

impl Write for CapturedLogs {
  fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
    self.0.lock().unwrap().extend_from_slice(bytes);
    Ok(bytes.len())
  }

  fn flush(&mut self) -> std::io::Result<()> {
    Ok(())
  }
}

impl CapturedLogs {
  fn text(&self) -> String {
    String::from_utf8_lossy(&self.0.lock().unwrap()).into_owned()
  }
}

#[derive(QueryableByName)]
struct StoredRow {
  #[diesel(sql_type = Text)]
  row: String,
}

#[derive(QueryableByName)]
struct TableName {
  #[diesel(sql_type = Text)]
  name: String,
}

#[derive(Default)]
struct Exposed {
  error_bodies: Vec<String>,
  urls: Vec<String>,
}

impl Exposed {
  async fn collect(&mut self, response: axum::http::Response<Body>) -> (StatusCode, String) {
    let status = response.status();
    for name in ["location", "www-authenticate"] {
      for value in response.headers().get_all(name) {
        self.urls.push(value.to_str().unwrap().to_string());
      }
    }
    let body = common::body_text(response).await;
    if status.is_client_error() || status.is_server_error() {
      self.error_bodies.push(body.clone());
    }
    (status, body)
  }
}

fn stored_rows(app: &common::TestApp) -> Vec<String> {
  app
    .state
    .store
    .with_conn(|conn| {
      let tables: Vec<TableName> = diesel::sql_query(
        "SELECT table_name::text AS name FROM information_schema.tables WHERE table_schema = 'public' AND table_type = 'BASE TABLE'",
      )
      .load(conn)?;
      let mut rows = Vec::new();
      for table in tables {
        let loaded: Vec<StoredRow> = diesel::sql_query(format!(
          "SELECT row_to_json(t)::text AS row FROM \"{}\" t",
          table.name
        ))
        .load(conn)?;
        rows.extend(loaded.into_iter().map(|r| format!("{}: {}", table.name, r.row)));
      }
      Ok(rows)
    })
    .unwrap()
}

fn expire_browser_access(app: &common::TestApp) {
  app
    .state
    .store
    .with_conn(|conn| {
      diesel::sql_query(
        "UPDATE browser_sessions SET access_expires_at = now() - interval '1 second'",
      )
      .execute(conn)?;
      Ok(())
    })
    .unwrap();
}

async fn device_login(app: &common::TestApp, username: &str) -> String {
  let http = reqwest::Client::new();
  let base = format!(
    "{}/realms/{REALM}/protocol/openid-connect",
    app.keycloak.base_url
  );
  let authorization: Value = http
    .post(format!("{base}/auth/device"))
    .form(&[("client_id", CLI_CLIENT_ID)])
    .send()
    .await
    .unwrap()
    .json()
    .await
    .unwrap();
  assert!(
    app
      .keycloak
      .approve_device(authorization["user_code"].as_str().unwrap(), username)
  );
  let tokens: Value = http
    .post(format!("{base}/token"))
    .form(&[
      ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
      ("client_id", CLI_CLIENT_ID),
      (
        "device_code",
        authorization["device_code"].as_str().unwrap(),
      ),
    ])
    .send()
    .await
    .unwrap()
    .json()
    .await
    .unwrap();
  tokens["access_token"].as_str().unwrap().to_string()
}

async fn bearer(
  app: &common::TestApp,
  method: &str,
  path: &str,
  token: &str,
  body: Option<Value>,
) -> axum::http::Response<Body> {
  let request = Request::builder()
    .method(method)
    .uri(path)
    .header("host", "api.example.test")
    .header("authorization", format!("Bearer {token}"));
  let request = match body {
    Some(value) => request
      .header("content-type", "application/json")
      .body(Body::from(value.to_string())),
    None => request.body(Body::empty()),
  };
  app.send(request.unwrap()).await
}

#[tokio::test(flavor = "multi_thread")]
async fn issued_tokens_device_codes_and_client_secrets_never_leak() {
  let logs = CapturedLogs::default();
  let writer = logs.clone();
  tracing::subscriber::set_global_default(
    tracing_subscriber::fmt()
      .with_max_level(tracing::Level::TRACE)
      .with_ansi(false)
      .with_writer(move || writer.clone())
      .finish(),
  )
  .unwrap();

  let app = common::app().await;
  let mut exposed = Exposed::default();

  let started = app.start_login("?return_to=/", None).await;
  let authorize_url = common::location(&started);
  exposed.collect(started).await;
  let finished = app.complete_login("alice", &authorize_url).await;
  let cookie = common::set_cookie(&finished)
    .expect("session cookie")
    .split(';')
    .next()
    .unwrap()
    .to_string();
  exposed.collect(finished).await;
  let (status, _) = exposed
    .collect(app.browser("GET", "/api/v1/me", &cookie, false, None).await)
    .await;
  assert_eq!(status, StatusCode::OK);

  expire_browser_access(&app);
  let refreshes = app.keycloak.refresh_grants();
  let (status, _) = exposed
    .collect(
      app
        .browser(
          "POST",
          "/api/v1/orgs",
          &cookie,
          true,
          Some(json!({"slug": "acme", "name": "Acme"})),
        )
        .await,
    )
    .await;
  assert_eq!(status, StatusCode::CREATED);
  assert_eq!(app.keycloak.refresh_grants(), refreshes + 1);
  let (status, _) = exposed
    .collect(
      app
        .browser(
          "POST",
          "/api/v1/orgs/acme/projects",
          &cookie,
          true,
          Some(json!({"slug": "demo", "name": "Demo"})),
        )
        .await,
    )
    .await;
  assert_eq!(status, StatusCode::CREATED);

  let device_access = device_login(&app, "dave").await;
  let (status, _) = exposed
    .collect(bearer(&app, "GET", "/api/v1/me", &device_access, None).await)
    .await;
  assert_eq!(status, StatusCode::OK);
  let (status, _) = exposed
    .collect(
      bearer(
        &app,
        "POST",
        "/api/v1/orgs",
        &device_access,
        Some(json!({"slug": "Not A Slug", "name": "x"})),
      )
      .await,
    )
    .await;
  assert!(status.is_client_error());
  let (status, _) = exposed
    .collect(
      bearer(
        &app,
        "GET",
        "/api/v1/projects/acme/demo/status",
        &device_access,
        None,
      )
      .await,
    )
    .await;
  assert_eq!(status, StatusCode::NOT_FOUND);

  let expired = app
    .keycloak
    .mint(&TokenSpec::new("alice", common::API_AUDIENCE).ttl(-60));
  let (status, _) = exposed
    .collect(bearer(&app, "GET", "/api/v1/me", &expired, None).await)
    .await;
  assert_eq!(status, StatusCode::UNAUTHORIZED);
  let stray_refresh = app.keycloak.issue_refresh_token("alice");
  let (status, _) = exposed
    .collect(bearer(&app, "GET", "/api/v1/me", &stray_refresh, None).await)
    .await;
  assert_eq!(status, StatusCode::UNAUTHORIZED);

  let mcp_state = McpState::new(
    app.state.store.clone(),
    Arc::new(Verifier::new(IdentityConfig {
      issuer: app.keycloak.issuer(),
      jwks_url: app.keycloak.jwks_url(),
    })),
    McpConfig {
      allowed_hosts: Vec::new(),
      resource: MCP_RESOURCE.into(),
      authorization_server: app.keycloak.issuer(),
      scopes: vec!["openid".into()],
    },
  );
  let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
  let address = listener.local_addr().unwrap();
  tokio::spawn(async move {
    axum::serve(listener, sdd_vault_mcp::router(mcp_state))
      .await
      .unwrap();
  });
  let mcp_token = app.keycloak.mint(&TokenSpec::new("alice", MCP_RESOURCE));
  let client = ()
    .serve(StreamableHttpClientTransport::from_config(
      StreamableHttpClientTransportConfig::with_uri(format!("http://{address}/mcp"))
        .auth_header(mcp_token),
    ))
    .await
    .unwrap();
  let mut params = CallToolRequestParams::new("list_specs");
  params.arguments = json!({"project": "acme/demo"}).as_object().cloned();
  client.call_tool(params).await.unwrap();
  let mut params = CallToolRequestParams::new("get_document");
  params.arguments = json!({"project": "acme/demo", "number": 7, "kind": "spec"})
    .as_object()
    .cloned();
  let failure = client.call_tool(params).await.unwrap_err();
  exposed.error_bodies.push(format!("{failure:?}"));
  client.cancel().await.unwrap();

  let http = reqwest::Client::new();
  for token in [expired.as_str(), stray_refresh.as_str()] {
    let response = http
      .post(format!("http://{address}/mcp"))
      .bearer_auth(token)
      .header("accept", "application/json, text/event-stream")
      .json(&json!({"jsonrpc": "2.0", "id": 1, "method": "tools/list"}))
      .send()
      .await
      .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    for value in response.headers().get_all("www-authenticate") {
      exposed.urls.push(value.to_str().unwrap().to_string());
    }
    exposed.error_bodies.push(response.text().await.unwrap());
  }

  let (status, body) = exposed
    .collect(
      app
        .browser("POST", "/auth/logout", &cookie, true, None)
        .await,
    )
    .await;
  assert_eq!(status, StatusCode::OK);
  let logout: Value = serde_json::from_str(&body).unwrap();
  exposed
    .urls
    .push(logout["logout_url"].as_str().unwrap().to_string());

  let mut secrets = app.keycloak.issued_secrets();
  secrets.push(APP_CLIENT_SECRET.into());
  secrets.push(ADMIN_CLIENT_SECRET.into());
  assert!(secrets.iter().any(|s| s.starts_with("device-")));
  assert!(secrets.iter().any(|s| s.starts_with("refresh-")));
  assert!(secrets.len() >= 8, "{} secrets issued", secrets.len());

  let log_text = logs.text();
  assert!(!log_text.is_empty(), "logs were captured");
  let rows = stored_rows(&app);
  assert!(rows.iter().any(|r| r.starts_with("audit_log: ")));
  let places = [
    ("logs", log_text.lines().map(str::to_string).collect()),
    ("stored rows", rows),
    ("error bodies", exposed.error_bodies),
    ("urls", exposed.urls),
  ];
  for (place, texts) in &places {
    for text in texts {
      for secret in &secrets {
        assert!(
          !text.contains(secret.as_str()),
          "a secret starting {:?} appears in {place}: {text}",
          &secret[..secret.len().min(12)]
        );
      }
    }
  }
}
