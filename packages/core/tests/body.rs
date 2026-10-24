mod common;

use sdd_vault_core::model::{DocumentKind, MAX_BODY_BYTES, Role, StoreError};

fn setup() -> (common::TestStore, sdd_vault_core::model::Caller) {
  let store = common::test_store();
  let owner = common::person(&store, "owner");
  common::organization(&store, &owner, common::ORG);
  store
    .create_project(&owner, &common::project("demo"), "Demo")
    .unwrap();
  store
    .create_spec(&owner, &common::project("demo"), "first")
    .unwrap();
  let _ = Role::Owner;
  (store, owner)
}

#[test]
fn crlf_bodies_round_trip_byte_for_byte() {
  let (store, owner) = setup();
  let body = "# Spec\r\n\r\nLine one\r\nLine two\r\n";
  store
    .put_document(
      &owner,
      "acme/demo",
      1,
      DocumentKind::Spec,
      body,
      "crlf",
      None,
    )
    .unwrap();
  let read = store
    .get_document(&owner, "acme/demo", 1, DocumentKind::Spec)
    .unwrap();
  assert_eq!(read.body.as_bytes(), body.as_bytes());
}

#[test]
fn bodies_up_to_one_mebibyte_are_stored_and_larger_rejected() {
  let (store, owner) = setup();
  let largest = "x".repeat(MAX_BODY_BYTES);
  store
    .put_document(
      &owner,
      "acme/demo",
      1,
      DocumentKind::Plan,
      &largest,
      "max",
      None,
    )
    .unwrap();
  assert_eq!(
    store
      .get_document(&owner, "acme/demo", 1, DocumentKind::Plan)
      .unwrap()
      .body
      .len(),
    MAX_BODY_BYTES
  );
  let error = store
    .put_document(
      &owner,
      "acme/demo",
      1,
      DocumentKind::Tasks,
      &format!("{largest}x"),
      "big",
      None,
    )
    .unwrap_err();
  assert!(matches!(error, StoreError::Validation(_)), "{error:?}");
}

#[test]
fn nul_is_rejected_with_a_validation_error() {
  let (store, owner) = setup();
  let error = store
    .put_document(
      &owner,
      "acme/demo",
      1,
      DocumentKind::Spec,
      "a\0b",
      "nul",
      None,
    )
    .unwrap_err();
  match error {
    StoreError::Validation(message) => assert!(message.contains("NUL"), "{message}"),
    other => panic!("expected validation, got {other:?}"),
  }
}
