mod common;

use axum::http::StatusCode;
use serde_json::json;

#[test]
fn no_documented_route_writes_specs_documents_constitutions_or_statuses() {
  let api = spec_store_api::router::openapi();
  let writes = ["post", "put", "patch", "delete"];
  let api = serde_json::to_value(&api).unwrap();
  for (path, item) in api["paths"].as_object().unwrap() {
    for method in writes {
      if item.get(method).is_none() {
        continue;
      }
      let touches_documents = [
        "/specs",
        "/documents",
        "/constitution",
        "/status",
        "/revisions",
        "/tasks",
      ]
      .iter()
      .any(|segment| path.contains(segment));
      assert!(
        !touches_documents,
        "{method} {path} would let the browser write documents"
      );
    }
  }
}

#[tokio::test]
async fn cookie_callers_cannot_import_or_export() {
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
  let cookie = app.browser_sign_in("alice").await;
  let import = app
    .browser(
      "POST",
      "/api/v1/projects/acme/demo/import",
      &cookie,
      true,
      Some(json!({"tree": {"constitution": null, "specs": [], "skipped": [], "problems": []}})),
    )
    .await;
  assert_eq!(import.status(), StatusCode::FORBIDDEN);
  let export = app
    .browser(
      "GET",
      "/api/v1/projects/acme/demo/export",
      &cookie,
      false,
      None,
    )
    .await;
  assert_eq!(export.status(), StatusCode::FORBIDDEN);
}
