mod common;

use std::time::{Duration, Instant};

use axum::body::Body;
use axum::http::{Request, StatusCode};
use sdd_vault_core::model::{DocumentKind, Role};

const CALLERS: usize = 10;
const READS_PER_CALLER: usize = 20;
const BODY_BYTES: usize = 200 * 1024;
const P95_LIMIT: Duration = Duration::from_millis(100);

fn large_spec() -> String {
  let mut body = String::from("# Spec 001 — Large\n\n## 5. Functional requirements\n");
  let mut n = 1;
  while body.len() < BODY_BYTES {
    body.push_str(&format!(
      "- **FR-{n}** (US-1): WHEN the caller sends request number {n}, THE SYSTEM SHALL answer within the agreed budget and record the outcome.\n"
    ));
    n += 1;
  }
  body
}

fn percentile(mut samples: Vec<Duration>, p: f64) -> Duration {
  samples.sort();
  let index = ((samples.len() as f64 - 1.0) * p).round() as usize;
  samples[index]
}

async fn read(app: &common::TestApp, path: &str, token: &str) -> usize {
  let response = app
    .send(
      Request::get(path)
        .header("host", "api.example.test")
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap(),
    )
    .await;
  assert_eq!(response.status(), StatusCode::OK, "{path}");
  common::body_text(response).await.len()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 10)]
#[ignore = "performance check, run in release by make test"]
async fn p95_read_of_a_200_kb_spec_under_100ms_with_10_callers() {
  let app = std::sync::Arc::new(common::app_with_pool(common::config(), None, 10).await);
  app.organization("alice", "acme").await;
  app.admit("acme", "rita");
  let store = &app.state.store;
  let alice = store.caller_by_handle("alice", false).unwrap();
  store.create_project(&alice, "acme/demo", "Demo").unwrap();
  store
    .set_member(&alice, "acme/demo", "rita", Role::Reader)
    .unwrap();
  store.create_spec(&alice, "acme/demo", "large").unwrap();
  let body = large_spec();
  store
    .put_document(
      &alice,
      "acme/demo",
      1,
      DocumentKind::Spec,
      &body,
      "large",
      None,
    )
    .unwrap();
  let token = app.token("rita");
  read(&app, "/api/v1/projects/acme/demo/status", &token).await;

  let callers: Vec<_> = (0..CALLERS)
    .map(|_| {
      let app = app.clone();
      let token = token.clone();
      tokio::spawn(async move {
        let mut samples = Vec::with_capacity(READS_PER_CALLER);
        for _ in 0..READS_PER_CALLER {
          let started = Instant::now();
          let document = read(
            &app,
            "/api/v1/projects/acme/demo/specs/1/documents/spec",
            &token,
          )
          .await;
          read(&app, "/api/v1/projects/acme/demo/status", &token).await;
          samples.push(started.elapsed());
          assert!(document >= BODY_BYTES);
        }
        samples
      })
    })
    .collect();
  let mut samples = Vec::with_capacity(CALLERS * READS_PER_CALLER);
  for caller in callers {
    samples.extend(caller.await.unwrap());
  }
  assert_eq!(samples.len(), CALLERS * READS_PER_CALLER);
  let p95 = percentile(samples, 0.95);
  eprintln!("p95 of a document read plus project status over the API: {p95:?}");
  assert!(p95 < P95_LIMIT, "p95 {p95:?} exceeds {P95_LIMIT:?}");
}
