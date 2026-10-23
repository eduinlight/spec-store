mod common;

use std::sync::Arc;

use rmcp::ServiceExt;
use rmcp::model::CallToolRequestParams;
use rmcp::service::ServiceError;
use rmcp::transport::StreamableHttpClientTransport;
use rmcp::transport::streamable_http_client::StreamableHttpClientTransportConfig;
use serde_json::json;
use spec_store_core::identity::{IdentityConfig, Verifier};
use spec_store_mcp::{AppState as McpState, McpConfig};
use spec_store_testkit::TokenSpec;

const MCP_RESOURCE: &str = "https://mcp.example.test/mcp";

#[tokio::test]
async fn the_same_failing_operation_reports_the_same_error_through_the_api_and_mcp() {
  let app = common::app().await;
  app.organization("alice", "acme").await;
  app
    .json(
      "POST",
      "/api/v1/orgs/acme/projects",
      "alice",
      json!({"slug": "demo", "name": "Demo"}),
    )
    .await;

  let verifier = Verifier::new(IdentityConfig {
    issuer: app.keycloak.issuer(),
    jwks_url: app.keycloak.jwks_url(),
  });
  let mcp_state = McpState::new(
    app.state.store.clone(),
    Arc::new(verifier),
    McpConfig {
      allowed_hosts: Vec::new(),
      resource: MCP_RESOURCE.into(),
      authorization_server: app.keycloak.issuer(),
      scopes: vec!["openid".into()],
    },
  );
  let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
  let address = listener.local_addr().unwrap();
  let server = tokio::spawn(async move {
    axum::serve(listener, spec_store_mcp::router(mcp_state))
      .await
      .unwrap();
  });
  let token = app.keycloak.mint(&TokenSpec::new("alice", MCP_RESOURCE));
  let client = ()
    .serve(StreamableHttpClientTransport::from_config(
      StreamableHttpClientTransportConfig::with_uri(format!("http://{address}/mcp"))
        .auth_header(token),
    ))
    .await
    .unwrap();

  let cases = [
    (
      "get_document",
      json!({"project": "acme/demo", "number": 9, "kind": "spec"}),
      "/api/v1/projects/acme/demo/specs/9/documents/spec",
    ),
    (
      "get_constitution",
      json!({"project": "acme/demo"}),
      "/api/v1/projects/acme/demo/constitution",
    ),
    (
      "list_specs",
      json!({"project": "acme/missing"}),
      "/api/v1/projects/acme/missing/specs",
    ),
  ];
  for (tool, args, path) in cases {
    let mut params = CallToolRequestParams::new(tool);
    params.arguments = args.as_object().cloned();
    let mcp_error = match client.call_tool(params).await.unwrap_err() {
      ServiceError::McpError(data) => data,
      other => panic!("expected MCP error, got {other:?}"),
    };
    let api = common::body_json(app.authorized("GET", path, "alice").await).await;
    assert_eq!(
      api["code"],
      mcp_error.data.as_ref().unwrap()["kind"],
      "{tool}"
    );
    assert_eq!(
      api["message"].as_str().unwrap(),
      mcp_error.message,
      "{tool}"
    );
  }
  client.cancel().await.unwrap();
  server.abort();
}
