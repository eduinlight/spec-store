mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use spec_store::model::{Caller, DocumentKind, Role};
use spec_store::server::{ServerConfig, build_router};
use spec_store::store::Store;
use tower::ServiceExt;

const SPEC: &str = "# Spec 001 — First

## 1. Context

A <b>bold</b> idea with a table:

| a | b |
|---|---|
| 1 | 2 |

<script>alert(1)</script>

## 12. Open questions

- [NEEDS CLARIFICATION: scope?]
";

const TASKS: &str = "# Tasks — Spec 001

## Phase 1 — Setup
- [ ] T01 (FR: —) Create the crate — Done when: `cargo build` passes

## Phase 2 — US-1
- [ ] T02 [P] (FR-1, FR-2) Write tests — Done when: tests written and red
- [ ] T03 (FR-1, FR-2) Implement — Done when: T02 green
";

const CRLF: &str = "# Spec 002 — Second\r\n\r\nLine one\r\nLine two\r\n";

struct Fixture {
  store: Store,
  admin: Caller,
}

fn fixture() -> Fixture {
  let store = Store::open(common::temp_db_path("ui-specs")).unwrap();
  store.create_user(None, "root", false).unwrap();
  let admin = store.caller_by_handle("root").unwrap();
  for h in ["bob", "carol"] {
    store.create_user(Some(&admin), h, false).unwrap();
  }
  store
    .create_project(&admin, "demo", "Demo project")
    .unwrap();
  store
    .create_project(&admin, "bare", "Bare project")
    .unwrap();
  store
    .set_member(&admin, "demo", "carol", Role::Owner)
    .unwrap();
  store
    .set_member(&admin, "demo", "bob", Role::Editor)
    .unwrap();
  store
    .set_member(&admin, "bare", "bob", Role::Reader)
    .unwrap();
  let bob = store.caller_by_handle("bob").unwrap();
  let carol = store.caller_by_handle("carol").unwrap();
  store
    .put_constitution(
      &carol,
      "demo",
      "# Constitution\n\n1. Rule\n",
      "1.0",
      "ratified",
      None,
    )
    .unwrap();
  store.create_spec(&bob, "demo", "first").unwrap();
  store
    .put_document(&bob, "demo", 1, DocumentKind::Spec, SPEC, "draft", None)
    .unwrap();
  store
    .put_document(&bob, "demo", 1, DocumentKind::Tasks, TASKS, "tasks", None)
    .unwrap();
  let marked = store
    .mark_task_done(&bob, "demo", 1, "T01", "abc1234")
    .unwrap();
  let current = store
    .get_document(&bob, "demo", 1, DocumentKind::Tasks)
    .unwrap();
  store
    .put_document(
      &bob,
      "demo",
      1,
      DocumentKind::Tasks,
      &format!("{}- [ ] T04 broken line\n", current.body),
      "add a broken line",
      Some(marked.revision_id),
    )
    .unwrap();
  store.create_spec(&bob, "demo", "second").unwrap();
  store
    .put_document(&bob, "demo", 2, DocumentKind::Spec, CRLF, "crlf", None)
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

async fn fetch(f: &Fixture, path: &str, cookie: &str) -> (StatusCode, String, Vec<u8>) {
  let app = build_router(f.store.clone(), ServerConfig::default());
  let request = Request::builder()
    .uri(path)
    .header(header::HOST, "localhost")
    .header(header::COOKIE, format!("spec_store_session={cookie}"))
    .body(Body::empty())
    .unwrap();
  let response = app.oneshot(request).await.unwrap();
  let status = response.status();
  let content_type = response
    .headers()
    .get(header::CONTENT_TYPE)
    .map(|v| v.to_str().unwrap().to_string())
    .unwrap_or_default();
  let bytes = response.into_body().collect().await.unwrap().to_bytes();
  (status, content_type, bytes.to_vec())
}

async fn page(f: &Fixture, path: &str, cookie: &str) -> (StatusCode, String) {
  let (status, _, bytes) = fetch(f, path, cookie).await;
  (status, String::from_utf8_lossy(&bytes).into_owned())
}

fn row_containing<'a>(html: &'a str, needle: &str) -> &'a str {
  html
    .split("<tr")
    .find(|row| row.contains(needle))
    .unwrap_or_else(|| panic!("no row containing {needle:?} in\n{html}"))
}

