mod common;

use spec_store::auth::hash_token;
use spec_store::model::StoreError;
use spec_store::store::Store;

struct Fixture {
  store: Store,
  token: String,
  token_id: i64,
}

fn fixture() -> Fixture {
  let store = Store::open(common::temp_db_path("store-sessions")).unwrap();
  store.create_user(None, "root", false).unwrap();
  let admin = store.caller_by_handle("root").unwrap();
  store.create_user(Some(&admin), "bob", false).unwrap();
  let issued = store.issue_token(Some(&admin), "bob", "laptop").unwrap();
  Fixture {
    store,
    token: issued.token,
    token_id: issued.id,
  }
}

fn stored_hashes(store: &Store) -> Vec<String> {
  store
    .with_conn(|c| {
      let mut stmt = c.prepare("SELECT secret_hash FROM sessions ORDER BY id")?;
      let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
      Ok(rows.collect::<Result<Vec<_>, _>>()?)
    })
    .unwrap()
}

#[test]
fn create_session_returns_a_prefixed_secret_stored_only_hashed() {
  let f = fixture();
  let secret = f.store.create_session(&f.token).unwrap();
  assert!(secret.starts_with("sss_"), "{secret}");
  assert!(secret.len() > 40);
  let hashes = stored_hashes(&f.store);
  assert_eq!(hashes, vec![hash_token(&secret)]);
  assert!(!hashes[0].contains(&secret));
  assert!(!hashes[0].contains(&f.token));
}

#[test]
fn create_session_rejects_unknown_and_revoked_tokens() {
  let f = fixture();
  assert!(matches!(
    f.store.create_session("sst_nope"),
    Err(StoreError::Unauthenticated)
  ));
  assert!(matches!(
    f.store.create_session(""),
    Err(StoreError::Unauthenticated)
  ));
  let admin = f.store.caller_by_handle("root").unwrap();
  f.store.revoke_token(Some(&admin), f.token_id).unwrap();
  assert!(matches!(
    f.store.create_session(&f.token),
    Err(StoreError::Unauthenticated)
  ));
  assert!(stored_hashes(&f.store).is_empty());
}

#[test]
fn session_caller_resolves_user_and_updates_last_used() {
  let f = fixture();
  let secret = f.store.create_session(&f.token).unwrap();
  let before: Option<String> = f
    .store
    .with_conn(|c| {
      Ok(c.query_row(
        "SELECT last_used_at FROM api_tokens WHERE id = ?1",
        [f.token_id],
        |r| r.get(0),
      )?)
    })
    .unwrap();
  std::thread::sleep(std::time::Duration::from_millis(5));
  let (caller, token_id) = f.store.session_caller(&secret).unwrap();
  assert_eq!(caller.handle, "bob");
  assert!(!caller.is_admin);
  assert_eq!(token_id, f.token_id);
  let after: Option<String> = f
    .store
    .with_conn(|c| {
      Ok(c.query_row(
        "SELECT last_used_at FROM api_tokens WHERE id = ?1",
        [f.token_id],
        |r| r.get(0),
      )?)
    })
    .unwrap();
  assert!(after.is_some());
  assert_ne!(before, after);
}

#[test]
fn unknown_ended_revoked_and_expired_sessions_are_unauthenticated() {
  let f = fixture();
  assert!(matches!(
    f.store.session_caller("sss_unknown"),
    Err(StoreError::Unauthenticated)
  ));
  assert!(matches!(
    f.store.session_caller(""),
    Err(StoreError::Unauthenticated)
  ));

  let ended = f.store.create_session(&f.token).unwrap();
  f.store.end_session(&ended).unwrap();
  f.store.end_session(&ended).unwrap();
  f.store.end_session("sss_unknown").unwrap();
  assert!(matches!(
    f.store.session_caller(&ended),
    Err(StoreError::Unauthenticated)
  ));

  let expired = f.store.create_session(&f.token).unwrap();
  f.store
    .with_tx(|tx| {
      tx.execute(
        "UPDATE sessions SET expires_at = '2000-01-01T00:00:00.000Z' WHERE secret_hash = ?1",
        [hash_token(&expired)],
      )?;
      Ok(())
    })
    .unwrap();
  assert!(matches!(
    f.store.session_caller(&expired),
    Err(StoreError::Unauthenticated)
  ));

  let revoked = f.store.create_session(&f.token).unwrap();
  f.store.session_caller(&revoked).unwrap();
  let admin = f.store.caller_by_handle("root").unwrap();
  f.store.revoke_token(Some(&admin), f.token_id).unwrap();
  assert!(matches!(
    f.store.session_caller(&revoked),
    Err(StoreError::Unauthenticated)
  ));
}

#[test]
fn sessions_expire_thirty_days_after_creation() {
  let f = fixture();
  let secret = f.store.create_session(&f.token).unwrap();
  let days: f64 = f
    .store
    .with_conn(|c| {
      Ok(c.query_row(
        "SELECT julianday(expires_at) - julianday(created_at) FROM sessions WHERE secret_hash = ?1",
        [hash_token(&secret)],
        |r| r.get(0),
      )?)
    })
    .unwrap();
  assert!((days - 30.0).abs() < 0.01, "{days}");
  assert_eq!(spec_store::store::sessions::SESSION_LIFETIME_DAYS, 30);
}
