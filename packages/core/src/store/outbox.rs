use diesel::pg::PgConnection;
use diesel::prelude::*;
use diesel::sql_types::{BigInt, Bool, Integer, Text};

use crate::model::StoreResult;
use crate::schema::email_outbox;
use crate::store::Store;

pub const RETRY_WINDOW_SECONDS: i64 = 3600;
pub const LEASE_SECONDS: i64 = 120;
pub const MAX_BACKOFF_SECONDS: i64 = 600;

#[derive(Debug, Clone, PartialEq, Eq, QueryableByName)]
pub struct OutboxMessage {
  #[diesel(sql_type = BigInt)]
  pub id: i64,
  #[diesel(sql_type = Text)]
  pub kind: String,
  #[diesel(sql_type = Text)]
  pub recipient: String,
  #[diesel(sql_type = Text)]
  pub language: String,
  #[diesel(sql_type = Text)]
  pub payload: String,
  #[diesel(sql_type = Integer)]
  pub attempts: i32,
}

#[derive(QueryableByName)]
struct Failed {
  #[diesel(sql_type = Bool)]
  failed: bool,
}

pub fn enqueue(
  conn: &mut PgConnection,
  kind: &str,
  recipient: &str,
  language: &str,
  payload: &str,
) -> StoreResult<i64> {
  Ok(
    diesel::insert_into(email_outbox::table)
      .values((
        email_outbox::kind.eq(kind),
        email_outbox::recipient.eq(recipient),
        email_outbox::language.eq(language),
        email_outbox::payload.eq(payload),
      ))
      .returning(email_outbox::id)
      .get_result(conn)?,
  )
}

impl Store {
  pub fn claim_due_messages(&self, limit: i64) -> StoreResult<Vec<OutboxMessage>> {
    self.with_tx(|tx| {
      Ok(
        diesel::sql_query(
          "UPDATE email_outbox
             SET attempts = attempts + 1,
                 next_attempt_at = now() + make_interval(secs => $2)
           WHERE id IN (
             SELECT id FROM email_outbox
              WHERE sent_at IS NULL AND failed_at IS NULL AND next_attempt_at <= now()
              ORDER BY id
              FOR UPDATE SKIP LOCKED
              LIMIT $1)
           RETURNING id, kind, recipient, language, payload, attempts",
        )
        .bind::<BigInt, _>(limit)
        .bind::<BigInt, _>(LEASE_SECONDS)
        .load(tx)?,
      )
    })
  }

  pub fn mark_message_sent(&self, id: i64) -> StoreResult<()> {
    self.with_conn(|conn| {
      diesel::update(email_outbox::table.filter(email_outbox::id.eq(id)))
        .set(email_outbox::sent_at.eq(diesel::dsl::sql::<
          diesel::sql_types::Nullable<diesel::sql_types::Timestamptz>,
        >("now()")))
        .execute(conn)?;
      Ok(())
    })
  }

  pub fn mark_message_attempt_failed(&self, id: i64) -> StoreResult<bool> {
    self.with_conn(|conn| {
      let row: Failed = diesel::sql_query(
        "UPDATE email_outbox
           SET failed_at = CASE WHEN created_at + make_interval(secs => $2) <= now() THEN now() END,
               next_attempt_at = now() + make_interval(secs => least($3, 60 * power(2, greatest(attempts - 1, 0))::bigint))
         WHERE id = $1
         RETURNING failed_at IS NOT NULL AS failed",
      )
      .bind::<BigInt, _>(id)
      .bind::<BigInt, _>(RETRY_WINDOW_SECONDS)
      .bind::<BigInt, _>(MAX_BACKOFF_SECONDS)
      .get_result(conn)?;
      Ok(row.failed)
    })
  }
}
