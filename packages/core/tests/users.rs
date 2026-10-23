mod common;

use spec_store_core::model::StoreError;
use spec_store_core::store::users::Identity;

fn identity(subject: &str, handle: &str, email: &str, name: &str) -> Identity {
  Identity {
    subject: subject.into(),
    handle: handle.into(),
    email: email.into(),
    name: name.into(),
  }
}

#[test]
fn first_sign_in_creates_the_profile_from_the_identity() {
  let store = common::test_store();
  let user = store
    .upsert_user_from_identity(&identity("kc-1", "alice", "Alice@Example.com", "Alice Doe"))
    .unwrap();
  assert_eq!(user.handle, "alice");
  assert_eq!(user.email, "alice@example.com");
  assert_eq!(user.name, "Alice Doe");
  assert!(user.created_at.ends_with('Z'));
  assert_eq!(store.user_by_subject("kc-1").unwrap(), user);
}

#[test]
fn later_sign_ins_update_email_and_name_but_keep_the_handle() {
  let store = common::test_store();
  let first = store
    .upsert_user_from_identity(&identity("kc-1", "alice", "alice@example.com", "Alice"))
    .unwrap();
  let second = store
    .upsert_user_from_identity(&identity(
      "kc-1",
      "alice-renamed",
      "new@example.com",
      "Alice New",
    ))
    .unwrap();
  assert_eq!(second.id, first.id);
  assert_eq!(second.handle, "alice");
  assert_eq!(second.email, "new@example.com");
  assert_eq!(second.name, "Alice New");
  assert_eq!(common::int(&store, "SELECT count(*) FROM users"), 1);
}

#[test]
fn a_handle_taken_by_another_identity_is_refused() {
  let store = common::test_store();
  store
    .upsert_user_from_identity(&identity("kc-1", "alice", "a@example.com", "A"))
    .unwrap();
  match store.upsert_user_from_identity(&identity("kc-2", "alice", "b@example.com", "B")) {
    Err(StoreError::Validation(message)) => assert!(message.contains("alice"), "{message}"),
    other => panic!("expected a validation error, got {other:?}"),
  }
}

#[test]
fn identities_without_subject_or_username_are_refused() {
  let store = common::test_store();
  assert!(matches!(
    store.upsert_user_from_identity(&identity("", "alice", "a@example.com", "A")),
    Err(StoreError::Validation(_))
  ));
  assert!(matches!(
    store.upsert_user_from_identity(&identity("kc-1", "", "a@example.com", "A")),
    Err(StoreError::Validation(_))
  ));
}
