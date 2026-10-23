mod common;

use spec_store_core::model::StoreError;
use spec_store_core::store::sessions::NewSession;

fn new_session(user_id: i64, sid: &str) -> NewSession {
  NewSession {
    user_id,
    identity_session_id: Some(sid.to_string()),
    refresh_token_sealed: b"sealed-refresh".to_vec(),
    id_token_sealed: b"sealed-id".to_vec(),
    access_expires_in_seconds: 300,
    is_operator: false,
  }
}

#[test]
fn a_session_is_found_by_its_secret_and_ended() {
  let store = common::test_store();
  let alice = common::person(&store, "alice");
  let secret = store
    .create_session(&new_session(alice.user_id, "kc-sid-1"))
    .unwrap();
  assert!(secret.len() >= 40);
  let stored_hash = common::text(&store, "SELECT secret_hash FROM browser_sessions");
  assert_ne!(stored_hash, secret, "only a hash of the secret is stored");
  let session = store.session(&secret).unwrap();
  assert_eq!(session.caller.handle, "alice");
  assert!(!session.caller.is_operator);
  assert!(!session.access_expired);
  assert_eq!(session.refresh_token_sealed, b"sealed-refresh");
  assert_eq!(session.id_token_sealed, b"sealed-id");
  store.end_session(&secret).unwrap();
  assert!(matches!(
    store.session(&secret),
    Err(StoreError::Unauthenticated)
  ));
  assert!(matches!(
    store.session("unknown"),
    Err(StoreError::Unauthenticated)
  ));
}

#[test]
fn an_expired_access_token_is_reported_and_refreshing_resets_it() {
  let store = common::test_store();
  let alice = common::person(&store, "alice");
  let secret = store
    .create_session(&new_session(alice.user_id, "kc-sid-1"))
    .unwrap();
  common::exec(
    &store,
    "UPDATE browser_sessions SET access_expires_at = now() - interval '1 second'",
  );
  let session = store.session(&secret).unwrap();
  assert!(session.access_expired);
  store
    .refresh_session(session.id, b"new-refresh", Some(b"new-id".as_slice()), 300)
    .unwrap();
  let refreshed = store.session(&secret).unwrap();
  assert!(!refreshed.access_expired);
  assert_eq!(refreshed.refresh_token_sealed, b"new-refresh");
  assert_eq!(refreshed.id_token_sealed, b"new-id");
}

#[test]
fn back_channel_logout_ends_every_session_of_an_identity_session() {
  let store = common::test_store();
  let alice = common::person(&store, "alice");
  let first = store
    .create_session(&new_session(alice.user_id, "kc-sid-1"))
    .unwrap();
  let second = store
    .create_session(&new_session(alice.user_id, "kc-sid-1"))
    .unwrap();
  let other = store
    .create_session(&new_session(alice.user_id, "kc-sid-2"))
    .unwrap();
  assert_eq!(store.end_identity_session("kc-sid-1").unwrap(), 2);
  assert!(store.session(&first).is_err());
  assert!(store.session(&second).is_err());
  assert!(store.session(&other).is_ok());
}

#[test]
fn login_attempts_are_single_use_and_expire_after_ten_minutes() {
  let store = common::test_store();
  store
    .begin_login("state-1", b"sealed-verifier", "/projects/acme")
    .unwrap();
  let attempt = store.finish_login("state-1").unwrap();
  assert_eq!(attempt.pkce_verifier_sealed, b"sealed-verifier");
  assert_eq!(attempt.return_to, "/projects/acme");
  assert!(matches!(
    store.finish_login("state-1"),
    Err(StoreError::Unauthenticated)
  ));

  store.begin_login("state-2", b"v", "/").unwrap();
  common::exec(
    &store,
    "UPDATE login_attempts SET created_at = now() - interval '601 seconds'",
  );
  assert!(matches!(
    store.finish_login("state-2"),
    Err(StoreError::Unauthenticated)
  ));
  assert_eq!(
    common::int(&store, "SELECT count(*) FROM login_attempts"),
    0
  );
}
