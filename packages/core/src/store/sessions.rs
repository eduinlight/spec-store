use diesel::prelude::*;
use diesel::sql_types::{Nullable, Timestamptz};

use crate::model::{Caller, StoreError, StoreResult, Via};
use crate::schema::{browser_sessions, login_attempts, users};
use crate::secrets::{hash_token, random_secret};
use crate::store::Store;

pub const LOGIN_ATTEMPT_SECONDS: i64 = 600;
pub const REFRESH_LEASE_SECONDS: i64 = 15;
const SESSION_PREFIX: &str = "ses_";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewSession {
  pub user_id: i64,
  pub identity_session_id: Option<String>,
  pub refresh_token_sealed: Vec<u8>,
  pub id_token_sealed: Vec<u8>,
  pub access_expires_in_seconds: i64,
  pub is_operator: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrowserSession {
  pub id: i64,
  pub caller: Caller,
  pub identity_session_id: Option<String>,
  pub refresh_token_sealed: Vec<u8>,
  pub id_token_sealed: Vec<u8>,
  pub access_expired: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoginAttempt {
  pub pkce_verifier_sealed: Vec<u8>,
  pub return_to: String,
}

fn expiry_in(seconds: i64) -> diesel::expression::SqlLiteral<Timestamptz> {
  diesel::dsl::sql::<Timestamptz>(&format!("now() + interval '{} seconds'", seconds.max(0)))
}

type SessionRow = (
  i64,
  i64,
  String,
  bool,
  Option<String>,
  Vec<u8>,
  Vec<u8>,
  bool,
);

impl Store {
  pub fn create_session(&self, session: &NewSession) -> StoreResult<String> {
    let secret = random_secret(SESSION_PREFIX);
    self.with_conn(|conn| {
      diesel::insert_into(browser_sessions::table)
        .values((
          browser_sessions::secret_hash.eq(hash_token(&secret)),
          browser_sessions::user_id.eq(session.user_id),
          browser_sessions::identity_session_id.eq(&session.identity_session_id),
          browser_sessions::refresh_token_sealed.eq(&session.refresh_token_sealed),
          browser_sessions::id_token_sealed.eq(&session.id_token_sealed),
          browser_sessions::access_expires_at.eq(expiry_in(session.access_expires_in_seconds)),
          browser_sessions::is_operator.eq(session.is_operator),
        ))
        .execute(conn)?;
      Ok(())
    })?;
    Ok(secret)
  }

  pub fn session(&self, secret: &str) -> StoreResult<BrowserSession> {
    self.with_conn(|conn| {
      let row: Option<SessionRow> = browser_sessions::table
        .inner_join(users::table)
        .filter(browser_sessions::secret_hash.eq(hash_token(secret)))
        .filter(browser_sessions::ended_at.is_null())
        .select((
          browser_sessions::id,
          users::id,
          users::handle,
          browser_sessions::is_operator,
          browser_sessions::identity_session_id,
          browser_sessions::refresh_token_sealed,
          browser_sessions::id_token_sealed,
          browser_sessions::access_expires_at.le(diesel::dsl::now),
        ))
        .first(conn)
        .optional()?;
      let (
        id,
        user_id,
        handle,
        is_operator,
        identity_session_id,
        refresh_token_sealed,
        id_token_sealed,
        access_expired,
      ) = row.ok_or(StoreError::Unauthenticated)?;
      Ok(BrowserSession {
        id,
        caller: Caller {
          user_id,
          handle,
          is_operator,
          via: Via::default(),
        },
        identity_session_id,
        refresh_token_sealed,
        id_token_sealed,
        access_expired,
      })
    })
  }

  pub fn refresh_session(
    &self,
    id: i64,
    refresh_token_sealed: &[u8],
    id_token_sealed: Option<&[u8]>,
    access_expires_in_seconds: i64,
  ) -> StoreResult<()> {
    self.with_conn(|conn| {
      diesel::update(browser_sessions::table.filter(browser_sessions::id.eq(id)))
        .set((
          browser_sessions::refresh_token_sealed.eq(refresh_token_sealed),
          browser_sessions::access_expires_at.eq(expiry_in(access_expires_in_seconds)),
        ))
        .execute(conn)?;
      if let Some(id_token_sealed) = id_token_sealed {
        diesel::update(browser_sessions::table.filter(browser_sessions::id.eq(id)))
          .set(browser_sessions::id_token_sealed.eq(id_token_sealed))
          .execute(conn)?;
      }
      Ok(())
    })
  }

  pub fn claim_session_refresh(&self, id: i64) -> StoreResult<bool> {
    self.with_conn(|conn| {
      Ok(
        diesel::update(
          browser_sessions::table
            .filter(browser_sessions::id.eq(id))
            .filter(browser_sessions::access_expires_at.le(diesel::dsl::now))
            .filter(browser_sessions::ended_at.is_null()),
        )
        .set(browser_sessions::access_expires_at.eq(expiry_in(REFRESH_LEASE_SECONDS)))
        .execute(conn)?
          == 1,
      )
    })
  }

  pub fn end_session(&self, secret: &str) -> StoreResult<()> {
    self.with_conn(|conn| {
      diesel::update(
        browser_sessions::table
          .filter(browser_sessions::secret_hash.eq(hash_token(secret)))
          .filter(browser_sessions::ended_at.is_null()),
      )
      .set(browser_sessions::ended_at.eq(diesel::dsl::sql::<Nullable<Timestamptz>>("now()")))
      .execute(conn)?;
      Ok(())
    })
  }

  pub fn end_session_by_id(&self, id: i64) -> StoreResult<()> {
    self.with_conn(|conn| {
      diesel::update(
        browser_sessions::table
          .filter(browser_sessions::id.eq(id))
          .filter(browser_sessions::ended_at.is_null()),
      )
      .set(browser_sessions::ended_at.eq(diesel::dsl::sql::<Nullable<Timestamptz>>("now()")))
      .execute(conn)?;
      Ok(())
    })
  }

  pub fn end_identity_session(&self, identity_session_id: &str) -> StoreResult<usize> {
    self.with_conn(|conn| {
      Ok(
        diesel::update(
          browser_sessions::table
            .filter(browser_sessions::identity_session_id.eq(identity_session_id))
            .filter(browser_sessions::ended_at.is_null()),
        )
        .set(browser_sessions::ended_at.eq(diesel::dsl::sql::<Nullable<Timestamptz>>("now()")))
        .execute(conn)?,
      )
    })
  }

  pub fn begin_login(
    &self,
    state: &str,
    pkce_verifier_sealed: &[u8],
    return_to: &str,
  ) -> StoreResult<()> {
    self.with_conn(|conn| {
      diesel::delete(login_attempts::table.filter(login_attempts::created_at.lt(
        diesel::dsl::sql::<Timestamptz>(&format!(
          "now() - interval '{LOGIN_ATTEMPT_SECONDS} seconds'"
        )),
      )))
      .execute(conn)?;
      diesel::insert_into(login_attempts::table)
        .values((
          login_attempts::state_hash.eq(hash_token(state)),
          login_attempts::pkce_verifier_sealed.eq(pkce_verifier_sealed),
          login_attempts::return_to.eq(return_to),
        ))
        .execute(conn)?;
      Ok(())
    })
  }

  pub fn finish_login(&self, state: &str) -> StoreResult<LoginAttempt> {
    self.with_conn(|tx| {
      let row: Option<(Vec<u8>, String, bool)> = diesel::delete(
        login_attempts::table.filter(login_attempts::state_hash.eq(hash_token(state))),
      )
      .returning((
        login_attempts::pkce_verifier_sealed,
        login_attempts::return_to,
        diesel::dsl::sql::<diesel::sql_types::Bool>(&format!(
          "created_at > now() - interval '{LOGIN_ATTEMPT_SECONDS} seconds'"
        )),
      ))
      .get_result(tx)
      .optional()?;
      match row {
        Some((pkce_verifier_sealed, return_to, true)) => Ok(LoginAttempt {
          pkce_verifier_sealed,
          return_to,
        }),
        _ => Err(StoreError::Unauthenticated),
      }
    })
  }
}
