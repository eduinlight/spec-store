mod common;

use rmcp::model::{CallToolRequestParams, CallToolResult};
use rmcp::service::ServiceError;
use serde_json::{Value, json};

async fn call(client: &common::Client, name: &str, args: Value) -> Result<Value, ServiceError> {
  let mut params = CallToolRequestParams::new(name.to_string());
  params.arguments = args.as_object().cloned();
  let result: CallToolResult = client.call_tool(params).await?;
  Ok(result.structured_content.unwrap_or(Value::Null))
}

fn kind(error: ServiceError) -> String {
  match error {
    ServiceError::McpError(data) => data.data.unwrap()["kind"].as_str().unwrap().to_string(),
    other => panic!("expected MCP error, got {other:?}"),
  }
}

#[tokio::test]
async fn an_agent_of_one_organization_sees_and_reaches_nothing_of_another() {
  let mcp = common::start().await;
  let store = &mcp.store;
  let alice = common::person(store, "alice");
  let bob = common::person(store, "bob");
  store.create_organization(&alice, "acme", "Acme").unwrap();
  store.create_organization(&bob, "beta", "Beta").unwrap();
  store
    .create_project(&alice, "acme/web", "Acme web")
    .unwrap();
  store.create_project(&bob, "beta/web", "Beta web").unwrap();
  store.create_spec(&alice, "acme/web", "secret").unwrap();

  let client = mcp.connect("bob").await;
  let projects = call(&client, "list_projects", json!({})).await.unwrap();
  assert_eq!(
    projects["projects"],
    json!([{"slug": "beta/web", "name": "Beta web", "role": "owner"}])
  );
  for (tool, args) in [
    ("list_specs", json!({"project": "acme/web"})),
    ("get_active_spec", json!({"project": "acme/web"})),
    ("project_status", json!({"project": "acme/web"})),
    (
      "create_spec",
      json!({"project": "acme/web", "slug": "intrusion"}),
    ),
    (
      "get_document",
      json!({"project": "acme/web", "number": 1, "kind": "spec"}),
    ),
  ] {
    let error = call(&client, tool, args).await.unwrap_err();
    assert_eq!(kind(error), "not_found", "{tool}");
  }
  assert_eq!(store.list_specs(&alice, "acme/web").unwrap().len(), 1);
  client.cancel().await.unwrap();
}

#[tokio::test]
async fn a_guest_agent_lists_only_the_projects_it_was_invited_to() {
  let mcp = common::start().await;
  let store = &mcp.store;
  let alice = common::person(store, "alice");
  let gwen = common::person(store, "gwen");
  store.create_organization(&alice, "acme", "Acme").unwrap();
  store.create_project(&alice, "acme/web", "Web").unwrap();
  store.create_project(&alice, "acme/api", "Api").unwrap();
  let invited = store
    .invite_to_project(
      &alice,
      "acme/web",
      "gwen@example.com",
      sdd_vault_core::model::Role::Editor,
      "en",
      "https://app.example.test/invitations",
    )
    .unwrap();
  let sdd_vault_core::store::invitations::ProjectInvitation::Invited(invited) = invited else {
    panic!("expected an invitation");
  };
  store
    .accept_invitation(&gwen, &invited.token, true)
    .unwrap();

  let client = mcp.connect("gwen").await;
  let projects = call(&client, "list_projects", json!({})).await.unwrap();
  assert_eq!(
    projects["projects"],
    json!([{"slug": "acme/web", "name": "Web", "role": "editor"}])
  );
  let error = call(&client, "list_specs", json!({"project": "acme/api"}))
    .await
    .unwrap_err();
  assert_eq!(kind(error), "not_found");
  client.cancel().await.unwrap();
}
