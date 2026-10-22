mod common;

use serde_json::{Value, json};
use spec_store_testkit::TokenSpec;

const METADATA: &str = "https://mcp.example.test/.well-known/oauth-protected-resource/mcp";

fn initialize() -> Value {
  json!({
    "jsonrpc": "2.0",
    "id": 1,
    "method": "initialize",
    "params": {
      "protocolVersion": "2025-06-18",
      "capabilities": {},
      "clientInfo": { "name": "test", "version": "0" }
    }
  })
}

async fn post(mcp: &common::TestMcp, token: Option<&str>) -> reqwest::Response {
  let mut request = reqwest::Client::new()
    .post(&mcp.url)
    .header("accept", "application/json, text/event-stream")
    .json(&initialize());
  if let Some(token) = token {
    request = request.bearer_auth(token);
  }
  request.send().await.unwrap()
}

fn challenge(response: &reqwest::Response) -> String {
  response
    .headers()
    .get("www-authenticate")
    .expect("WWW-Authenticate header")
    .to_str()
    .unwrap()
    .to_string()
}

#[tokio::test]
async fn unauthenticated_request_is_told_where_to_authorize() {
  let mcp = common::start().await;
  let response = post(&mcp, None).await;
  assert_eq!(response.status(), 401);
  let header = challenge(&response);
  assert!(header.starts_with("Bearer "), "{header}");
  assert!(
    header.contains(&format!("resource_metadata=\"{METADATA}\"")),
    "{header}"
  );
  assert!(
    header.contains("scope=\"openid spec-store-mcp\""),
    "{header}"
  );
}

#[tokio::test]
async fn protected_resource_metadata_names_the_authorization_server() {
  let mcp = common::start().await;
  for path in [
    "/.well-known/oauth-protected-resource/mcp",
    "/.well-known/oauth-protected-resource",
  ] {
    let response = reqwest::get(format!("{}{path}", mcp.base)).await.unwrap();
    assert_eq!(response.status(), 200, "{path}");
    let body: Value = response.json().await.unwrap();
    assert_eq!(body["resource"], common::RESOURCE, "{path}");
    assert_eq!(
      body["authorization_servers"],
      json!([mcp.keycloak.issuer()]),
      "{path}"
    );
    assert_eq!(
      body["scopes_supported"],
      json!(["openid", "spec-store-mcp"]),
      "{path}"
    );
    assert_eq!(
      body["bearer_methods_supported"],
      json!(["header"]),
      "{path}"
    );
  }
}

#[tokio::test]
async fn token_for_another_audience_is_rejected() {
  let mcp = common::start().await;
  let token = mcp
    .keycloak
    .mint(&TokenSpec::new("alice", "spec-store-api"));
  let response = post(&mcp, Some(&token)).await;
  assert_eq!(response.status(), 401);
  let header = challenge(&response);
  assert!(header.contains("error=\"invalid_token\""), "{header}");
  assert!(
    header.contains(&format!("resource_metadata=\"{METADATA}\"")),
    "{header}"
  );
}

#[tokio::test]
async fn expired_token_is_rejected() {
  let mcp = common::start().await;
  let token = mcp
    .keycloak
    .mint(&TokenSpec::new("alice", common::RESOURCE).ttl(-1));
  let response = post(&mcp, Some(&token)).await;
  assert_eq!(response.status(), 401);
  assert!(challenge(&response).contains("error=\"invalid_token\""));
}

#[tokio::test]
async fn valid_token_is_accepted() {
  let mcp = common::start().await;
  let response = post(&mcp, Some(&mcp.token("alice"))).await;
  assert_eq!(response.status(), 200);
}
