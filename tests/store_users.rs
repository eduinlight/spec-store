mod common;

use spec_store::model::{Caller, StoreError};
use spec_store::store::Store;

fn store() -> Store {
  Store::open(common::temp_db_path("store-users")).expect("open store")
}

fn caller(store: &Store, handle: &str) -> Caller {
  store.caller_by_handle(handle).unwrap()
}

#[test]
fn creates_user_and_rejects_duplicate_handle() {
  let store = store();
  let alice = store.create_user(None, "alice", false).unwrap();
  assert_eq!(alice.handle, "alice");
  assert!(alice.is_admin, "first user becomes admin");
  let admin = caller(&store, "alice");

  let bob = store.create_user(Some(&admin), "bob", false).unwrap();
  assert!(!bob.is_admin);

  let err = store.create_user(Some(&admin), "bob", false).unwrap_err();
  match err {
    StoreError::Validation(msg) => assert!(msg.contains("bob"), "{msg}"),
    other => panic!("expected validation error, got {other:?}"),
  }
  assert!(matches!(
    store.create_user(Some(&admin), "Bad Handle", false),
    Err(StoreError::Validation(_))
  ));
}

#[test]
fn later_users_require_an_admin_actor() {
  let store = store();
  store.create_user(None, "alice", false).unwrap();
  assert!(matches!(
    store.create_user(None, "bob", false),
    Err(StoreError::Forbidden)
  ));
  let admin = caller(&store, "alice");
  store.create_user(Some(&admin), "bob", false).unwrap();
  let bob = caller(&store, "bob");
  assert!(matches!(
    store.create_user(Some(&bob), "carol", false),
    Err(StoreError::Forbidden)
  ));
  assert!(matches!(
    store.caller_by_handle("nobody"),
    Err(StoreError::NotFound(_))
  ));
}

#[test]
fn issues_token_once_and_lists_without_secret() {
  let store = store();
  store.create_user(None, "alice", false).unwrap();
  let admin = caller(&store, "alice");
  store.create_user(Some(&admin), "bob", false).unwrap();

  let issued = store.issue_token(Some(&admin), "bob", "laptop").unwrap();
  assert!(issued.token.starts_with("sst_"), "{}", issued.token);
  assert!(issued.token.len() >= 40, "{}", issued.token);

  let tokens = store.list_tokens(Some(&admin), "bob").unwrap();
  assert_eq!(tokens.len(), 1);
  assert_eq!(tokens[0].id, issued.id);
  assert_eq!(tokens[0].label, "laptop");
  assert!(tokens[0].last_used_at.is_none());
  assert!(tokens[0].revoked_at.is_none());
  let dump = format!("{tokens:?}");
  assert!(!dump.contains(&issued.token), "token must not be listed");

  let stored_hash: String = store
    .with_conn(|conn| Ok(conn.query_row("SELECT token_hash FROM api_tokens", [], |r| r.get(0))?))
    .unwrap();
  assert_ne!(stored_hash, issued.token);
  assert!(!stored_hash.contains(issued.token.trim_start_matches("sst_")));

  let bob = caller(&store, "bob");
  assert!(matches!(
    store.issue_token(Some(&bob), "bob", "self"),
    Err(StoreError::Forbidden)
  ));
  assert!(matches!(
    store.issue_token(Some(&admin), "nobody", "x"),
    Err(StoreError::NotFound(_))
  ));
}

#[test]
fn authenticate_resolves_user_and_updates_last_used() {
  let store = store();
  store.create_user(None, "alice", false).unwrap();
  let admin = caller(&store, "alice");
  let issued = store.issue_token(Some(&admin), "alice", "cli").unwrap();

  let who = store.authenticate(&issued.token).unwrap();
  assert_eq!(who.handle, "alice");
  assert!(who.is_admin);
  let tokens = store.list_tokens(Some(&admin), "alice").unwrap();
  assert!(tokens[0].last_used_at.is_some());

  assert!(matches!(
    store.authenticate("sst_unknown"),
    Err(StoreError::Unauthenticated)
  ));
  assert!(matches!(
    store.authenticate(""),
    Err(StoreError::Unauthenticated)
  ));
}

#[test]
fn revoked_token_fails_other_tokens_work() {
  let store = store();
  store.create_user(None, "alice", false).unwrap();
  let admin = caller(&store, "alice");
  let first = store.issue_token(Some(&admin), "alice", "one").unwrap();
  let second = store.issue_token(Some(&admin), "alice", "two").unwrap();

  store.revoke_token(Some(&admin), first.id).unwrap();
  assert!(matches!(
    store.authenticate(&first.token),
    Err(StoreError::Unauthenticated)
  ));
  assert_eq!(store.authenticate(&second.token).unwrap().handle, "alice");

  let tokens = store.list_tokens(Some(&admin), "alice").unwrap();
  let revoked = tokens.iter().find(|t| t.id == first.id).unwrap();
  assert!(revoked.revoked_at.is_some());
  assert!(matches!(
    store.revoke_token(Some(&admin), 999),
    Err(StoreError::NotFound(_))
  ));
}

#[test]
fn token_helpers_are_deterministic_and_prefixed() {
  let token = spec_store::auth::generate_token();
  assert!(token.starts_with("sst_"));
  assert_ne!(token, spec_store::auth::generate_token());
  let hash = spec_store::auth::hash_token(&token);
  assert_eq!(hash.len(), 64);
  assert_eq!(hash, spec_store::auth::hash_token(&token));
  assert!(hash.bytes().all(|b| b.is_ascii_hexdigit()));
}
