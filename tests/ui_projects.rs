mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use spec_store::model::{Caller, DocumentKind, Role};
use spec_store::server::{ServerConfig, build_router};
use spec_store::store::Store;
use tower::ServiceExt;

const TASKS: &str = "# Tasks — Spec 001

## Phase 1 — Setup
- [ ] T01 (FR: —) Create the crate — Done when: `cargo build` passes

## Phase 2 — US-1
- [ ] T02 [P] (FR-1, FR-2) Write tests — Done when: tests written and red
- [ ] T03 (FR-1, FR-2) Implement — Done when: T02 green
";

struct Fixture {
  store: Store,
  admin: Caller,
}

fn fixture() -> Fixture {
  let store = Store::open(common::temp_db_path("ui-projects")).unwrap();
  store.create_user(None, "root", false).unwrap();
  let admin = store.caller_by_handle("root").unwrap();
  for h in ["bob", "carol", "dave", "eve"] {
    store.create_user(Some(&admin), h, false).unwrap();
  }
  store
    .create_project(&admin, "demo", "Demo project")
    .unwrap();
  store
    .create_project(&admin, "secret", "Secret project")
    .unwrap();
  store
    .set_member(&admin, "demo", "carol", Role::Owner)
    .unwrap();
  store
    .set_member(&admin, "demo", "bob", Role::Editor)
    .unwrap();
  store
    .set_member(&admin, "demo", "dave", Role::Reader)
    .unwrap();
  let bob = store.caller_by_handle("bob").unwrap();
  let carol = store.caller_by_handle("carol").unwrap();
  store
    .put_constitution(&carol, "demo", "# Constitution\n", "1.0", "ratified", None)
    .unwrap();
  store.create_spec(&bob, "demo", "first").unwrap();
  store
    .put_document(&bob, "demo", 1, DocumentKind::Tasks, TASKS, "tasks", None)
    .unwrap();
  store
    .mark_task_done(&bob, "demo", 1, "T01", "abc1234")
    .unwrap();
  store.create_spec(&bob, "demo", "second").unwrap();
  store
    .put_document(
      &bob,
      "demo",
      2,
      DocumentKind::Spec,
      "# Spec 002 — Second\n\n## Open\n\n[NEEDS CLARIFICATION: scope?]\n",
      "draft",
      None,
    )
    .unwrap();
  Fixture { store, admin }
}

fn session(f: &Fixture, handle: &str) -> String {
  let token = f
    .store
    .issue_token(Some(&f.admin), handle, "browser")
    .unwrap()
    .token;
  f.store.create_session(&token).unwrap()
}

async fn page(f: &Fixture, path: &str, cookie: &str) -> (StatusCode, String) {
  let app = build_router(f.store.clone(), ServerConfig::default());
  let request = Request::builder()
    .uri(path)
    .header(header::HOST, "localhost")
    .header(header::COOKIE, format!("spec_store_session={cookie}"))
    .body(Body::empty())
    .unwrap();
  let response = app.oneshot(request).await.unwrap();
  let status = response.status();
  let bytes = response.into_body().collect().await.unwrap().to_bytes();
  (status, String::from_utf8_lossy(&bytes).into_owned())
}

fn row_containing<'a>(html: &'a str, needle: &str) -> &'a str {
  html
    .split("<tr")
    .find(|row| row.contains(needle))
    .unwrap_or_else(|| panic!("no row containing {needle:?} in\n{html}"))
}

#[tokio::test]
async fn projects_page_lists_memberships_with_progress() {
  let f = fixture();
  let bob = session(&f, "bob");
  let (status, html) = page(&f, "/ui/", &bob).await;
  assert_eq!(status, StatusCode::OK);
  assert!(html.contains("Demo project"));
  assert!(!html.contains("Secret project"));
  assert!(html.contains("href=\"/ui/p/demo\""));
  let row = row_containing(&html, "Demo project");
  assert!(row.contains("editor"), "{row}");
  assert!(row.contains("1.0"), "constitution version: {row}");
  assert!(row.contains("<td>2</td>"), "spec count: {row}");
  assert!(row.contains("002"), "active spec: {row}");
  assert!(row.contains("second"), "{row}");
  assert!(row.contains("0/0"), "tasks of the active spec: {row}");
  assert!(row.contains("1 marker"), "{row}");
  assert!(!html.contains("no membership"));

  let admin = session(&f, "root");
  let (_, html) = page(&f, "/ui/", &admin).await;
  assert!(html.contains("Demo project"));
  assert!(html.contains("Secret project"));
  assert_eq!(html.matches("no membership").count(), 2);
  let secret = row_containing(&html, "Secret project");
  assert!(secret.contains("no constitution"), "{secret}");
  assert!(secret.contains("no active spec"), "{secret}");
  assert!(secret.contains("<td>0</td>"), "{secret}");

  let eve = session(&f, "eve");
  let (status, html) = page(&f, "/ui/", &eve).await;
  assert_eq!(status, StatusCode::OK);
  assert!(html.contains("not a member of any project"));
  assert!(!html.contains("<table"));
}

