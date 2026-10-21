mod common;

use std::time::{Duration, Instant};

use diesel::connection::SimpleConnection;
use spec_store::store::Store;

const SPECS: i64 = 100;
const LIMIT: Duration = Duration::from_secs(1);

#[test]
#[ignore]
fn opening_a_full_database_takes_under_one_second() {
  let path = common::previous_release_db("perf-open", 2);
  {
    let mut conn = common::raw_connection(&path);
    for n in 2..=SPECS + 1 {
      conn
        .batch_execute(&format!(
          "INSERT INTO specs (project_id, number, slug, status, branch)
           VALUES (1, {n}, 'spec-{n}', 'draft', printf('%03d-spec-{n}', {n}));"
        ))
        .unwrap();
    }
  }
  for label in ["upgrade", "steady state"] {
    let started = Instant::now();
    let store = Store::open(&path).unwrap();
    let elapsed = started.elapsed();
    let alice = store.caller_by_handle("alice").unwrap();
    assert_eq!(
      store.list_specs(&alice, "demo").unwrap().len() as i64,
      SPECS + 1
    );
    assert!(elapsed < LIMIT, "{label} open took {elapsed:?}");
  }
}
