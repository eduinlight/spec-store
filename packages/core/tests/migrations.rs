mod common;

use std::sync::{Arc, Barrier};
use std::thread;

use sdd_vault_core::db;

#[test]
fn concurrent_start_applies_once() {
  let test_db = sdd_vault_testkit::TestDb::empty();
  let url = Arc::new(test_db.url.clone());
  let barrier = Arc::new(Barrier::new(4));
  let handles: Vec<_> = (0..4)
    .map(|_| {
      let url = Arc::clone(&url);
      let barrier = Arc::clone(&barrier);
      thread::spawn(move || {
        barrier.wait();
        db::open_with_size(&url, 1).map(|_| ())
      })
    })
    .collect();
  for handle in handles {
    handle.join().unwrap().expect("every instance starts");
  }
  let pool = db::open_with_size(&url, 1).unwrap();
  let mut conn = pool.get().unwrap();
  assert_eq!(common::diesel_versions(&mut conn), vec!["0001", "0002"]);
}
