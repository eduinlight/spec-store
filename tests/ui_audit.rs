mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use spec_store::model::{Caller, Role};
use spec_store::server::{ServerConfig, build_router};
use spec_store::store::Store;
use tower::ServiceExt;

struct Fixture {
  store: Store,
  admin: Caller,
}

fn fixture() -> Fixture {
  let store = Store::open(common::temp_db_path("ui-audit")).unwrap();
  store.create_user(None, "root", false).unwrap();
  let admin = store.caller_by_handle("root").unwrap();
  for h in ["owner", "editor"] {
    store.create_user(Some(&admin), h, false).unwrap();
  }
  store
    .create_project(&admin, "demo", "Demo project")
    .unwrap();
  store
    .create_project(&admin, "other", "Other project")
    .unwrap();
  store
    .set_member(&admin, "demo", "owner", Role::Owner)
    .unwrap();
  store
    .set_member(&admin, "demo", "editor", Role::Editor)
    .unwrap();
  let editor = store.caller_by_handle("editor").unwrap();
  for i in 0..60 {
    store
      .create_spec(&editor, "demo", &format!("spec-{i}"))
      .unwrap();
  }
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

fn rows(html: &str) -> usize {
  html.matches("<tr class=\"entry\"").count()
}

#[tokio::test]
async fn owners_see_their_project_audit_in_pages() {
  let f = fixture();
  let owner = session(&f, "owner");
  let (status, html) = page(&f, "/ui/p/demo/audit", &owner).await;
  assert_eq!(status, StatusCode::OK);
  assert_eq!(rows(&html), 50);
  assert!(html.contains("spec.create"));
  assert!(html.contains("060-spec-59"));
  assert!(
    !html.contains("001-spec-0"),
    "oldest entries are on the next page"
  );
  assert!(html.contains("editor"));
  let newest = html.find("060-spec-59").unwrap();
  let older = html.find("011-spec-10").unwrap();
  assert!(newest < older, "newest first");
  let older_link = html
    .split("href=\"")
    .map(|s| s.split('"').next().unwrap_or_default())
    .find(|href| href.starts_with("/ui/p/demo/audit?before="))
    .expect("link to older entries")
    .to_string();
  assert!(html.contains("Older"));
  assert!(!html.contains("Newer"), "first page has no newer link");

  let (status, html) = page(&f, &older_link, &owner).await;
  assert_eq!(status, StatusCode::OK);
  assert!(rows(&html) >= 10 && rows(&html) < 50, "{}", rows(&html));
  assert!(html.contains("001-spec-0"));
  assert!(html.contains("project.create"));
  assert!(html.contains("member.set"));
  assert!(html.contains("Newer"));
  assert!(html.contains("href=\"/ui/p/demo/audit\""));
  assert!(!html.contains("?before="), "last page has no older link");
  assert!(
    !html.contains("user.create"),
    "entries without project stay out of a project page"
  );

  let (status, _) = page(&f, "/ui/p/other/audit", &owner).await;
  assert_eq!(
    status,
    StatusCode::NOT_FOUND,
    "no membership means not found"
  );
}

#[tokio::test]
async fn editors_are_not_allowed_and_see_no_link() {
  let f = fixture();
  let editor = session(&f, "editor");
  let (status, html) = page(&f, "/ui/p/demo/audit", &editor).await;
  assert_eq!(status, StatusCode::FORBIDDEN);
  assert!(html.contains("Not allowed"));
  assert!(!html.contains("spec.create"));
  let (status, _) = page(&f, "/ui/audit", &editor).await;
  assert_eq!(status, StatusCode::FORBIDDEN);
  let (_, project_page) = page(&f, "/ui/p/demo", &editor).await;
  assert!(!project_page.contains("/ui/p/demo/audit"));
  let (_, projects) = page(&f, "/ui/", &editor).await;
  assert!(!projects.contains("href=\"/ui/audit\""));

  let owner = session(&f, "owner");
  let (status, _) = page(&f, "/ui/audit", &owner).await;
  assert_eq!(status, StatusCode::FORBIDDEN, "global audit is for admins");
}

#[tokio::test]
async fn admins_see_everything_with_a_project_filter() {
  let f = fixture();
  let admin = session(&f, "root");
  let (status, html) = page(&f, "/ui/audit", &admin).await;
  assert_eq!(status, StatusCode::OK);
  assert_eq!(rows(&html), 50);
  assert!(html.contains("name=\"project\""), "filter form");
  assert!(html.contains("Older"));

  let (status, html) = page(&f, "/ui/audit?project=other", &admin).await;
  assert_eq!(status, StatusCode::OK);
  assert!(html.contains("project.create"));
  assert!(html.contains("Other project") || html.contains("other"));
  assert!(!html.contains("spec.create"));
  assert!(html.contains("value=\"other\""));

  let (status, html) = page(&f, "/ui/audit?project=", &admin).await;
  assert_eq!(status, StatusCode::OK);
  assert_eq!(rows(&html), 50);

  let (status, _) = page(&f, "/ui/audit?project=missing", &admin).await;
  assert_eq!(status, StatusCode::NOT_FOUND);

  let (status, html) = page(&f, "/ui/p/other/audit", &admin).await;
  assert_eq!(status, StatusCode::OK, "admins read any project's audit");
  assert!(html.contains("project.create"));

  let last = html.find("user.create");
  assert!(
    last.is_none(),
    "project page excludes entries without project"
  );
  let (_, global) = page(&f, "/ui/audit?before=4", &admin).await;
  assert!(
    global.contains("user.create"),
    "global page includes entries without project"
  );
}
