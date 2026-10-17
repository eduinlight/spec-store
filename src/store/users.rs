use rusqlite::{OptionalExtension, params};

use crate::auth::{generate_token, hash_token};
use crate::model::{Caller, StoreError, StoreResult, validate_slug};
use crate::store::{Store, audit};

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

pub fn user_id_by_handle(conn: &rusqlite::Connection, handle: &str) -> StoreResult<i64> {
  conn
    .query_row("SELECT id FROM users WHERE handle = ?1", [handle], |r| {
      r.get(0)
    })
    .optional()?
    .ok_or_else(|| StoreError::NotFound(format!("user {handle:?}")))
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
      let existing: i64 = tx.query_row("SELECT count(*) FROM users", [], |r| r.get(0))?;
      let bootstrap = existing == 0;
      let actor_id = if bootstrap {
        None
      } else {
        Some(require_admin(actor)?.user_id)
      };
      let duplicate: i64 = tx.query_row(
        "SELECT count(*) FROM users WHERE handle = ?1",
        [handle],
        |r| r.get(0),
      )?;
      if duplicate > 0 {
        return Err(StoreError::Validation(format!(
          "user handle {handle:?} already exists"
        )));
      }
      let admin_flag = bootstrap || is_admin;
      tx.execute(
        "INSERT INTO users (handle, is_admin) VALUES (?1, ?2)",
        params![handle, admin_flag],
      )?;
      let id = tx.last_insert_rowid();
      audit(tx, actor_id.unwrap_or(id), None, "user.create", handle)?;
      let created_at: String =
        tx.query_row("SELECT created_at FROM users WHERE id = ?1", [id], |r| {
          r.get(0)
        })?;
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
      conn
        .query_row(
          "SELECT id, handle, is_admin FROM users WHERE handle = ?1",
          [handle],
          |r| {
            Ok(Caller {
              user_id: r.get(0)?,
              handle: r.get(1)?,
              is_admin: r.get(2)?,
            })
          },
        )
        .optional()?
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
      tx.execute(
        "INSERT INTO api_tokens (user_id, token_hash, label) VALUES (?1, ?2, ?3)",
        params![user_id, hash_token(&token), label],
      )?;
      let id = tx.last_insert_rowid();
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
      let handle: Option<String> = tx
        .query_row(
          "SELECT u.handle FROM api_tokens t JOIN users u ON u.id = t.user_id WHERE t.id = ?1",
          [token_id],
          |r| r.get(0),
        )
        .optional()?;
      let handle = handle.ok_or_else(|| StoreError::NotFound(format!("token {token_id}")))?;
      tx.execute(
        "UPDATE api_tokens SET revoked_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE id = ?1 AND revoked_at IS NULL",
        [token_id],
      )?;
      audit(tx, admin.user_id, None, "token.revoke", &format!("{handle}#{token_id}"))
    })
  }

  pub fn list_tokens(&self, actor: Option<&Caller>, handle: &str) -> StoreResult<Vec<TokenInfo>> {
    require_admin(actor)?;
    self.with_conn(|conn| {
      let user_id = user_id_by_handle(conn, handle)?;
      let mut stmt = conn.prepare(
        "SELECT id, label, created_at, last_used_at, revoked_at FROM api_tokens WHERE user_id = ?1 ORDER BY id",
      )?;
      let rows = stmt.query_map([user_id], |r| {
        Ok(TokenInfo {
          id: r.get(0)?,
          label: r.get(1)?,
          created_at: r.get(2)?,
          last_used_at: r.get(3)?,
          revoked_at: r.get(4)?,
        })
      })?;
      rows.collect::<Result<Vec<_>, _>>().map_err(StoreError::from)
    })
  }

  pub fn list_own_tokens(&self, caller: &Caller) -> StoreResult<Vec<TokenInfo>> {
    self.with_conn(|conn| {
      let mut stmt = conn.prepare(
        "SELECT id, label, created_at, last_used_at, revoked_at FROM api_tokens WHERE user_id = ?1 ORDER BY id",
      )?;
      let rows = stmt.query_map([caller.user_id], |r| {
        Ok(TokenInfo {
          id: r.get(0)?,
          label: r.get(1)?,
          created_at: r.get(2)?,
          last_used_at: r.get(3)?,
          revoked_at: r.get(4)?,
        })
      })?;
      rows.collect::<Result<Vec<_>, _>>().map_err(StoreError::from)
    })
  }

  pub fn authenticate(&self, token: &str) -> StoreResult<Caller> {
    if token.is_empty() {
      return Err(StoreError::Unauthenticated);
    }
    let hash = hash_token(token);
    self.with_tx(|tx| {
      let found = tx
        .query_row(
          "SELECT t.id, u.id, u.handle, u.is_admin
           FROM api_tokens t JOIN users u ON u.id = t.user_id
           WHERE t.token_hash = ?1 AND t.revoked_at IS NULL",
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
      tx.execute(
        "UPDATE api_tokens SET last_used_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE id = ?1",
        [token_id],
      )?;
      Ok(caller)
    })
  }
}
