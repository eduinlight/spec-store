use diesel::pg::PgConnection;
use diesel::prelude::*;

use crate::model::{Caller, StoreError, StoreResult};
use crate::schema::{iso_utc, users};
use crate::store::Store;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct User {
  pub id: i64,
  pub handle: String,
  pub email: String,
  pub name: String,
  pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Identity {
  pub subject: String,
  pub handle: String,
  pub email: String,
  pub name: String,
}

pub fn user_id_by_handle(conn: &mut PgConnection, handle: &str) -> StoreResult<i64> {
  users::table
    .filter(users::handle.eq(handle))
    .select(users::id)
    .first(conn)
    .optional()?
    .ok_or_else(|| StoreError::NotFound(format!("user {handle:?}")))
}

type UserRow = (i64, String, String, String, String);

fn to_user((id, handle, email, name, created_at): UserRow) -> User {
  User {
    id,
    handle,
    email,
    name,
    created_at,
  }
}

impl Store {
  pub fn upsert_user_from_identity(&self, identity: &Identity) -> StoreResult<User> {
    if identity.subject.is_empty() || identity.handle.is_empty() {
      return Err(StoreError::Validation(
        "an identity needs a subject and a username".into(),
      ));
    }
    let email = identity.email.to_lowercase();
    self.with_tx(|tx| {
      let taken: i64 = users::table
        .filter(users::handle.eq(&identity.handle))
        .filter(users::identity_subject.ne(&identity.subject))
        .count()
        .get_result(tx)?;
      if taken > 0 {
        return Err(StoreError::Validation(format!(
          "username {:?} already belongs to another account",
          identity.handle
        )));
      }
      let row: UserRow = diesel::insert_into(users::table)
        .values((
          users::identity_subject.eq(&identity.subject),
          users::handle.eq(&identity.handle),
          users::email.eq(&email),
          users::name.eq(&identity.name),
        ))
        .on_conflict(users::identity_subject)
        .do_update()
        .set((
          users::email.eq(&email),
          users::name.eq(&identity.name),
          users::updated_at.eq(diesel::dsl::now),
        ))
        .returning((
          users::id,
          users::handle,
          users::email,
          users::name,
          iso_utc(users::created_at),
        ))
        .get_result(tx)?;
      Ok(to_user(row))
    })
  }

  pub fn user_by_subject(&self, subject: &str) -> StoreResult<User> {
    self.with_conn(|conn| {
      users::table
        .filter(users::identity_subject.eq(subject))
        .select((
          users::id,
          users::handle,
          users::email,
          users::name,
          iso_utc(users::created_at),
        ))
        .first::<UserRow>(conn)
        .optional()?
        .map(to_user)
        .ok_or_else(|| StoreError::NotFound(format!("user with subject {subject:?}")))
    })
  }

  pub fn caller_for(&self, identity: &Identity, is_operator: bool) -> StoreResult<Caller> {
    let user = self.upsert_user_from_identity(identity)?;
    Ok(Caller {
      user_id: user.id,
      handle: user.handle,
      is_operator,
    })
  }

  pub fn caller_by_handle(&self, handle: &str, is_operator: bool) -> StoreResult<Caller> {
    self.with_conn(|conn| {
      Ok(Caller {
        user_id: user_id_by_handle(conn, handle)?,
        handle: handle.to_string(),
        is_operator,
      })
    })
  }
}
