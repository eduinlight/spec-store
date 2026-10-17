mod common;

use std::path::Path;

use rmcp::ServiceExt;
use rmcp::model::{CallToolRequestParams, CallToolResult};
use rmcp::service::{RoleClient, RunningService, ServiceError};
use rmcp::transport::{ConfigureCommandExt, TokioChildProcess};
use serde_json::{Value, json};
use spec_store::model::{DocumentKind, Role};
use spec_store::store::Store;

type Client = RunningService<RoleClient, ()>;

async fn connect(db: &Path, as_handle: &str) -> Client {
  let db = db.to_path_buf();
  let as_handle = as_handle.to_string();
  let transport = TokioChildProcess::new(
    tokio::process::Command::new(env!("CARGO_BIN_EXE_spec-store")).configure(|cmd| {
      cmd
        .arg("--db")
        .arg(&db)
        .arg("stdio")
        .arg("--as")
        .arg(&as_handle);
    }),
  )
  .expect("spawn server");
  ().serve(transport).await.expect("initialize")
}

async fn call(client: &Client, name: &str, args: Value) -> Result<Value, ServiceError> {
  let mut params = CallToolRequestParams::new(name.to_string());
  params.arguments = args.as_object().cloned();
  let result: CallToolResult = client.call_tool(params).await?;
  match result.structured_content.clone() {
    Some(value) => Ok(value),
    None => panic!("tool {name} returned no structured content: {result:?}"),
  }
}

fn error_of(err: ServiceError) -> rmcp::ErrorData {
  match err {
    ServiceError::McpError(data) => data,
    other => panic!("expected MCP error, got {other:?}"),
  }
}

fn kind(err: &rmcp::ErrorData) -> String {
  err
    .data
    .as_ref()
    .and_then(|d| d.get("kind"))
    .and_then(Value::as_str)
    .unwrap_or_default()
    .to_string()
}

fn seed(db: &Path) -> Store {
  let store = Store::open(db).unwrap();
  store.create_user(None, "root", false).unwrap();
  let admin = store.caller_by_handle("root").unwrap();
  for h in ["editor", "reader", "outsider"] {
    store.create_user(Some(&admin), h, false).unwrap();
  }
  store
    .create_project(&admin, "demo", "Demo project")
    .unwrap();
  store.create_project(&admin, "other", "Other").unwrap();
  store
    .set_member(&admin, "demo", "editor", Role::Editor)
    .unwrap();
  store
    .set_member(&admin, "demo", "reader", Role::Reader)
    .unwrap();
  store
    .with_tx(|tx| {
      tx.execute(
        "INSERT INTO specs (project_id, number, slug, status, branch)
         VALUES ((SELECT id FROM projects WHERE slug = 'demo'), 1, 'first', 'clarified', '001-first')",
        [],
      )?;
      Ok(())
    })
    .unwrap();
  let editor = store.caller_by_handle("editor").unwrap();
  store
    .put_document(
      &editor,
      "demo",
      1,
      DocumentKind::Spec,
      "# Spec 001 — First\n",
      "initial",
      None,
    )
    .unwrap();
  let owner_like = store.caller_by_handle("root").unwrap();
  store
    .set_member(&admin, "demo", "root", Role::Owner)
    .unwrap();
  store
    .put_constitution(
      &owner_like,
      "demo",
      "# Constitution\n",
      "1.0",
      "ratified",
      None,
    )
    .unwrap();
  store
}

#[tokio::test]
async fn lists_tools_and_projects_for_the_acting_user() {
  let db = common::temp_db_path("mcp-read-list");
  seed(&db);
  let client = connect(&db, "editor").await;

  let tools = client.list_tools(None).await.unwrap();
  let names: Vec<&str> = tools.tools.iter().map(|t| t.name.as_ref()).collect();
  assert!(names.contains(&"list_projects"), "{names:?}");

  let projects = call(&client, "list_projects", json!({})).await.unwrap();
  assert_eq!(
    projects,
    json!({"projects": [{"slug": "demo", "name": "Demo project", "role": "editor"}]})
  );

  let admin = connect(&db, "root").await;
  let projects = call(&admin, "list_projects", json!({})).await.unwrap();
  let slugs: Vec<&str> = projects["projects"]
    .as_array()
    .unwrap()
    .iter()
    .map(|p| p["slug"].as_str().unwrap())
    .collect();
  assert_eq!(slugs, vec!["demo", "other"]);
  client.cancel().await.unwrap();
  admin.cancel().await.unwrap();
}