#[tokio::test]
async fn spec_page_shows_documents_in_order_with_metadata() {
  let f = fixture();
  let bob = session(&f, "bob");
  let (status, html) = page(&f, "/ui/p/demo/specs/1", &bob).await;
  assert_eq!(status, StatusCode::OK);
  assert!(html.contains("001"));
  assert!(html.contains("first"));
  assert!(html.contains("001-first"));
  assert!(html.contains("badge-draft"));

  let positions: Vec<usize> = ["spec", "plan", "tasks", "research", "validation"]
    .iter()
    .map(|kind| {
      html
        .find(&format!("id=\"{kind}\""))
        .unwrap_or_else(|| panic!("no section for {kind}"))
    })
    .collect();
  assert!(positions.windows(2).all(|w| w[0] < w[1]), "{positions:?}");
  assert_eq!(html.matches("Not written yet").count(), 3);

  assert!(html.contains("bob"));
  assert!(html.contains("draft"), "revision message");
  assert!(html.contains("<table>"), "rendered table");
  assert!(html.contains("<td>2</td>"));
  assert!(!html.contains("<script>"));
  assert!(html.contains("&lt;script&gt;alert(1)&lt;/script&gt;"));
  assert!(html.contains("&lt;b&gt;bold&lt;/b&gt;"));

  assert!(html.contains("href=\"/ui/p/demo/specs/1/spec\""));
  assert!(html.contains("href=\"/ui/p/demo/specs/1/spec?view=raw\""));
  assert!(html.contains("href=\"/ui/p/demo/specs/1/spec/history\""));
  assert!(html.contains("href=\"/ui/p/demo/specs/1/tasks?view=raw\""));
  assert!(html.contains("href=\"/ui/p/demo/constitution\""));
  assert!(html.contains("href=\"/ui/p/demo\""));
}

#[tokio::test]
async fn spec_page_groups_tasks_by_phase_and_lists_markers() {
  let f = fixture();
  let bob = session(&f, "bob");
  let (_, html) = page(&f, "/ui/p/demo/specs/1", &bob).await;

  let table_at = html.find("id=\"task-table\"").expect("tasks section");
  let tasks = &html[table_at..];
  let phase1 = tasks.find("Phase 1 — Setup").expect("phase 1 heading");
  let phase2 = tasks.find("Phase 2 — US-1").expect("phase 2 heading");
  assert!(phase1 < phase2);
  let between = &tasks[phase1..phase2];
  assert!(between.contains("1/1"), "{between}");
  let after = &tasks[phase2..];
  assert!(after.contains("0/2"), "{after}");

  let t01 = row_containing(tasks, "T01");
  assert!(t01.contains("class=\"done\""), "{t01}");
  assert!(t01.contains("abc1234"));
  assert!(
    !t01.contains("href=\"https://"),
    "no repository, plain text"
  );
  assert!(t01.contains("`cargo build` passes") || t01.contains("<code>cargo build</code> passes"));
  let t02 = row_containing(tasks, "T02");
  assert!(t02.contains("FR-1, FR-2") || t02.contains("FR-1</li>") || t02.contains("FR-1 FR-2"));
  assert!(t02.contains("[P]") || t02.contains("parallel"));
  assert!(!t02.contains("class=\"done\""));
  assert!(html.contains("Malformed"));
  assert!(html.contains("T04 broken line"));
  assert!(html.contains("line 9") || html.contains(">9<"));

  assert!(html.contains("scope?"));
  assert!(html.contains("12. Open questions"));
  assert!(html.contains("clarification"));

  f.store
    .set_repository_url(&f.admin, "demo", Some("https://github.com/acme/demo/"))
    .unwrap();
  let (_, html) = page(&f, "/ui/p/demo/specs/1", &bob).await;
  assert!(
    html.contains("href=\"https://github.com/acme/demo/commit/abc1234\""),
    "single slash between repository and commit"
  );

  let (_, html) = page(&f, "/ui/p/demo/specs/2", &bob).await;
  assert!(html.contains("No tasks document"));
  assert!(html.contains("No markers"));
}

