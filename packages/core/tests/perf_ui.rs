mod common;

use std::time::{Duration, Instant};

use axum::body::Body;
use axum::http::{Request, header};
use http_body_util::BodyExt;
use spec_store::model::{DocumentKind, Role};
use spec_store::server::{ServerConfig, build_router};
use spec_store::store::Store;
use tower::ServiceExt;

const BODY_BYTES: usize = 200 * 1024;
const THREADS: usize = 10;
const REQUESTS_PER_THREAD: usize = 20;
const LIMIT: Duration = Duration::from_millis(300);

fn large_spec() -> String {
  let mut body = String::from("# Spec 001 — Large\n\n");
  let mut n = 0;
  while body.len() < BODY_BYTES {
    n += 1;
    body.push_str(&format!(
      "## Section {n}\n\n- **FR-{n}** (US-1): WHEN something happens, THE SYSTEM SHALL respond within a bounded time and record it.\n\n| a | b |\n|---|---|\n| {n} | value |\n\n"
    ));
  }
  body
}

fn percentile(samples: &mut [Duration], p: f64) -> Duration {
  samples.sort();
  let rank = ((samples.len() as f64) * p).ceil() as usize;
  samples[rank.saturating_sub(1).min(samples.len() - 1)]
}

#[test]
#[ignore = "performance check, run with --release and --ignored"]
fn p95_spec_page_under_300ms() {
  let store = Store::open(common::temp_db_path("perf-ui")).unwrap();
  store.create_user(None, "root", false).unwrap();
  let admin = store.caller_by_handle("root").unwrap();
  store.create_user(Some(&admin), "bob", false).unwrap();
  store.create_project(&admin, "demo", "Demo").unwrap();
  store
    .set_member(&admin, "demo", "bob", Role::Editor)
    .unwrap();
  let bob = store.caller_by_handle("bob").unwrap();
  store.create_spec(&bob, "demo", "large").unwrap();
  let body = large_spec();
  assert!(body.len() >= BODY_BYTES);
  store
    .put_document(&bob, "demo", 1, DocumentKind::Spec, &body, "big", None)
    .unwrap();
  let token = store
    .issue_token(Some(&admin), "bob", "perf")
    .unwrap()
    .token;
  let secret = store.create_session(&token).unwrap();

  let handles: Vec<_> = (0..THREADS)
    .map(|_| {
      let store = store.clone();
      let secret = secret.clone();
      std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
          .enable_all()
          .build()
          .unwrap();
        let mut samples = Vec::with_capacity(REQUESTS_PER_THREAD);
        for _ in 0..REQUESTS_PER_THREAD {
          let app = build_router(store.clone(), ServerConfig::default());
          let request = Request::builder()
            .uri("/ui/p/demo/specs/1")
            .header(header::HOST, "localhost")
            .header(header::COOKIE, format!("spec_store_session={secret}"))
            .body(Body::empty())
            .unwrap();
          let started = Instant::now();
          runtime.block_on(async {
            let response = app.oneshot(request).await.unwrap();
            assert_eq!(response.status(), 200);
            let bytes = response.into_body().collect().await.unwrap().to_bytes();
            assert!(bytes.len() > BODY_BYTES);
          });
          samples.push(started.elapsed());
        }
        samples
      })
    })
    .collect();
  let mut samples: Vec<Duration> = handles
    .into_iter()
    .flat_map(|h| h.join().unwrap())
    .collect();
  let p95 = percentile(&mut samples, 0.95);
  println!("p95 of the spec page for a 200 KB spec: {p95:?}");
  assert!(p95 < LIMIT, "p95 {p95:?} exceeds {LIMIT:?}");
}
