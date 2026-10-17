use rusqlite::{OptionalExtension, params};

use crate::auth::{hash_token, random_secret};
use crate::model::{Caller, StoreError, StoreResult};
use crate::store::Store;

pub const SESSION_PREFIX: &str = "sss_";
pub const SESSION_LIFETIME_DAYS: i64 = 30;

const NOW: &str = "strftime('%Y-%m-%dT%H:%M:%fZ', 'now')";

impl Store {
  pub fn create_session(&self, token: &str) -> StoreResult<String> {
    if token.is_empty() {
      return Err(StoreError::Unauthenticated);
    }
    let token_hash = hash_token(token);
    self.with_tx(|tx| {
      let found: Option<(i64, i64)> = tx
        .query_row(
          "SELECT id, user_id FROM api_tokens WHERE token_hash = ?1 AND revoked_at IS NULL",
          [&token_hash],
          |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
      let (token_id, user_id) = found.ok_or(StoreError::Unauthenticated)?;
      let secret = random_secret(SESSION_PREFIX);
      tx.execute(
        &format!(
          "INSERT INTO sessions (secret_hash, token_id, user_id, expires_at)
           VALUES (?1, ?2, ?3, strftime('%Y-%m-%dT%H:%M:%fZ', 'now', '+{SESSION_LIFETIME_DAYS} days'))"
        ),
        params![hash_token(&secret), token_id, user_id],
      )?;
      touch_token(tx, token_id)?;
      Ok(secret)
    })
  }

  pub fn session_caller(&self, secret: &str) -> StoreResult<(Caller, i64)> {
    if secret.is_empty() {
      return Err(StoreError::Unauthenticated);
    }
    let hash = hash_token(secret);
    self.with_tx(|tx| {
      let found = tx
        .query_row(
          &format!(
            "SELECT t.id, u.id, u.handle, u.is_admin
             FROM sessions s
             JOIN api_tokens t ON t.id = s.token_id
             JOIN users u ON u.id = s.user_id
             WHERE s.secret_hash = ?1
               AND s.ended_at IS NULL
               AND s.expires_at > {NOW}
               AND t.revoked_at IS NULL"
          ),
          [&hash],
          |r| {
            Ok((
              r.get::<_, i64>(0)?,
              Caller {
                user_id: r.get(1)?,
                handle: r.get(2)?,
                is_admin: r.get(3)?,
              },
            ))
          },
        )
        .optional()?;
      let (token_id, caller) = found.ok_or(StoreError::Unauthenticated)?;
      touch_token(tx, token_id)?;
      Ok((caller, token_id))
    })
  }

  pub fn end_session(&self, secret: &str) -> StoreResult<()> {
    let hash = hash_token(secret);
    self.with_tx(|tx| {
      tx.execute(
        &format!(
          "UPDATE sessions SET ended_at = {NOW} WHERE secret_hash = ?1 AND ended_at IS NULL"
        ),
        [&hash],
      )?;
      Ok(())
    })
  }
}

fn touch_token(tx: &rusqlite::Transaction, token_id: i64) -> StoreResult<()> {
  tx.execute(
    &format!("UPDATE api_tokens SET last_used_at = {NOW} WHERE id = ?1"),
    [token_id],
  )?;
  Ok(())
}