#[tokio::test]
async fn project_page_shows_specs_constitution_repository_and_members() {
  let f = fixture();
  let bob = session(&f, "bob");
  let (status, html) = page(&f, "/ui/p/demo", &bob).await;
  assert_eq!(status, StatusCode::OK);
  assert!(html.contains("<h1>Demo project</h1>"));
  assert!(html.contains("demo"));
  assert!(html.contains("No repository URL"));
  assert!(html.contains("href=\"/ui/p/demo/constitution\""));
  assert!(html.contains("1.0"));
  assert!(html.contains("href=\"/ui/p/demo/specs/1\""));
  assert!(html.contains("href=\"/ui/p/demo/specs/2\""));
  let first = row_containing(&html, "/ui/p/demo/specs/1\"");
  assert!(first.contains("001"), "{first}");
  assert!(first.contains("first"), "{first}");
  assert!(first.contains("001-first"), "branch: {first}");
  assert!(first.contains("tasks"), "kinds: {first}");
  assert!(first.contains("1/3"), "tasks done over total: {first}");
  assert!(first.contains("badge-draft"), "{first}");
  assert!(!first.contains("active"), "{first}");
  let second = row_containing(&html, "/ui/p/demo/specs/2\"");
  assert!(second.contains("active"), "{second}");
  assert!(second.contains("spec"), "kinds: {second}");
  assert!(second.contains("0/0"), "{second}");
  assert!(second.contains("1 marker"), "{second}");

  let members_at = html.find("Members").expect("members section");
  let members = &html[members_at..];
  let carol = members.find("carol").unwrap();
  let bob_at = members.find("bob").unwrap();
  let dave = members.find("dave").unwrap();
  assert!(
    carol < bob_at && bob_at < dave,
    "owners first, then editors, then readers"
  );
  assert!(members.contains("badge-owner"));
  assert!(!html.contains("read-only"));
  assert!(
    !html.contains("href=\"/ui/p/demo/audit\""),
    "editors get no audit link"
  );

  f.store
    .set_repository_url(&f.admin, "demo", Some("https://github.com/acme/demo"))
    .unwrap();
  let (_, html) = page(&f, "/ui/p/demo", &bob).await;
  assert!(html.contains("href=\"https://github.com/acme/demo\""));
  assert!(!html.contains("No repository URL"));

  let carol = session(&f, "carol");
  let (_, html) = page(&f, "/ui/p/demo", &carol).await;
  assert!(
    html.contains("href=\"/ui/p/demo/audit\""),
    "owners get the audit link"
  );
}

#[tokio::test]
async fn empty_project_and_admin_view() {
  let f = fixture();
  let admin = session(&f, "root");
  let (status, html) = page(&f, "/ui/p/secret", &admin).await;
  assert_eq!(status, StatusCode::OK);
  assert!(html.contains("<h1>Secret project</h1>"));
  assert!(html.contains("No constitution yet"));
  assert!(html.contains("No specs yet"));
  assert!(html.contains("No members"));
  assert!(html.contains("read-only"));
  assert!(html.contains("href=\"/ui/p/secret/audit\""));
}

#[tokio::test]
async fn outsiders_get_the_same_not_found_page_for_existing_and_missing_projects() {
  let f = fixture();
  let eve = session(&f, "eve");
  let (existing_status, existing) = page(&f, "/ui/p/demo", &eve).await;
  let (missing_status, missing) = page(&f, "/ui/p/nope", &eve).await;
  assert_eq!(existing_status, StatusCode::NOT_FOUND);
  assert_eq!(missing_status, StatusCode::NOT_FOUND);
  assert_eq!(existing, missing);
  assert!(!existing.contains("Demo project"));

  let (status, _) = page(&f, "/ui/p/demo/specs/1", &eve).await;
  assert_eq!(status, StatusCode::NOT_FOUND);
}
