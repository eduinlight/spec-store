mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use spec_store::model::{Caller, DocumentKind, Role};
use spec_store::server::{ServerConfig, build_router};
use spec_store::store::Store;
use tower::ServiceExt;

const TABLES: [&str; 8] = [
  "users",
  "api_tokens",
  "projects",
  "memberships",
  "specs",
  "documents",
  "revisions",
  "audit_log",
];

struct Fixture {
  store: Store,
  admin: Caller,
  revision: i64,
}

fn fixture() -> Fixture {
  let store = Store::open(common::temp_db_path("ui-readonly")).unwrap();
  store.create_user(None, "root", false).unwrap();
  let admin = store.caller_by_handle("root").unwrap();
  store.create_user(Some(&admin), "carol", false).unwrap();
  store
    .create_project(&admin, "demo", "Demo project")
    .unwrap();
  store
    .set_member(&admin, "demo", "carol", Role::Owner)
    .unwrap();
  let carol = store.caller_by_handle("carol").unwrap();
  store
    .put_constitution(&carol, "demo", "# C\n", "1.0", "m", None)
    .unwrap();
  store.create_spec(&carol, "demo", "first").unwrap();
  let revision = store
    .put_document(
      &carol,
      "demo",
      1,
      DocumentKind::Tasks,
      "# T\n\n## Phase 1\n- [ ] T01 (FR-1) x — Done when: y\n",
      "m",
      None,
    )
    .unwrap();
  Fixture {
    store,
    admin,
    revision,
  }
}

fn counts(store: &Store) -> Vec<(String, i64)> {
  let mut out = Vec::new();
  for table in TABLES {
    let n = common::int(store, &format!("SELECT count(*) FROM {table}"));
    out.push((table.to_string(), n));
  }
  let bodies = common::int(store, "SELECT sum(length(body)) FROM revisions");
  out.push(("revision bytes".to_string(), bodies));
  let roles = common::text(
    store,
    "SELECT group_concat(role, ',') FROM (SELECT role FROM memberships ORDER BY user_id)",
  );
  out.push(("roles".to_string(), roles.len() as i64));
  out
}

fn session(f: &Fixture, handle: &str) -> String {
  let token = f
    .store
    .issue_token(Some(&f.admin), handle, "browser")
    .unwrap()
    .token;
  f.store.create_session(&token).unwrap()
}

fn paths(f: &Fixture) -> Vec<String> {
  vec![
    "/ui/".to_string(),
    "/ui/profile".to_string(),
    "/ui/audit".to_string(),
    "/ui/p/demo".to_string(),
    "/ui/p/demo/audit".to_string(),
    "/ui/p/demo/constitution".to_string(),
    "/ui/p/demo/constitution?view=raw".to_string(),
    "/ui/p/demo/constitution/history".to_string(),
    "/ui/p/demo/specs/1".to_string(),
    "/ui/p/demo/specs/1/tasks".to_string(),
    "/ui/p/demo/specs/1/tasks?view=raw".to_string(),
    "/ui/p/demo/specs/1/tasks/history".to_string(),
    format!("/ui/p/demo/r/{}", f.revision),
    format!("/ui/p/demo/r/{}/diff", f.revision),
  ]
}

async fn send(f: &Fixture, method: &str, path: &str, cookie: &str) -> StatusCode {
  let app = build_router(f.store.clone(), ServerConfig::default());
  let request = Request::builder()
    .method(method)
    .uri(path)
    .header(header::HOST, "localhost")
    .header(header::COOKIE, format!("spec_store_session={cookie}"))
    .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
    .body(Body::from("body=anything&status=archived"))
    .unwrap();
  let response = app.oneshot(request).await.unwrap();
  let status = response.status();
  let _ = response.into_body().collect().await.unwrap();
  status
}

#[tokio::test]
async fn ui_paths_reject_writing_methods() {
  let f = fixture();
  let cookie = session(&f, "carol");
  let before = counts(&f.store);
  for path in paths(&f) {
    for method in ["POST", "PUT", "DELETE", "PATCH"] {
      let status = send(&f, method, &path, &cookie).await;
      assert!(
        status == StatusCode::METHOD_NOT_ALLOWED || status == StatusCode::NOT_FOUND,
        "{method} {path} answered {status}"
      );
    }
  }
  for method in ["PUT", "DELETE", "PATCH"] {
    for path in ["/ui/login", "/ui/logout"] {
      let status = send(&f, method, path, &cookie).await;
      assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED, "{method} {path}");
    }
  }
  assert_eq!(counts(&f.store), before);
}

#[tokio::test]
async fn browsing_every_page_changes_nothing_but_the_session() {
  let f = fixture();
  let cookie = session(&f, "root");
  let before = counts(&f.store);
  for path in paths(&f) {
    let status = send(&f, "GET", &path, &cookie).await;
    assert_eq!(status, StatusCode::OK, "{path}");
  }
  let status = send(&f, "GET", "/ui/p/demo/specs/9", &cookie).await;
  assert_eq!(status, StatusCode::NOT_FOUND);
  assert_eq!(counts(&f.store), before);
  let sessions: i64 = common::int(&f.store, "SELECT count(*) FROM sessions");
  assert_eq!(sessions, 1);
}
