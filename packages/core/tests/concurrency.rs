mod common;

use std::sync::{Arc, Barrier};
use std::thread;

use spec_store_core::model::{Caller, DocumentKind, StoreError};
use spec_store_core::store::Store;

fn setup() -> (common::TestStore, Caller) {
  let store = common::test_store();
  let owner = common::person(&store, "owner");
  common::organization(&store, &owner, common::ORG);
  store
    .create_project(&owner, &common::project("demo"), "Demo")
    .unwrap();
  store
    .create_spec(&owner, &common::project("demo"), "first")
    .unwrap();
  (store, owner)
}

fn race<T, F>(
  store: &Store,
  caller: &Caller,
  attempts: usize,
  action: F,
) -> Vec<Result<T, StoreError>>
where
  T: Send + 'static,
  F: Fn(&Store, &Caller, usize) -> Result<T, StoreError> + Send + Sync + 'static,
{
  let barrier = Arc::new(Barrier::new(attempts));
  let action = Arc::new(action);
  let handles: Vec<_> = (0..attempts)
    .map(|i| {
      let store = store.clone();
      let caller = caller.clone();
      let barrier = Arc::clone(&barrier);
      let action = Arc::clone(&action);
      thread::spawn(move || {
        barrier.wait();
        action(&store, &caller, i)
      })
    })
    .collect();
  handles.into_iter().map(|h| h.join().unwrap()).collect()
}

#[test]
fn racing_writes_with_the_same_expected_revision_accept_exactly_one() {
  let (store, owner) = setup();
  let first = store
    .put_document(&owner, "acme/demo", 1, DocumentKind::Spec, "v1", "v1", None)
    .unwrap();
  let expected = first;
  let results = race(&store, &owner, 4, move |store, caller, i| {
    store.put_document(
      caller,
      "acme/demo",
      1,
      DocumentKind::Spec,
      &format!("v2-{i}"),
      "v2",
      Some(expected),
    )
  });
  let winners: Vec<i64> = results
    .iter()
    .filter_map(|r| r.as_ref().ok().copied())
    .collect();
  assert_eq!(winners.len(), 1, "{results:?}");
  for result in &results {
    if let Err(error) = result {
      match error {
        StoreError::Conflict { current_revision } => assert_eq!(*current_revision, winners[0]),
        other => panic!("expected conflict, got {other:?}"),
      }
    }
  }
}

#[test]
fn racing_spec_creation_assigns_distinct_numbers() {
  let (store, owner) = setup();
  let results = race(&store, &owner, 4, |store, caller, i| {
    store.create_spec(caller, "acme/demo", &format!("racing-{i}"))
  });
  let mut numbers: Vec<i64> = results
    .into_iter()
    .map(|r| r.expect("created").number)
    .collect();
  numbers.sort();
  assert_eq!(numbers, vec![2, 3, 4, 5]);
}

#[test]
fn racing_first_constitution_writes_accept_exactly_one() {
  let (store, owner) = setup();
  let results = race(&store, &owner, 4, |store, caller, i| {
    store.put_constitution(caller, "acme/demo", "# C", &format!("1.{i}"), "init", None)
  });
  let winners = results.iter().filter(|r| r.is_ok()).count();
  assert_eq!(winners, 1, "{results:?}");
  assert!(
    results
      .iter()
      .filter_map(|r| r.as_ref().err())
      .all(|e| matches!(e, StoreError::Conflict { .. })),
    "{results:?}"
  );
}
