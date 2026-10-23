mod common;

use rmcp::model::{CallToolRequestParams, CallToolResult};
use rmcp::service::ServiceError;
use serde_json::{Value, json};
use spec_store_core::model::{DocumentKind, Role};
use spec_store_core::store::Store;

async fn put(client: &common::Client, body: &str, expected: i64) -> Result<Value, ServiceError> {
  let mut params = CallToolRequestParams::new("put_document");
  params.arguments = json!({
    "project": "acme/demo",
    "number": 1,
    "kind": "spec",
    "body": body,
    "message": "replica write",
    "expected_revision": expected,
  })
  .as_object()
  .cloned();
  let result: CallToolResult = client.call_tool(params).await?;
  Ok(result.structured_content.expect("structured content"))
}

#[tokio::test]
async fn two_instances_accept_exactly_one_of_two_racing_writes() {
  let first = common::start().await;
  let store = &first.store;
  let root = common::person(store, "root");
  store.create_organization(&root, "acme", "Acme").unwrap();
  let editor = common::person(store, "editor");
  common::join(store, &root, "acme", &editor);
  store.create_project(&root, "acme/demo", "Demo").unwrap();
  store
    .set_member(&root, "acme/demo", "editor", Role::Editor)
    .unwrap();
  store.create_spec(&editor, "acme/demo", "first").unwrap();
  let initial = store
    .put_document(
      &editor,
      "acme/demo",
      1,
      DocumentKind::Spec,
      "v1",
      "v1",
      None,
    )
    .unwrap();

  let second_store = Store::open_with_size(&first.db.url, 4).unwrap();
  let (second_base, _second_server) = common::serve(second_store, &first.keycloak).await;
  let token = first.token("editor");
  let client_a = common::connect_with_token(&first.url, &token).await;
  let client_b = common::connect_with_token(&format!("{second_base}/mcp"), &token).await;

  let (a, b) = tokio::join!(
    put(&client_a, "from a", initial),
    put(&client_b, "from b", initial)
  );
  let results = [a, b];
  let winners: Vec<i64> = results
    .iter()
    .filter_map(|r| r.as_ref().ok())
    .map(|v| v["revision_id"].as_i64().unwrap())
    .collect();
  assert_eq!(winners.len(), 1, "{results:?}");
  let loser = results
    .iter()
    .find_map(|r| r.as_ref().err())
    .expect("one conflict");
  match loser {
    ServiceError::McpError(data) => {
      let details = data.data.as_ref().unwrap();
      assert_eq!(details["kind"], "conflict");
      assert_eq!(details["current_revision"].as_i64(), Some(winners[0]));
    }
    other => panic!("expected MCP error, got {other:?}"),
  }
  client_a.cancel().await.unwrap();
  client_b.cancel().await.unwrap();
}
