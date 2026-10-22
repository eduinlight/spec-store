mod common;

use std::sync::{Arc, Barrier, mpsc};
use std::time::{Duration, Instant};

use diesel::connection::SimpleConnection;
use spec_store::db::FILE_POOL_SIZE;
use spec_store::model::{Caller, DocumentKind, Role, StoreError};
use spec_store::store::Store;

struct Fixture {
  store: Store,
  editor: Caller,
}

fn fixture(label: &str) -> Fixture {
  let store = Store::open(common::temp_db_path(label)).unwrap();
  store.create_user(None, "root", false).unwrap();
  let admin = store.caller_by_handle("root").unwrap();
  store.create_user(Some(&admin), "editor", false).unwrap();
  store.create_project(&admin, "demo", "Demo").unwrap();
  store
    .set_member(&admin, "demo", "editor", Role::Editor)
    .unwrap();
  let editor = store.caller_by_handle("editor").unwrap();
  store.create_spec(&editor, "demo", "first").unwrap();
  Fixture { store, editor }
}

fn counts(store: &Store) -> Vec<i64> {
  ["documents", "revisions", "audit_log"]
    .into_iter()
    .map(|table| common::int(store, &format!("SELECT count(*) FROM {table}")))
    .collect()
}

#[test]
fn file_pool_puts_exactly_one_wins() {
  let f = fixture("pool-race");
  let mut current = None;
  for round in 0..20 {
    let base = f
      .store
      .put_document(
        &f.editor,
        "demo",
        1,
        DocumentKind::Tasks,
        &format!("base {round}"),
        "base",
        current,
      )
      .unwrap();
    let barrier = Arc::new(Barrier::new(2));
    let handles: Vec<_> = (0..2)
      .map(|i| {
        let store = f.store.clone();
        let caller = f.editor.clone();
        let barrier = barrier.clone();
        std::thread::spawn(move || {
          barrier.wait();
          store.put_document(
            &caller,
            "demo",
            1,
            DocumentKind::Tasks,
            &format!("round {round} writer {i}"),
            "race",
            Some(base),
          )
        })
      })
      .collect();
    let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    let wins = results.iter().filter(|r| r.is_ok()).count();
    let conflicts = results
      .iter()
      .filter(|r| matches!(r, Err(StoreError::Conflict { .. })))
      .count();
    assert_eq!((wins, conflicts), (1, 1), "round {round}: {results:?}");
    current = results.into_iter().find_map(Result::ok);
  }
  assert_eq!(common::int(&f.store, "SELECT count(*) FROM revisions"), 40);
}

#[test]
fn failed_write_leaves_no_rows() {
  let f = fixture("pool-failed-write");
  common::exec(
    &f.store,
    "CREATE TRIGGER abort_document_put BEFORE INSERT ON audit_log
     WHEN NEW.action = 'document.put'
     BEGIN SELECT RAISE(ABORT, 'forced failure'); END;",
  );
  let before = counts(&f.store);
  let result = f.store.put_document(
    &f.editor,
    "demo",
    1,
    DocumentKind::Plan,
    "# Plan",
    "first",
    None,
  );
  assert!(matches!(result, Err(StoreError::Database(_))), "{result:?}");
  assert_eq!(counts(&f.store), before);
}

fn on_every_pooled_connection<T: Send + 'static>(
  store: &Store,
  check: fn(&mut diesel::SqliteConnection) -> T,
) -> Vec<T> {
  let size = FILE_POOL_SIZE as usize;
  let barrier = Arc::new(Barrier::new(size));
  let handles: Vec<_> = (0..size)
    .map(|_| {
      let store = store.clone();
      let barrier = barrier.clone();
      std::thread::spawn(move || {
        store
          .with_conn(|conn| {
            barrier.wait();
            let value = check(conn);
            barrier.wait();
            Ok(value)
          })
          .unwrap()
      })
    })
    .collect();
  handles.into_iter().map(|h| h.join().unwrap()).collect()
}

#[test]
fn every_pooled_connection_enforces_foreign_keys() {
  let f = fixture("pool-foreign-keys");
  let results = on_every_pooled_connection(&f.store, |conn| {
    let enabled = common::foreign_keys(conn);
    let insert = conn.batch_execute(
      "INSERT INTO memberships (project_id, user_id, role) VALUES (999, 1, 'reader')",
    );
    (enabled, insert.is_err())
  });
  assert_eq!(results, vec![(1, true); FILE_POOL_SIZE as usize]);
  assert_eq!(common::int(&f.store, "SELECT count(*) FROM memberships"), 1);
}

#[test]
fn every_pooled_connection_waits_and_uses_wal() {
  let f = fixture("pool-settings");
  let results = on_every_pooled_connection(&f.store, |conn| {
    (common::busy_timeout(conn), common::journal_mode(conn))
  });
  assert_eq!(
    results,
    vec![(5000, "wal".to_string()); FILE_POOL_SIZE as usize]
  );
}

#[test]
fn exhausted_pool_fails_after_timeout() {
  let f = fixture("pool-exhausted");
  let size = FILE_POOL_SIZE as usize;
  let held = Arc::new(Barrier::new(size + 1));
  let (release, released) = mpsc::channel::<()>();
  let released = Arc::new(std::sync::Mutex::new(released));
  let handles: Vec<_> = (0..size)
    .map(|_| {
      let store = f.store.clone();
      let held = held.clone();
      let released = released.clone();
      std::thread::spawn(move || {
        store
          .with_conn(|_| {
            held.wait();
            let _ = released
              .lock()
              .unwrap()
              .recv_timeout(Duration::from_secs(30));
            Ok(())
          })
          .unwrap()
      })
    })
    .collect();
  held.wait();
  let started = Instant::now();
  let result = f.store.list_projects(&f.editor);
  let waited = started.elapsed();
  for _ in 0..size {
    release.send(()).unwrap();
  }
  for handle in handles {
    handle.join().unwrap();
  }
  assert!(matches!(result, Err(StoreError::Pool(_))), "{result:?}");
  assert!(
    waited >= Duration::from_millis(4500) && waited <= Duration::from_secs(10),
    "{waited:?}"
  );
  assert_eq!(f.store.list_projects(&f.editor).unwrap().len(), 1);
}
