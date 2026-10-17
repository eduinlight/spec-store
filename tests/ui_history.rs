mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use spec_store::model::{Caller, DocumentKind, Role};
use spec_store::server::{ServerConfig, build_router};
use spec_store::store::Store;
use tower::ServiceExt;

const FIRST: &str = "# Spec 001 — First\n\n## Goal\n\nShip it.\n\n## Scope\n\nSmall.\n";
const SECOND: &str = "# Spec 001 — First\n\n## Goal\n\nShip it well.\n\n## Scope\n\nSmall.\n";

struct Fixture {
  store: Store,
  admin: Caller,
  first: i64,
  second: i64,
  constitution_first: i64,
  constitution_second: i64,
}

fn fixture() -> Fixture {
  let store = Store::open(common::temp_db_path("ui-history")).unwrap();
  store.create_user(None, "root", false).unwrap();
  let admin = store.caller_by_handle("root").unwrap();
  for h in ["bob", "carol", "eve"] {
    store.create_user(Some(&admin), h, false).unwrap();
  }
  store
    .create_project(&admin, "demo", "Demo project")
    .unwrap();
  store
    .set_member(&admin, "demo", "carol", Role::Owner)
    .unwrap();
  store
    .set_member(&admin, "demo", "bob", Role::Editor)
    .unwrap();
  let bob = store.caller_by_handle("bob").unwrap();
  let carol = store.caller_by_handle("carol").unwrap();
  let constitution_first = store
    .put_constitution(
      &carol,
      "demo",
      "# Constitution\n\n1. One\n",
      "1.0",
      "ratified",
      None,
    )
    .unwrap();
  let constitution_second = store
    .put_constitution(
      &carol,
      "demo",
      "# Constitution\n\n1. One\n2. Two\n",
      "1.1",
      "amended",
      Some(constitution_first),
    )
    .unwrap();
  store.create_spec(&bob, "demo", "first").unwrap();
  let first = store
    .put_document(&bob, "demo", 1, DocumentKind::Spec, FIRST, "initial", None)
    .unwrap();
  let second = store
    .put_document(
      &carol,
      "demo",
      1,
      DocumentKind::Spec,
      SECOND,
      "clarified goal",
      Some(first),
    )
    .unwrap();
  Fixture {
    store,
    admin,
    first,
    second,
    constitution_first,
    constitution_second,
  }
}

fn session(f: &Fixture, handle: &str) -> String {
  let token = f
    .store
    .issue_token(Some(&f.admin), handle, "browser")
    .unwrap()
    .token;
  f.store.create_session(&token).unwrap()
}

async fn fetch(f: &Fixture, path: &str, cookie: &str) -> (StatusCode, Vec<u8>) {
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
  (status, bytes.to_vec())
}

async fn page(f: &Fixture, path: &str, cookie: &str) -> (StatusCode, String) {
  let (status, bytes) = fetch(f, path, cookie).await;
  (status, String::from_utf8_lossy(&bytes).into_owned())
}

fn row_containing<'a>(html: &'a str, needle: &str) -> &'a str {
  html
    .split("<tr")
    .find(|row| row.contains(needle))
    .unwrap_or_else(|| panic!("no row containing {needle:?} in\n{html}"))
}

