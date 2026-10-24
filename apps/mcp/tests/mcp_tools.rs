mod common;

use rmcp::model::{CallToolRequestParams, CallToolResult};
use rmcp::service::ServiceError;
use sdd_vault_core::model::{DocumentKind, Role};
use serde_json::{Value, json};

use common::{Client, TestMcp};

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

async fn seed() -> TestMcp {
  let mcp = common::start().await;
  let store = &mcp.store;
  let root = common::person(store, "root");
  store.create_organization(&root, "acme", "Acme").unwrap();
  for h in ["editor", "reader"] {
    let member = common::person(store, h);
    common::join(store, &root, "acme", &member);
  }
  common::person(store, "outsider");
  store
    .create_project(&root, "acme/demo", "Demo project")
    .unwrap();
  store.create_project(&root, "acme/other", "Other").unwrap();
  store
    .set_member(&root, "acme/demo", "editor", Role::Editor)
    .unwrap();
  store
    .set_member(&root, "acme/demo", "reader", Role::Reader)
    .unwrap();
  common::exec(
    store,
    "INSERT INTO specs (project_id, number, slug, status, branch)
     VALUES ((SELECT id FROM projects WHERE slug = 'demo'), 1, 'first', 'clarified', '001-first')",
  );
  let editor = store.caller_by_handle("editor", false).unwrap();
  store
    .put_document(
      &editor,
      "acme/demo",
      1,
      DocumentKind::Spec,
      "# Spec 001 — First\n",
      "initial",
      None,
    )
    .unwrap();
  store
    .put_constitution(
      &root,
      "acme/demo",
      "# Constitution\n",
      "1.0",
      "ratified",
      None,
    )
    .unwrap();
  mcp
}

#[tokio::test]
async fn lists_tools_and_projects_for_the_acting_user() {
  let mcp = seed().await;
  let client = mcp.connect("editor").await;

  let tools = client.list_tools(None).await.unwrap();
  let names: Vec<&str> = tools.tools.iter().map(|t| t.name.as_ref()).collect();
  assert!(names.contains(&"list_projects"), "{names:?}");

  let projects = call(&client, "list_projects", json!({})).await.unwrap();
  assert_eq!(
    projects,
    json!({"projects": [{"slug": "acme/demo", "name": "Demo project", "role": "editor"}]})
  );

  let admin = mcp.connect("root").await;
  let projects = call(&admin, "list_projects", json!({})).await.unwrap();
  let slugs: Vec<&str> = projects["projects"]
    .as_array()
    .unwrap()
    .iter()
    .map(|p| p["slug"].as_str().unwrap())
    .collect();
  assert_eq!(slugs, vec!["acme/demo", "acme/other"]);
  client.cancel().await.unwrap();
  admin.cancel().await.unwrap();
}