#[tokio::test]
async fn reads_specs_documents_constitution_and_templates() {
  let db = common::temp_db_path("mcp-read-docs");
  seed(&db);
  let client = connect(&db, "reader").await;

  let tools = client.list_tools(None).await.unwrap();
  let names: Vec<&str> = tools.tools.iter().map(|t| t.name.as_ref()).collect();
  for expected in [
    "list_specs",
    "get_active_spec",
    "get_document",
    "get_constitution",
    "get_template",
  ] {
    assert!(
      names.contains(&expected),
      "missing tool {expected} in {names:?}"
    );
  }

  let specs = call(&client, "list_specs", json!({"project": "demo"}))
    .await
    .unwrap();
  assert_eq!(
    specs,
    json!({"specs": [{"number": 1, "slug": "first", "status": "clarified", "branch": "001-first", "kinds": ["spec"]}]})
  );

  let active = call(&client, "get_active_spec", json!({"project": "demo"}))
    .await
    .unwrap();
  assert_eq!(active["spec"]["number"], 1);
  assert_eq!(active["spec"]["branch"], "001-first");

  let none = call(&client, "get_active_spec", json!({"project": "other"})).await;
  assert_eq!(kind(&error_of(none.unwrap_err())), "forbidden");

  let doc = call(
    &client,
    "get_document",
    json!({"project": "demo", "number": 1, "kind": "spec"}),
  )
  .await
  .unwrap();
  assert_eq!(doc["body"], "# Spec 001 — First\n");
  assert_eq!(doc["revision"]["author"], "editor");
  assert_eq!(doc["revision"]["message"], "initial");
  assert!(doc["revision"]["id"].is_i64());

  let missing = call(
    &client,
    "get_document",
    json!({"project": "demo", "number": 1, "kind": "plan"}),
  )
  .await;
  let err = error_of(missing.unwrap_err());
  assert_eq!(kind(&err), "not_found");
  assert!(err.message.contains("plan"), "{}", err.message);

  let bad_kind = call(
    &client,
    "get_document",
    json!({"project": "demo", "number": 1, "kind": "notes"}),
  )
  .await;
  assert_eq!(kind(&error_of(bad_kind.unwrap_err())), "validation");

  let constitution = call(&client, "get_constitution", json!({"project": "demo"}))
    .await
    .unwrap();
  assert_eq!(constitution["body"], "# Constitution\n");
  assert_eq!(constitution["version"], "1.0");

  let template = call(&client, "get_template", json!({"kind": "tasks"}))
    .await
    .unwrap();
  assert_eq!(template["kind"], "tasks");
  assert!(template["body"].as_str().unwrap().contains("Done when"));
  let no_template = call(&client, "get_template", json!({"kind": "research"})).await;
  assert_eq!(kind(&error_of(no_template.unwrap_err())), "validation");
  client.cancel().await.unwrap();
}

#[tokio::test]
async fn outsider_is_forbidden_without_existence_leak() {
  let db = common::temp_db_path("mcp-read-forbidden");
  seed(&db);
  let client = connect(&db, "outsider").await;

  let existing = error_of(
    call(&client, "list_specs", json!({"project": "demo"}))
      .await
      .unwrap_err(),
  );
  let missing = error_of(
    call(&client, "list_specs", json!({"project": "nope"}))
      .await
      .unwrap_err(),
  );
  assert_eq!(kind(&existing), "forbidden");
  assert_eq!(kind(&missing), "forbidden");
  assert_eq!(existing.message, missing.message);
  assert_eq!(existing.code, missing.code);
  client.cancel().await.unwrap();
}
