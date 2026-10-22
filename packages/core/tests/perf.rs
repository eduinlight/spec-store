mod common;

use std::time::{Duration, Instant};

use spec_store_core::model::{DocumentKind, Role};
use spec_store_core::store::Store;

const THREADS: usize = 10;
const READS_PER_THREAD: usize = 20;
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

#[test]
#[ignore = "performance check, run with --release and --ignored"]
fn p95_read_under_100ms() {
  let test_store = common::test_store_with_pool(spec_store_core::db::POOL_SIZE);
  let store: Store = test_store.store.clone();
  let root = common::person(&store, "root");
  common::organization(&store, &root, common::ORG);
  let editor = common::person(&store, "editor");
  common::join(&store, &root, common::ORG, &editor);
  store.create_project(&root, "acme/demo", "Demo").unwrap();
  store
    .set_member(&root, "acme/demo", "editor", Role::Editor)
    .unwrap();
  store.create_spec(&editor, "acme/demo", "large").unwrap();
  let body = large_spec();
  assert!(body.len() >= BODY_BYTES);
  store
    .put_document(
      &editor,
      "acme/demo",
      1,
      DocumentKind::Spec,
      &body,
      "large",
      None,
    )
    .unwrap();
  store
    .put_document(
      &editor,
      "acme/demo",
      1,
      DocumentKind::Tasks,
      &body.replace("- **FR-", "- [ ] T01 (FR-1) FR-"),
      "tasks",
      None,
    )
    .ok();

  let handles: Vec<_> = (0..THREADS)
    .map(|_| {
      let store = store.clone();
      let caller = editor.clone();
      std::thread::spawn(move || {
        let mut samples = Vec::with_capacity(READS_PER_THREAD);
        for _ in 0..READS_PER_THREAD {
          let start = Instant::now();
          let doc = store
            .get_document(&caller, "acme/demo", 1, DocumentKind::Spec)
            .unwrap();
          store.project_status(&caller, "acme/demo").unwrap();
          samples.push(start.elapsed());
          assert!(doc.body.len() >= BODY_BYTES);
        }
        samples
      })
    })
    .collect();
  let samples: Vec<Duration> = handles
    .into_iter()
    .flat_map(|h| h.join().unwrap())
    .collect();
  assert_eq!(samples.len(), THREADS * READS_PER_THREAD);
  let p95 = percentile(samples, 0.95);
  eprintln!("p95 of a document read plus project status: {p95:?}");
  assert!(p95 < P95_LIMIT, "p95 {p95:?} exceeds {P95_LIMIT:?}");
}