#[tokio::test]
async fn reads_specs_documents_constitution_and_templates() {
  let mcp = seed().await;
  let client = mcp.connect("reader").await;

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

  let specs = call(&client, "list_specs", json!({"project": "acme/demo"}))
    .await
    .unwrap();
  assert_eq!(
    specs,
    json!({"specs": [{"number": 1, "slug": "first", "status": "clarified", "branch": "001-first", "kinds": ["spec"]}]})
  );

  let active = call(&client, "get_active_spec", json!({"project": "acme/demo"}))
    .await
    .unwrap();
  assert_eq!(active["spec"]["number"], 1);
  assert_eq!(active["spec"]["branch"], "001-first");

  let none = call(&client, "get_active_spec", json!({"project": "acme/other"})).await;
  assert_eq!(kind(&error_of(none.unwrap_err())), "not_found");

  let doc = call(
    &client,
    "get_document",
    json!({"project": "acme/demo", "number": 1, "kind": "spec"}),
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
    json!({"project": "acme/demo", "number": 1, "kind": "plan"}),
  )
  .await;
  let err = error_of(missing.unwrap_err());
  assert_eq!(kind(&err), "not_found");
  assert!(err.message.contains("plan"), "{}", err.message);

  let bad_kind = call(
    &client,
    "get_document",
    json!({"project": "acme/demo", "number": 1, "kind": "notes"}),
  )
  .await;
  assert_eq!(kind(&error_of(bad_kind.unwrap_err())), "validation");

  let constitution = call(&client, "get_constitution", json!({"project": "acme/demo"}))
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
async fn outsider_gets_not_found_without_existence_leak() {
  let mcp = seed().await;
  let client = mcp.connect("outsider").await;

  let existing = error_of(
    call(&client, "list_specs", json!({"project": "acme/demo"}))
      .await
      .unwrap_err(),
  );
  let missing = error_of(
    call(&client, "list_specs", json!({"project": "acme/nope"}))
      .await
      .unwrap_err(),
  );
  assert_eq!(kind(&existing), "not_found");
  assert_eq!(kind(&missing), "not_found");
  assert_eq!(
    existing.message.replace("demo", "X"),
    missing.message.replace("nope", "X")
  );
  assert_eq!(existing.code, missing.code);
  client.cancel().await.unwrap();
}

const TASKS_BODY: &str = "# Tasks — Spec 002\n\n## Phase 1 — Setup\n- [ ] T01 (FR: —) Create the crate — Done when: `cargo build` passes\n- [ ] T02 (FR-1) Implement — Done when: tests green\n";

#[tokio::test]
async fn writes_specs_documents_status_and_tasks() {
  let mcp = seed().await;
  let client = mcp.connect("editor").await;

  let tools = client.list_tools(None).await.unwrap();
  let names: Vec<&str> = tools.tools.iter().map(|t| t.name.as_ref()).collect();
  for expected in [
    "create_spec",
    "put_document",
    "put_constitution",
    "set_spec_status",
    "mark_task_done",
  ] {
    assert!(
      names.contains(&expected),
      "missing tool {expected} in {names:?}"
    );
  }

  let created = call(
    &client,
    "create_spec",
    json!({"project": "acme/demo", "slug": "second"}),
  )
  .await
  .unwrap();
  assert_eq!(
    created,
    json!({"number": 2, "slug": "second", "branch": "002-second", "status": "draft"})
  );
  let dup = call(
    &client,
    "create_spec",
    json!({"project": "acme/demo", "slug": "second"}),
  )
  .await;
  assert_eq!(kind(&error_of(dup.unwrap_err())), "validation");

  let put = call(
    &client,
    "put_document",
    json!({"project": "acme/demo", "number": 2, "kind": "tasks", "body": TASKS_BODY, "message": "tasks"}),
  )
  .await
  .unwrap();
  let first = put["revision_id"].as_i64().expect("revision id");

  let conflict = call(
    &client,
    "put_document",
    json!({"project": "acme/demo", "number": 2, "kind": "tasks", "body": "x", "message": "again"}),
  )
  .await;
  let err = error_of(conflict.unwrap_err());
  assert_eq!(kind(&err), "conflict");
  assert_eq!(err.data.as_ref().unwrap()["current_revision"], first);

  let second = call(
    &client,
    "put_document",
    json!({"project": "acme/demo", "number": 2, "kind": "tasks", "body": TASKS_BODY, "message": "same", "expected_revision": first}),
  )
  .await
  .unwrap();
  let second_id = second["revision_id"].as_i64().unwrap();
  assert!(second_id > first);

  let marked = call(
    &client,
    "mark_task_done",
    json!({"project": "acme/demo", "number": 2, "task_id": "T01", "commit": "abc1234"}),
  )
  .await
  .unwrap();
  assert!(marked["revision_id"].as_i64().unwrap() > second_id);
  assert!(
    marked["line"].as_str().unwrap().starts_with("- [x] T01 "),
    "{}",
    marked["line"]
  );
  assert!(
    marked["line"]
      .as_str()
      .unwrap()
      .ends_with("· commit: abc1234")
  );
  let again = call(
    &client,
    "mark_task_done",
    json!({"project": "acme/demo", "number": 2, "task_id": "T01", "commit": "abc1234"}),
  )
  .await;
  assert_eq!(kind(&error_of(again.unwrap_err())), "validation");
  let doc = call(
    &client,
    "get_document",
    json!({"project": "acme/demo", "number": 2, "kind": "tasks"}),
  )
  .await
  .unwrap();
  assert!(doc["body"].as_str().unwrap().contains("- [x] T01"));

  let bad = call(
    &client,
    "set_spec_status",
    json!({"project": "acme/demo", "number": 2, "status": "approved"}),
  )
  .await;
  let err = error_of(bad.unwrap_err());
  assert_eq!(kind(&err), "validation");
  assert!(err.message.contains("clarified"), "{}", err.message);
  let status = call(
    &client,
    "set_spec_status",
    json!({"project": "acme/demo", "number": 2, "status": "clarified"}),
  )
  .await
  .unwrap();
  assert_eq!(status, json!({"number": 2, "status": "clarified"}));
  let archive = call(
    &client,
    "set_spec_status",
    json!({"project": "acme/demo", "number": 2, "status": "archived"}),
  )
  .await;
  assert_eq!(kind(&error_of(archive.unwrap_err())), "validation");

  let not_owner = call(
    &client,
    "put_constitution",
    json!({"project": "acme/demo", "body": "c", "version": "2.0", "message": "m"}),
  )
  .await;
  assert_eq!(kind(&error_of(not_owner.unwrap_err())), "forbidden");
  client.cancel().await.unwrap();

  let owner = mcp.connect("root").await;
  let current = call(&owner, "get_constitution", json!({"project": "acme/demo"}))
    .await
    .unwrap();
  let current_id = current["revision"]["id"].as_i64().unwrap();
  let same_version = call(
    &owner,
    "put_constitution",
    json!({"project": "acme/demo", "body": "c", "version": "1.0", "message": "m", "expected_revision": current_id}),
  )
  .await;
  assert_eq!(kind(&error_of(same_version.unwrap_err())), "conflict");
  let amended = call(
    &owner,
    "put_constitution",
    json!({"project": "acme/demo", "body": "# Constitution v1.1\n", "version": "1.1", "message": "amended", "expected_revision": current_id}),
  )
  .await
  .unwrap();
  assert!(amended["revision_id"].as_i64().unwrap() > current_id);
  let after = call(&owner, "get_constitution", json!({"project": "acme/demo"}))
    .await
    .unwrap();
  assert_eq!(after["version"], "1.1");
  owner.cancel().await.unwrap();

  let reader = mcp.connect("reader").await;
  let forbidden = call(
    &reader,
    "put_document",
    json!({"project": "acme/demo", "number": 2, "kind": "plan", "body": "p", "message": "m"}),
  )
  .await;
  assert_eq!(kind(&error_of(forbidden.unwrap_err())), "forbidden");
  reader.cancel().await.unwrap();
}

#[tokio::test]
async fn derived_reads_tasks_markers_and_status() {
  let mcp = seed().await;
  let client = mcp.connect("editor").await;

  let tools = client.list_tools(None).await.unwrap();
  let names: Vec<&str> = tools.tools.iter().map(|t| t.name.as_ref()).collect();
  for expected in ["list_tasks", "list_markers", "project_status"] {
    assert!(
      names.contains(&expected),
      "missing tool {expected} in {names:?}"
    );
  }

  call(
    &client,
    "create_spec",
    json!({"project": "acme/demo", "slug": "second"}),
  )
  .await
  .unwrap();
  call(
    &client,
    "put_document",
    json!({"project": "acme/demo", "number": 2, "kind": "tasks", "body": TASKS_BODY, "message": "tasks"}),
  )
  .await
  .unwrap();
  call(
    &client,
    "put_document",
    json!({"project": "acme/demo", "number": 2, "kind": "spec", "body": "# Spec 002\n\n## 12. Open questions\n- [NEEDS CLARIFICATION: who approves?]\n", "message": "spec"}),
  )
  .await
  .unwrap();
  call(
    &client,
    "mark_task_done",
    json!({"project": "acme/demo", "number": 2, "task_id": "T01", "commit": "abc1234"}),
  )
  .await
  .unwrap();

  let tasks = call(
    &client,
    "list_tasks",
    json!({"project": "acme/demo", "number": 2}),
  )
  .await
  .unwrap();
  assert_eq!(tasks["malformed"], json!([]));
  let list = tasks["tasks"].as_array().unwrap();
  assert_eq!(list.len(), 2);
  assert_eq!(
    list[0],
    json!({
      "id": "T01", "phase": "Phase 1 — Setup", "parallel": false, "frs": [],
      "description": "Create the crate", "done_when": "`cargo build` passes",
      "done": true, "commit": "abc1234", "line": 4
    })
  );
  assert_eq!(list[1]["id"], "T02");
  assert_eq!(list[1]["frs"], json!(["FR-1"]));
  assert_eq!(list[1]["done"], false);
  assert!(list[1]["commit"].is_null());

  let pending = call(
    &client,
    "list_tasks",
    json!({"project": "acme/demo", "number": 2, "pending_only": true}),
  )
  .await
  .unwrap();
  assert_eq!(pending["tasks"].as_array().unwrap().len(), 1);
  assert_eq!(pending["tasks"][0]["id"], "T02");

  let markers = call(
    &client,
    "list_markers",
    json!({"project": "acme/demo", "number": 2}),
  )
  .await
  .unwrap();
  assert_eq!(
    markers,
    json!({"markers": [{
      "document_kind": "spec", "kind": "clarification",
      "heading": "12. Open questions", "text": "who approves?", "line": 4
    }]})
  );

  let status = call(&client, "project_status", json!({"project": "acme/demo"}))
    .await
    .unwrap();
  assert_eq!(status["constitution_version"], "1.0");
  assert_eq!(status["specs"].as_array().unwrap().len(), 2);
  assert_eq!(status["specs"][1]["kinds"], json!(["spec", "tasks"]));
  let active = &status["active"];
  assert_eq!(active["number"], 2);
  assert_eq!(active["slug"], "second");
  assert_eq!(active["status"], "draft");
  assert_eq!(active["branch"], "002-second");
  assert_eq!(active["tasks_done"], 1);
  assert_eq!(active["tasks_total"], 2);
  assert_eq!(active["current_phase"], "Phase 1 — Setup");
  assert_eq!(active["next_task"]["id"], "T02");
  assert_eq!(active["next_task"]["done_when"], "tests green");
  assert_eq!(active["markers"], 1);
  client.cancel().await.unwrap();

  let admin = mcp.connect("root").await;
  let empty = call(&admin, "project_status", json!({"project": "acme/other"}))
    .await
    .unwrap();
  assert!(empty["constitution_version"].is_null());
  assert_eq!(empty["specs"], json!([]));
  assert!(empty["active"].is_null());
  admin.cancel().await.unwrap();
}

#[tokio::test]
async fn history_tools_list_and_read_revisions() {
  let mcp = seed().await;
  let client = mcp.connect("editor").await;

  let tools = client.list_tools(None).await.unwrap();
  let names: Vec<&str> = tools.tools.iter().map(|t| t.name.as_ref()).collect();
  for expected in ["list_revisions", "get_revision"] {
    assert!(
      names.contains(&expected),
      "missing tool {expected} in {names:?}"
    );
  }
  assert_eq!(names.len(), 16, "{names:?}");

  let current = call(
    &client,
    "get_document",
    json!({"project": "acme/demo", "number": 1, "kind": "spec"}),
  )
  .await
  .unwrap();
  let first = current["revision"]["id"].as_i64().unwrap();
  let second = call(
    &client,
    "put_document",
    json!({"project": "acme/demo", "number": 1, "kind": "spec", "body": "# Spec 001 v2\n", "message": "second", "expected_revision": first}),
  )
  .await
  .unwrap()["revision_id"]
    .as_i64()
    .unwrap();

  let revisions = call(
    &client,
    "list_revisions",
    json!({"project": "acme/demo", "number": 1, "kind": "spec"}),
  )
  .await
  .unwrap();
  let list = revisions["revisions"].as_array().unwrap();
  assert_eq!(list.len(), 2);
  assert_eq!(list[0]["id"], second);
  assert_eq!(list[0]["message"], "second");
  assert_eq!(list[0]["author"], "editor");
  assert_eq!(list[1]["id"], first);
  assert!(list[0]["created_at"].as_str().unwrap().ends_with('Z'));

  let constitution = call(
    &client,
    "list_revisions",
    json!({"project": "acme/demo", "kind": "constitution"}),
  )
  .await
  .unwrap();
  assert_eq!(constitution["revisions"][0]["version"], "1.0");

  let old = call(
    &client,
    "get_revision",
    json!({"project": "acme/demo", "revision_id": first}),
  )
  .await
  .unwrap();
  assert_eq!(old["body"], "# Spec 001 — First\n");
  assert_eq!(old["kind"], "spec");
  assert_eq!(old["number"], 1);
  assert_eq!(old["revision"]["id"], first);
  assert_eq!(old["revision"]["message"], "initial");

  let missing = call(
    &client,
    "get_revision",
    json!({"project": "acme/demo", "revision_id": 9999}),
  )
  .await;
  assert_eq!(kind(&error_of(missing.unwrap_err())), "not_found");
  client.cancel().await.unwrap();
}