#[tokio::test]
async fn history_lists_revisions_newest_first_and_marks_the_current_one() {
  let f = fixture();
  let bob = session(&f, "bob");
  let (status, html) = page(&f, "/ui/p/demo/specs/1/spec/history", &bob).await;
  assert_eq!(status, StatusCode::OK);
  let second_at = html
    .find(&format!("/ui/p/demo/r/{}\"", f.second))
    .expect("second");
  let first_at = html
    .find(&format!("/ui/p/demo/r/{}\"", f.first))
    .expect("first");
  assert!(second_at < first_at, "newest first");
  let second_row = row_containing(&html, &format!("/ui/p/demo/r/{}\"", f.second));
  assert!(second_row.contains("carol"));
  assert!(second_row.contains("clarified goal"));
  assert!(second_row.contains("current"), "{second_row}");
  assert!(second_row.contains(&format!("/ui/p/demo/r/{}/diff", f.second)));
  let first_row = row_containing(&html, &format!("/ui/p/demo/r/{}\"", f.first));
  assert!(first_row.contains("bob"));
  assert!(first_row.contains("initial"));
  assert!(!first_row.contains("current"), "{first_row}");
  assert_eq!(html.matches("badge\">current").count(), 1);
  assert!(html.contains("href=\"/ui/p/demo/specs/1/spec\""));
  assert!(html.contains("href=\"/ui/p/demo/specs/1\""));

  let (status, html) = page(&f, "/ui/p/demo/constitution/history", &bob).await;
  assert_eq!(status, StatusCode::OK);
  let newest = row_containing(&html, &format!("/ui/p/demo/r/{}\"", f.constitution_second));
  assert!(newest.contains("1.1"), "{newest}");
  assert!(newest.contains("amended"));
  let oldest = row_containing(&html, &format!("/ui/p/demo/r/{}\"", f.constitution_first));
  assert!(oldest.contains("1.0"), "{oldest}");
  assert!(html.contains("href=\"/ui/p/demo/constitution\""));

  let (status, _) = page(&f, "/ui/p/demo/specs/1/plan/history", &bob).await;
  assert_eq!(status, StatusCode::NOT_FOUND);
  let (status, _) = page(&f, "/ui/p/demo/specs/7/spec/history", &bob).await;
  assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn revision_pages_show_old_bodies_and_point_to_the_current_one() {
  let f = fixture();
  let bob = session(&f, "bob");
  let (status, html) = page(&f, &format!("/ui/p/demo/r/{}", f.first), &bob).await;
  assert_eq!(status, StatusCode::OK);
  assert!(html.contains("<p>Ship it.</p>"));
  assert!(html.contains("not the current revision"));
  assert!(html.contains("href=\"/ui/p/demo/specs/1/spec\""));
  assert!(html.contains(&format!("href=\"/ui/p/demo/r/{}?view=raw\"", f.first)));
  assert!(html.contains(&format!("href=\"/ui/p/demo/r/{}/diff\"", f.first)));
  assert!(html.contains("href=\"/ui/p/demo/specs/1/spec/history\""));
  assert!(html.contains("bob"));
  assert!(html.contains("initial"));

  let (status, html) = page(&f, &format!("/ui/p/demo/r/{}", f.second), &bob).await;
  assert_eq!(status, StatusCode::OK);
  assert!(html.contains("<p>Ship it well.</p>"));
  assert!(!html.contains("not the current revision"));

  let (_, bytes) = fetch(&f, &format!("/ui/p/demo/r/{}?view=raw", f.first), &bob).await;
  assert_eq!(bytes, FIRST.as_bytes());

  let (status, html) = page(&f, &format!("/ui/p/demo/r/{}", f.constitution_first), &bob).await;
  assert_eq!(status, StatusCode::OK);
  assert!(html.contains("1.0"));
  assert!(html.contains("not the current revision"));
  assert!(html.contains("href=\"/ui/p/demo/constitution\""));
  assert!(html.contains("href=\"/ui/p/demo/constitution/history\""));

  let (status, _) = page(&f, "/ui/p/demo/r/9999", &bob).await;
  assert_eq!(status, StatusCode::NOT_FOUND);
  let (status, _) = page(&f, "/ui/p/demo/r/abc", &bob).await;
  assert_eq!(status, StatusCode::NOT_FOUND);
  let eve = session(&f, "eve");
  let (status, _) = page(&f, &format!("/ui/p/demo/r/{}", f.first), &eve).await;
  assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn diff_pages_compare_with_the_predecessor() {
  let f = fixture();
  let bob = session(&f, "bob");
  let (status, html) = page(&f, &format!("/ui/p/demo/r/{}/diff", f.second), &bob).await;
  assert_eq!(status, StatusCode::OK);
  assert!(html.contains("class=\"diff\""));
  let inserted = row_containing(&html, "class=\"ins\"");
  assert!(inserted.contains("Ship it well."), "{inserted}");
  let deleted = row_containing(&html, "class=\"del\"");
  assert!(deleted.contains("Ship it."), "{deleted}");
  assert!(!deleted.contains("Ship it well."));
  assert!(html.contains("class=\"ctx\""), "context lines");
  assert!(
    html.contains(&format!("/ui/p/demo/r/{}\"", f.first)),
    "names the predecessor"
  );
  assert!(html.contains(&format!("href=\"/ui/p/demo/r/{}\"", f.second)));
  assert!(html.contains("href=\"/ui/p/demo/specs/1/spec/history\""));

  let (status, html) = page(&f, &format!("/ui/p/demo/r/{}/diff", f.first), &bob).await;
  assert_eq!(status, StatusCode::OK);
  assert!(html.contains("no predecessor"));
  assert!(!html.contains("class=\"ins\""));

  let (status, html) = page(
    &f,
    &format!("/ui/p/demo/r/{}/diff", f.constitution_second),
    &bob,
  )
  .await;
  assert_eq!(status, StatusCode::OK);
  assert!(row_containing(&html, "class=\"ins\"").contains("2. Two"));
  assert!(html.contains("1.1"));
  assert!(html.contains("1.0"));

  let (status, _) = page(&f, "/ui/p/demo/r/9999/diff", &bob).await;
  assert_eq!(status, StatusCode::NOT_FOUND);
}
