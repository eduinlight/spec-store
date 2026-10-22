use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;

use crate::auth::{generate_token, hash_token};
use crate::model::{Caller, StoreError, StoreResult, validate_slug};
use crate::schema::{api_tokens, users};
use crate::store::{Store, audit, now_utc};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct User {
  pub id: i64,
  pub handle: String,
  pub is_admin: bool,
  pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenInfo {
  pub id: i64,
  pub label: String,
  pub created_at: String,
  pub last_used_at: Option<String>,
  pub revoked_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IssuedToken {
  pub id: i64,
  pub token: String,
}

pub fn require_admin(actor: Option<&Caller>) -> StoreResult<&Caller> {
  actor.filter(|a| a.is_admin).ok_or(StoreError::Forbidden)
}

pub fn user_id_by_handle(conn: &mut SqliteConnection, handle: &str) -> StoreResult<i64> {
  users::table
    .filter(users::handle.eq(handle))
    .select(users::id)
    .first(conn)
    .optional()?
    .ok_or_else(|| StoreError::NotFound(format!("user {handle:?}")))
}

type CallerRow = (i64, String, i64);

pub(crate) fn caller_from_row((user_id, handle, is_admin): CallerRow) -> Caller {
  Caller {
    user_id,
    handle,
    is_admin: is_admin != 0,
  }
}

type TokenRow = (i64, String, String, Option<String>, Option<String>);

fn tokens_of(conn: &mut SqliteConnection, user_id: i64) -> StoreResult<Vec<TokenInfo>> {
  let rows: Vec<TokenRow> = api_tokens::table
    .filter(api_tokens::user_id.eq(user_id))
    .select((
      api_tokens::id,
      api_tokens::label,
      api_tokens::created_at,
      api_tokens::last_used_at,
      api_tokens::revoked_at,
    ))
    .order(api_tokens::id)
    .load(conn)?;
  Ok(
    rows
      .into_iter()
      .map(
        |(id, label, created_at, last_used_at, revoked_at)| TokenInfo {
          id,
          label,
          created_at,
          last_used_at,
          revoked_at,
        },
      )
      .collect(),
  )
}

impl Store {
  pub fn create_user(
    &self,
    actor: Option<&Caller>,
    handle: &str,
    is_admin: bool,
  ) -> StoreResult<User> {
    validate_slug(handle).map_err(|_| {
      StoreError::Validation(format!(
        "invalid handle {handle:?}: one to four lowercase words of letters and digits joined by single hyphens"
      ))
    })?;
    self.with_tx(|tx| {
      let existing: i64 = users::table.count().get_result(tx)?;
      let bootstrap = existing == 0;
      let actor_id = if bootstrap {
        None
      } else {
        Some(require_admin(actor)?.user_id)
      };
      let duplicate: i64 = users::table
        .filter(users::handle.eq(handle))
        .count()
        .get_result(tx)?;
      if duplicate > 0 {
        return Err(StoreError::Validation(format!(
          "user handle {handle:?} already exists"
        )));
      }
      let admin_flag = bootstrap || is_admin;
      let (id, created_at): (i64, String) = diesel::insert_into(users::table)
        .values((
          users::handle.eq(handle),
          users::is_admin.eq(i64::from(admin_flag)),
        ))
        .returning((users::id, users::created_at))
        .get_result(tx)?;
      audit(tx, actor_id.unwrap_or(id), None, "user.create", handle)?;
      Ok(User {
        id,
        handle: handle.to_string(),
        is_admin: admin_flag,
        created_at,
      })
    })
  }

  pub fn caller_by_handle(&self, handle: &str) -> StoreResult<Caller> {
    self.with_conn(|conn| {
      users::table
        .filter(users::handle.eq(handle))
        .select((users::id, users::handle, users::is_admin))
        .first::<CallerRow>(conn)
        .optional()?
        .map(caller_from_row)
        .ok_or_else(|| StoreError::NotFound(format!("user {handle:?}")))
    })
  }

  pub fn issue_token(
    &self,
    actor: Option<&Caller>,
    handle: &str,
    label: &str,
  ) -> StoreResult<IssuedToken> {
    let admin = require_admin(actor)?;
    self.with_tx(|tx| {
      let user_id = user_id_by_handle(tx, handle)?;
      let token = generate_token();
      let id: i64 = diesel::insert_into(api_tokens::table)
        .values((
          api_tokens::user_id.eq(user_id),
          api_tokens::token_hash.eq(hash_token(&token)),
          api_tokens::label.eq(label),
        ))
        .returning(api_tokens::id)
        .get_result(tx)?;
      audit(
        tx,
        admin.user_id,
        None,
        "token.issue",
        &format!("{handle}#{id}"),
      )?;
      Ok(IssuedToken { id, token })
    })
  }

  pub fn revoke_token(&self, actor: Option<&Caller>, token_id: i64) -> StoreResult<()> {
    let admin = require_admin(actor)?;
    self.with_tx(|tx| {
      let handle: Option<String> = api_tokens::table
        .inner_join(users::table)
        .filter(api_tokens::id.eq(token_id))
        .select(users::handle)
        .first(tx)
        .optional()?;
      let handle = handle.ok_or_else(|| StoreError::NotFound(format!("token {token_id}")))?;
      diesel::update(
        api_tokens::table
          .filter(api_tokens::id.eq(token_id))
          .filter(api_tokens::revoked_at.is_null()),
      )
      .set(api_tokens::revoked_at.eq(now_utc().nullable()))
      .execute(tx)?;
      audit(
        tx,
        admin.user_id,
        None,
        "token.revoke",
        &format!("{handle}#{token_id}"),
      )
    })
  }

  pub fn list_tokens(&self, actor: Option<&Caller>, handle: &str) -> StoreResult<Vec<TokenInfo>> {
    require_admin(actor)?;
    self.with_conn(|conn| {
      let user_id = user_id_by_handle(conn, handle)?;
      tokens_of(conn, user_id)
    })
  }

  pub fn list_own_tokens(&self, caller: &Caller) -> StoreResult<Vec<TokenInfo>> {
    self.with_conn(|conn| tokens_of(conn, caller.user_id))
  }

  pub fn authenticate(&self, token: &str) -> StoreResult<Caller> {
    if token.is_empty() {
      return Err(StoreError::Unauthenticated);
    }
    let hash = hash_token(token);
    self.with_tx(|tx| {
      let found: Option<(i64, CallerRow)> = api_tokens::table
        .inner_join(users::table)
        .filter(api_tokens::token_hash.eq(&hash))
        .filter(api_tokens::revoked_at.is_null())
        .select((api_tokens::id, (users::id, users::handle, users::is_admin)))
        .first(tx)
        .optional()?;
      let (token_id, caller) = found.ok_or(StoreError::Unauthenticated)?;
      diesel::update(api_tokens::table.filter(api_tokens::id.eq(token_id)))
        .set(api_tokens::last_used_at.eq(now_utc().nullable()))
        .execute(tx)?;
      Ok(caller_from_row(caller))
    })
  }
}
