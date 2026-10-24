mod common;

use std::sync::{Arc, Barrier};
use std::thread;

use sdd_vault_core::store::outbox::{OutboxMessage, RETRY_WINDOW_SECONDS};

fn enqueue(store: &sdd_vault_core::store::Store, n: usize) {
  for i in 0..n {
    store
      .with_tx(|tx| {
        sdd_vault_core::store::outbox::enqueue(
          tx,
          "invitation",
          &format!("person{i}@example.com"),
          "en",
          "{}",
        )
      })
      .unwrap();
  }
}

#[test]
fn messages_are_enqueued_inside_the_callers_transaction() {
  let store = common::test_store();
  let result: Result<(), sdd_vault_core::model::StoreError> = store.with_tx(|tx| {
    sdd_vault_core::store::outbox::enqueue(tx, "invitation", "a@example.com", "en", "{}")?;
    Err(sdd_vault_core::model::StoreError::Validation(
      "abort".into(),
    ))
  });
  assert!(result.is_err());
  assert_eq!(common::int(&store, "SELECT count(*) FROM email_outbox"), 0);
}

#[test]
fn concurrent_claims_get_different_messages() {
  let store = common::test_store();
  enqueue(&store, 6);
  let barrier = Arc::new(Barrier::new(2));
  let handles: Vec<_> = (0..2)
    .map(|_| {
      let store = store.store.clone();
      let barrier = Arc::clone(&barrier);
      thread::spawn(move || {
        barrier.wait();
        store.claim_due_messages(3).unwrap()
      })
    })
    .collect();
  let mut ids: Vec<i64> = handles
    .into_iter()
    .flat_map(|h| h.join().unwrap())
    .map(|m: OutboxMessage| m.id)
    .collect();
  let total = ids.len();
  ids.sort();
  ids.dedup();
  assert_eq!(ids.len(), total, "no message is claimed twice");
  assert_eq!(total, 6);
  assert!(
    store.claim_due_messages(10).unwrap().is_empty(),
    "claimed messages are leased"
  );
}

#[test]
fn sent_messages_are_not_claimed_again() {
  let store = common::test_store();
  enqueue(&store, 1);
  let claimed = store.claim_due_messages(10).unwrap();
  store.mark_message_sent(claimed[0].id).unwrap();
  common::exec(
    &store,
    "UPDATE email_outbox SET next_attempt_at = now() - interval '1 hour'",
  );
  assert!(store.claim_due_messages(10).unwrap().is_empty());
}

#[test]
fn failures_are_retried_until_the_window_closes_then_marked_failed() {
  let store = common::test_store();
  enqueue(&store, 1);
  let claimed = store.claim_due_messages(10).unwrap();
  assert_eq!(claimed[0].attempts, 1);
  let failed = store.mark_message_attempt_failed(claimed[0].id).unwrap();
  assert!(!failed, "still inside the retry window");
  common::exec(
    &store,
    "UPDATE email_outbox SET next_attempt_at = now() - interval '1 second'",
  );
  let again = store.claim_due_messages(10).unwrap();
  assert_eq!(again.len(), 1);
  assert_eq!(again[0].attempts, 2);
  common::exec(
    &store,
    &format!(
      "UPDATE email_outbox SET created_at = now() - interval '{} seconds'",
      RETRY_WINDOW_SECONDS + 1
    ),
  );
  let failed = store.mark_message_attempt_failed(again[0].id).unwrap();
  assert!(failed, "the retry window has closed");
  common::exec(
    &store,
    "UPDATE email_outbox SET next_attempt_at = now() - interval '1 hour'",
  );
  assert!(store.claim_due_messages(10).unwrap().is_empty());
  assert_eq!(
    common::int(
      &store,
      "SELECT count(*) FROM email_outbox WHERE failed_at IS NOT NULL"
    ),
    1
  );
}
