mod common;

use rmcp::model::{CallToolRequestParams, CallToolResult, Implementation, RequestMetaObject};
use rmcp::service::ServiceError;
use sdd_vault_core::model::{AuditResult, DocumentKind, Role};
use sdd_vault_core::store::AuditEntry;
use sdd_vault_testkit::TestDb;
use serde_json::{Value, json};

use common::{Client, TestMcp};

async fn call(client: &Client, name: &str, args: Value) -> Result<CallToolResult, ServiceError> {
  call_as(client, name, args, None).await
}

async fn call_as(
  client: &Client,
  name: &str,
  args: Value,
  client_name: Option<&str>,
) -> Result<CallToolResult, ServiceError> {
  let mut params = CallToolRequestParams::new(name.to_string());
  params.arguments = args.as_object().cloned();
  if let Some(client_name) = client_name {
    let mut meta = RequestMetaObject::new();
    meta.set_client_info(Implementation::new(client_name, "1.0.0"));
    params.meta = Some(meta);
  }
  client.call_tool(params).await
}

async fn seeded(trusted_proxy_hops: usize) -> TestMcp {
  let mcp = common::start_with_hops(TestDb::new(common::migrate), trusted_proxy_hops).await;
  let store = &mcp.store;
  let root = common::person(store, "root");
  store.create_organization(&root, "acme", "Acme").unwrap();
  for handle in ["editor", "reader"] {
    let member = common::person(store, handle);
    common::join(store, &root, "acme", &member);
  }
  store.create_project(&root, "acme/demo", "Demo").unwrap();
  store
    .set_member(&root, "acme/demo", "editor", Role::Editor)
    .unwrap();
  store
    .set_member(&root, "acme/demo", "reader", Role::Reader)
    .unwrap();
  common::exec(
    store,
    "INSERT INTO specs (project_id, number, slug, status, branch)
     VALUES ((SELECT id FROM projects WHERE slug = 'demo'), 1, 'first', 'draft', '001-first')",
  );
  mcp
}

fn newest(mcp: &TestMcp, action: &str) -> AuditEntry {
  mcp
    .store
    .list_audit(Some("acme/demo"), 50)
    .unwrap()
    .into_iter()
    .find(|e| e.action == action)
    .unwrap_or_else(|| panic!("no {action} entry"))
}

fn put(body: &str) -> Value {
  json!({"project": "acme/demo", "number": 1, "kind": "spec", "body": body, "message": "m"})
}

#[tokio::test]
async fn the_name_the_client_sends_with_its_request_is_the_client() {
  let mcp = seeded(0).await;
  let token = mcp.token("editor");
  let client = common::connect_with_token(&mcp.url, &token).await;
  call_as(&client, "put_document", put("# S\n"), Some("Cursor"))
    .await
    .unwrap();
  let entry = newest(&mcp, "document.put");
  assert_eq!(entry.client.as_deref(), Some("Cursor"));
  assert_eq!(entry.result, AuditResult::Ok);
  let revisions = mcp
    .store
    .list_revisions(
      &mcp.store.caller_by_handle("editor", false).unwrap(),
      "acme/demo",
      Some(1),
      DocumentKind::Spec,
    )
    .unwrap();
  assert_eq!(revisions[0].client.as_deref(), Some("Cursor"));
}

#[tokio::test]
async fn without_a_name_in_the_request_the_registered_name_is_used() {
  let mcp = seeded(0).await;
  mcp
    .keycloak
    .grant_consent("subject-editor", "dynamic-client-1", "Claude Code");
  let token = mcp.token_for_client("editor", "dynamic-client-1");
  let client = common::connect_with_token(&mcp.url, &token).await;
  call(&client, "put_document", put("# S\n")).await.unwrap();
  assert_eq!(
    newest(&mcp, "document.put").client.as_deref(),
    Some("Claude Code")
  );
}

