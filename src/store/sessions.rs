use diesel::dsl::sql;
use diesel::prelude::*;
use diesel::sql_types::Text;
use diesel::sqlite::SqliteConnection;

use crate::auth::{hash_token, random_secret};
use crate::model::{Caller, StoreError, StoreResult};
use crate::schema::{api_tokens, sessions, users};
use crate::store::users::caller_from_row;
use crate::store::{Store, now_utc};

pub const SESSION_PREFIX: &str = "sss_";
pub const SESSION_LIFETIME_DAYS: i64 = 30;

impl Store {
  pub fn create_session(&self, token: &str) -> StoreResult<String> {
    if token.is_empty() {
      return Err(StoreError::Unauthenticated);
    }
    let token_hash = hash_token(token);
    self.with_tx(|tx| {
      let found: Option<(i64, i64)> = api_tokens::table
        .filter(api_tokens::token_hash.eq(&token_hash))
        .filter(api_tokens::revoked_at.is_null())
        .select((api_tokens::id, api_tokens::user_id))
        .first(tx)
        .optional()?;
      let (token_id, user_id) = found.ok_or(StoreError::Unauthenticated)?;
      let secret = random_secret(SESSION_PREFIX);
      diesel::insert_into(sessions::table)
        .values((
          sessions::secret_hash.eq(hash_token(&secret)),
          sessions::token_id.eq(token_id),
          sessions::user_id.eq(user_id),
          sessions::expires_at.eq(sql::<Text>(&format!(
            "strftime('%Y-%m-%dT%H:%M:%fZ', 'now', '+{SESSION_LIFETIME_DAYS} days')"
          ))),
        ))
        .execute(tx)?;
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
      let found: Option<(i64, (i64, String, i64))> = sessions::table
        .inner_join(api_tokens::table)
        .inner_join(users::table)
        .filter(sessions::secret_hash.eq(&hash))
        .filter(sessions::ended_at.is_null())
        .filter(sessions::expires_at.gt(now_utc()))
        .filter(api_tokens::revoked_at.is_null())
        .select((api_tokens::id, (users::id, users::handle, users::is_admin)))
        .first(tx)
        .optional()?;
      let (token_id, caller) = found.ok_or(StoreError::Unauthenticated)?;
      touch_token(tx, token_id)?;
      Ok((caller_from_row(caller), token_id))
    })
  }

  pub fn end_session(&self, secret: &str) -> StoreResult<()> {
    let hash = hash_token(secret);
    self.with_tx(|tx| {
      diesel::update(
        sessions::table
          .filter(sessions::secret_hash.eq(&hash))
          .filter(sessions::ended_at.is_null()),
      )
      .set(sessions::ended_at.eq(now_utc().nullable()))
      .execute(tx)?;
      Ok(())
    })
  }
}

fn touch_token(conn: &mut SqliteConnection, token_id: i64) -> StoreResult<()> {
  diesel::update(api_tokens::table.filter(api_tokens::id.eq(token_id)))
    .set(api_tokens::last_used_at.eq(now_utc().nullable()))
    .execute(conn)?;
  Ok(())
}