#[tokio::test]
async fn document_pages_render_and_raw_view_returns_exact_bytes() {
  let f = fixture();
  let bob = session(&f, "bob");
  let (status, html) = page(&f, "/ui/p/demo/specs/1/spec", &bob).await;
  assert_eq!(status, StatusCode::OK);
  assert!(html.contains("<table>"));
  assert!(html.contains("bob"));
  assert!(html.contains("href=\"/ui/p/demo/specs/1/spec?view=raw\""));
  assert!(html.contains("href=\"/ui/p/demo/specs/1/spec/history\""));
  assert!(html.contains("href=\"/ui/p/demo/specs/1\""));

  let (status, content_type, bytes) = fetch(&f, "/ui/p/demo/specs/1/spec?view=raw", &bob).await;
  assert_eq!(status, StatusCode::OK);
  assert!(content_type.starts_with("text/plain"), "{content_type}");
  assert_eq!(bytes, SPEC.as_bytes());

  let (_, _, bytes) = fetch(&f, "/ui/p/demo/specs/2/spec?view=raw", &bob).await;
  assert_eq!(bytes, CRLF.as_bytes(), "CRLF kept byte for byte");

  let (status, _) = page(&f, "/ui/p/demo/specs/1/plan", &bob).await;
  assert_eq!(status, StatusCode::NOT_FOUND, "kind not written");
  let (status, _) = page(&f, "/ui/p/demo/specs/9/spec", &bob).await;
  assert_eq!(status, StatusCode::NOT_FOUND, "spec missing");
  let (status, _) = page(&f, "/ui/p/demo/specs/1/bogus", &bob).await;
  assert_eq!(status, StatusCode::NOT_FOUND, "unknown kind");
  let (status, _) = page(&f, "/ui/p/demo/specs/1/constitution", &bob).await;
  assert_eq!(
    status,
    StatusCode::NOT_FOUND,
    "constitution is not a spec kind"
  );
  let (status, _) = page(&f, "/ui/p/demo/specs/x/spec", &bob).await;
  assert!(status == StatusCode::NOT_FOUND || status == StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn constitution_page_and_large_documents() {
  let f = fixture();
  let bob = session(&f, "bob");
  let (status, html) = page(&f, "/ui/p/demo/constitution", &bob).await;
  assert_eq!(status, StatusCode::OK);
  assert!(html.contains("1.0"));
  assert!(html.contains("<h1>Constitution</h1>") || html.contains("Constitution"));
  assert!(html.contains("<li>Rule</li>") || html.contains("Rule"));
  assert!(html.contains("carol"));
  assert!(html.contains("href=\"/ui/p/demo/constitution?view=raw\""));
  assert!(html.contains("href=\"/ui/p/demo/constitution/history\""));
  let (_, content_type, bytes) = fetch(&f, "/ui/p/demo/constitution?view=raw", &bob).await;
  assert!(content_type.starts_with("text/plain"));
  assert_eq!(bytes, b"# Constitution\n\n1. Rule\n");

  let (status, _) = page(&f, "/ui/p/bare/constitution", &bob).await;
  assert_eq!(status, StatusCode::NOT_FOUND);

  let big = format!("# Big\n\n{}", "lorem ipsum dolor sit amet\n".repeat(38_000));
  assert!(big.len() > 1_000_000 && big.len() <= 1_048_576);
  f.store
    .put_document(
      &f.store.caller_by_handle("bob").unwrap(),
      "demo",
      1,
      DocumentKind::Plan,
      &big,
      "big",
      None,
    )
    .unwrap();
  let (status, html) = page(&f, "/ui/p/demo/specs/1/plan", &bob).await;
  assert_eq!(status, StatusCode::OK);
  assert!(html.contains("<h1>Big</h1>"));
  let (_, _, bytes) = fetch(&f, "/ui/p/demo/specs/1/plan?view=raw", &bob).await;
  assert_eq!(bytes, big.as_bytes());
}