#[tokio::test]
async fn a_client_without_any_name_is_an_mcp_client() {
  let mcp = seeded(0).await;
  let token = mcp.token_for_client("editor", "dynamic-client-2");
  let client = common::connect_with_token(&mcp.url, &token).await;
  call(&client, "put_document", put("# S\n")).await.unwrap();
  assert_eq!(
    newest(&mcp, "document.put").client.as_deref(),
    Some("MCP client")
  );
}

#[tokio::test]
async fn the_registered_name_is_looked_up_once_for_many_requests() {
  let mcp = seeded(0).await;
  mcp
    .keycloak
    .grant_consent("subject-editor", "dynamic-client-3", "Zed");
  let token = mcp.token_for_client("editor", "dynamic-client-3");
  let client = common::connect_with_token(&mcp.url, &token).await;
  call(&client, "put_document", put("# one\n")).await.unwrap();
  mcp.keycloak.clear_consents();
  call(
    &client,
    "create_spec",
    json!({"project": "acme/demo", "slug": "second"}),
  )
  .await
  .unwrap();
  assert_eq!(newest(&mcp, "spec.create").client.as_deref(), Some("Zed"));
}

#[tokio::test]
async fn the_origin_comes_from_the_socket_or_the_trusted_forwarded_entry() {
  let mcp = seeded(0).await;
  let token = mcp.token("editor");
  let client =
    common::connect_with_headers(&mcp.url, &token, &[("x-forwarded-for", "203.0.113.7")]).await;
  call(&client, "put_document", put("# S\n")).await.unwrap();
  assert_eq!(
    newest(&mcp, "document.put").origin.as_deref(),
    Some("127.0.0.1")
  );

  let behind = seeded(2).await;
  let token = behind.token("editor");
  let client = common::connect_with_headers(
    &behind.url,
    &token,
    &[("x-forwarded-for", "198.51.100.99, 203.0.113.7, 10.8.0.1")],
  )
  .await;
  call(&client, "put_document", put("# S\n")).await.unwrap();
  assert_eq!(
    newest(&behind, "document.put").origin.as_deref(),
    Some("203.0.113.7")
  );
}

#[tokio::test]
async fn a_denied_write_and_a_conflict_are_recorded_and_the_errors_stay_the_same() {
  let mcp = seeded(0).await;
  let reader = mcp.connect("reader").await;
  let denied = call_as(&reader, "put_document", put("# S\n"), Some("Cursor"))
    .await
    .unwrap_err();
  match denied {
    ServiceError::McpError(data) => {
      assert_eq!(data.data.unwrap()["kind"], "forbidden");
    }
    other => panic!("expected an MCP error, got {other:?}"),
  }
  let entry = newest(&mcp, "document.put");
  assert_eq!(entry.result, AuditResult::Denied);
  assert_eq!(entry.client.as_deref(), Some("Cursor"));
  assert_eq!(entry.actor, "reader");

  let editor = mcp.connect("editor").await;
  let first = call_as(&editor, "put_document", put("# one\n"), Some("Claude Code"))
    .await
    .unwrap();
  let first = first.structured_content.unwrap()["revision_id"]
    .as_i64()
    .unwrap();
  call_as(
    &editor,
    "put_document",
    json!({"project": "acme/demo", "number": 1, "kind": "spec", "body": "# two\n", "message": "m", "expected_revision": first}),
    Some("Claude Code"),
  )
  .await
  .unwrap();
  let stale = call_as(
    &editor,
    "put_document",
    json!({"project": "acme/demo", "number": 1, "kind": "spec", "body": "# three\n", "message": "m", "expected_revision": first}),
    Some("Claude Code"),
  )
  .await
  .unwrap_err();
  match stale {
    ServiceError::McpError(data) => {
      assert_eq!(data.data.unwrap()["kind"], "conflict");
    }
    other => panic!("expected an MCP error, got {other:?}"),
  }
  let entry = newest(&mcp, "document.put");
  assert_eq!(entry.result, AuditResult::Conflict);
  assert_eq!(entry.client.as_deref(), Some("Claude Code"));
  assert_eq!(
    entry.detail,
    Some(format!("expected r{first}, current r{}", first + 1))
  );
}
