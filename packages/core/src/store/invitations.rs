use diesel::pg::PgConnection;
use diesel::prelude::*;
use serde_json::json;

use crate::model::{Caller, OrgRole, StoreError, StoreResult};
use crate::schema::{invitations, iso_utc, organizations, users};
use crate::secrets::{hash_token, random_secret};
use crate::store::organizations::{admit_member, owned_organization};
use crate::store::outbox::enqueue;
use crate::store::refusals::Attempt;
use crate::store::{Store, audit_in};

pub const INVITATION_DAYS: i32 = 7;
pub const INVITATION_EMAIL: &str = "invitation";
const TOKEN_PREFIX: &str = "inv_";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvitationState {
  Pending,
  Accepted,
  Declined,
  Revoked,
  Expired,
}

impl InvitationState {
  pub fn as_str(self) -> &'static str {
    match self {
      InvitationState::Pending => "pending",
      InvitationState::Accepted => "accepted",
      InvitationState::Declined => "declined",
      InvitationState::Revoked => "revoked",
      InvitationState::Expired => "expired",
    }
  }

  fn from_row(state: &str, expired: bool) -> InvitationState {
    match state {
      "accepted" => InvitationState::Accepted,
      "declined" => InvitationState::Declined,
      "revoked" => InvitationState::Revoked,
      _ if expired => InvitationState::Expired,
      _ => InvitationState::Pending,
    }
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewInvitation {
  pub id: i64,
  pub email: String,
  pub token: String,
  pub expires_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvitationInfo {
  pub id: i64,
  pub email: String,
  pub state: InvitationState,
  pub inviter: String,
  pub expires_at: String,
  pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvitationPreview {
  pub id: i64,
  pub organization: String,
  pub organization_name: String,
  pub inviter: String,
  pub email: String,
  pub state: InvitationState,
  pub expires_at: String,
}

pub fn validate_email(email: &str) -> StoreResult<String> {
  let email = email.trim().to_lowercase();
  let invalid = || StoreError::Validation(format!("invalid email address {email:?}"));
  let (local, domain) = email.split_once('@').ok_or_else(invalid)?;
  if local.is_empty()
    || domain.is_empty()
    || !domain.contains('.')
    || domain.starts_with('.')
    || domain.ends_with('.')
    || domain.contains('@')
    || email.len() > 254
    || email.chars().any(|c| c.is_whitespace() || c.is_control())
  {
    return Err(invalid());
  }
  Ok(email)
}

type PreviewRow = (
  i64,
  String,
  String,
  String,
  String,
  String,
  bool,
  String,
  i64,
  String,
);

fn load_preview(
  conn: &mut PgConnection,
  filter: InvitationFilter<'_>,
) -> StoreResult<Option<(InvitationPreview, i64)>> {
  let mut query = invitations::table
    .inner_join(organizations::table)
    .inner_join(users::table.on(users::id.eq(invitations::inviter_id)))
    .select((
      invitations::id,
      organizations::slug,
      organizations::name,
      users::handle,
      invitations::email,
      invitations::state,
      invitations::expires_at.lt(diesel::dsl::now),
      iso_utc(invitations::expires_at),
      invitations::organization_id,
      invitations::language,
    ))
    .into_boxed();
  query = match filter {
    InvitationFilter::Token(token) => query.filter(invitations::token_hash.eq(hash_token(token))),
    InvitationFilter::Id(id) => query.filter(invitations::id.eq(id)),
  };
  let row: Option<PreviewRow> = query.first(conn).optional()?;
  Ok(row.map(
    |(
      id,
      organization,
      organization_name,
      inviter,
      email,
      state,
      expired,
      expires_at,
      organization_id,
      _language,
    )| {
      (
        InvitationPreview {
          id,
          organization,
          organization_name,
          inviter,
          email,
          state: InvitationState::from_row(&state, expired),
          expires_at,
        },
        organization_id,
      )
    },
  ))
}

#[derive(Debug, Clone, Copy)]
enum InvitationFilter<'a> {
  Token(&'a str),
  Id(i64),
}

fn caller_email(conn: &mut PgConnection, caller: &Caller) -> StoreResult<String> {
  Ok(
    users::table
      .filter(users::id.eq(caller.user_id))
      .select(users::email)
      .first(conn)?,
  )
}

struct InvitationEmail<'a> {
  organization_id: i64,
  email: &'a str,
  language: &'a str,
  token: &'a str,
  expires_at: &'a str,
  accept_base_url: &'a str,
}

fn queue_email(
  conn: &mut PgConnection,
  actor: &Caller,
  message: InvitationEmail<'_>,
) -> StoreResult<()> {
  let InvitationEmail {
    organization_id,
    email,
    language,
    token,
    expires_at,
    accept_base_url,
  } = message;
  let (slug, name): (String, String) = organizations::table
    .filter(organizations::id.eq(organization_id))
    .select((organizations::slug, organizations::name))
    .first(conn)?;
  let inviter_name: String = users::table
    .filter(users::id.eq(actor.user_id))
    .select(users::name)
    .first(conn)?;
  let payload = json!({
    "organization": slug,
    "organization_name": name,
    "inviter": actor.handle,
    "inviter_name": inviter_name,
    "accept_url": format!("{}/{token}", accept_base_url.trim_end_matches('/')),
    "expires_at": expires_at,
  });
  enqueue(
    conn,
    INVITATION_EMAIL,
    email,
    language,
    &payload.to_string(),
  )?;
  Ok(())
}

fn refusal(message: &str) -> StoreError {
  StoreError::Validation(message.to_string())
}

fn check_usable(preview: &InvitationPreview) -> StoreResult<()> {
  match preview.state {
    InvitationState::Pending => Ok(()),
    InvitationState::Expired => Err(refusal(
      "this invitation expired; ask the organization owner to invite you again",
    )),
    InvitationState::Revoked => Err(refusal(
      "this invitation was revoked by the organization owner",
    )),
    InvitationState::Accepted | InvitationState::Declined => {
      Err(refusal("this invitation was already used"))
    }
  }
}

impl Store {
  pub fn create_invitation(
    &self,
    actor: &Caller,
    organization: &str,
    email: &str,
    language: &str,
    accept_base_url: &str,
  ) -> StoreResult<NewInvitation> {
    let attempt = Attempt::organization("invitation.create", organization, email);
    self.audited(actor, attempt, |tx| {
      let organization_id = owned_organization(tx, actor, organization)?;
      let email = validate_email(email)?;
      let pending: i64 = invitations::table
        .filter(invitations::organization_id.eq(organization_id))
        .filter(invitations::email.eq(&email))
        .filter(invitations::state.eq("pending"))
        .filter(invitations::expires_at.gt(diesel::dsl::now))
        .count()
        .get_result(tx)?;
      if pending > 0 {
        return Err(StoreError::Validation(format!(
          "{email} already has a pending invitation to {organization:?}; resend it instead"
        )));
      }
      let token = random_secret(TOKEN_PREFIX);
      let (id, expires_at): (i64, String) = diesel::insert_into(invitations::table)
        .values((
          invitations::organization_id.eq(organization_id),
          invitations::email.eq(&email),
          invitations::token_hash.eq(hash_token(&token)),
          invitations::inviter_id.eq(actor.user_id),
          invitations::language.eq(language),
          invitations::expires_at.eq(diesel::dsl::sql::<diesel::sql_types::Timestamptz>(
            &format!("now() + interval '{INVITATION_DAYS} days'"),
          )),
        ))
        .returning((invitations::id, iso_utc(invitations::expires_at)))
        .get_result(tx)?;
      queue_email(
        tx,
        actor,
        InvitationEmail {
          organization_id,
          email: &email,
          language,
          token: &token,
          expires_at: &expires_at,
          accept_base_url,
        },
      )?;
      audit_in(
        tx,
        actor,
        Some(organization_id),
        None,
        "invitation.create",
        &format!("{email}#{id}"),
      )?;
      Ok(NewInvitation {
        id,
        email,
        token,
        expires_at,
      })
    })
  }

  pub fn list_invitations(
    &self,
    actor: &Caller,
    organization: &str,
  ) -> StoreResult<Vec<InvitationInfo>> {
    self.with_conn(|conn| {
      let organization_id = owned_organization(conn, actor, organization)?;
      let rows: Vec<(i64, String, String, bool, String, String, String)> = invitations::table
        .inner_join(users::table.on(users::id.eq(invitations::inviter_id)))
        .filter(invitations::organization_id.eq(organization_id))
        .select((
          invitations::id,
          invitations::email,
          invitations::state,
          invitations::expires_at.lt(diesel::dsl::now),
          users::handle,
          iso_utc(invitations::expires_at),
          iso_utc(invitations::created_at),
        ))
        .order(invitations::id.desc())
        .load(conn)?;
      Ok(
        rows
          .into_iter()
          .map(
            |(id, email, state, expired, inviter, expires_at, created_at)| InvitationInfo {
              id,
              email,
              state: InvitationState::from_row(&state, expired),
              inviter,
              expires_at,
              created_at,
            },
          )
          .collect(),
      )
    })
  }

  pub fn resend_invitation(
    &self,
    actor: &Caller,
    organization: &str,
    id: i64,
    accept_base_url: &str,
  ) -> StoreResult<NewInvitation> {
    let attempt = Attempt::organization("invitation.resend", organization, format!("#{id}"));
    self.audited(actor, attempt, |tx| {
      let organization_id = owned_organization(tx, actor, organization)?;
      let (preview, found_org) = load_preview(tx, InvitationFilter::Id(id))?
        .filter(|(_, org)| *org == organization_id)
        .ok_or_else(|| StoreError::NotFound(format!("invitation {id}")))?;
      check_usable(&preview)?;
      let language: String = invitations::table
        .filter(invitations::id.eq(id))
        .select(invitations::language)
        .first(tx)?;
      let token = random_secret(TOKEN_PREFIX);
      diesel::update(invitations::table.filter(invitations::id.eq(id)))
        .set((
          invitations::token_hash.eq(hash_token(&token)),
          invitations::updated_at.eq(diesel::dsl::now),
        ))
        .execute(tx)?;
      queue_email(
        tx,
        actor,
        InvitationEmail {
          organization_id: found_org,
          email: &preview.email,
          language: &language,
          token: &token,
          expires_at: &preview.expires_at,
          accept_base_url,
        },
      )?;
      audit_in(
        tx,
        actor,
        Some(organization_id),
        None,
        "invitation.resend",
        &format!("{}#{id}", preview.email),
      )?;
      Ok(NewInvitation {
        id,
        email: preview.email,
        token,
        expires_at: preview.expires_at,
      })
    })
  }

  pub fn revoke_invitation(&self, actor: &Caller, organization: &str, id: i64) -> StoreResult<()> {
    let attempt = Attempt::organization("invitation.revoke", organization, format!("#{id}"));
    self.audited(actor, attempt, |tx| {
      let organization_id = owned_organization(tx, actor, organization)?;
      let (preview, _) = load_preview(tx, InvitationFilter::Id(id))?
        .filter(|(_, org)| *org == organization_id)
        .ok_or_else(|| StoreError::NotFound(format!("invitation {id}")))?;
      check_usable(&preview)?;
      diesel::update(invitations::table.filter(invitations::id.eq(id)))
        .set((
          invitations::state.eq("revoked"),
          invitations::updated_at.eq(diesel::dsl::now),
        ))
        .execute(tx)?;
      audit_in(
        tx,
        actor,
        Some(organization_id),
        None,
        "invitation.revoke",
        &format!("{}#{id}", preview.email),
      )
    })
  }

  pub fn invitation_preview(&self, token: &str) -> StoreResult<InvitationPreview> {
    self.with_conn(|conn| {
      load_preview(conn, InvitationFilter::Token(token))?
        .map(|(preview, _)| preview)
        .ok_or_else(|| StoreError::NotFound("invitation".into()))
    })
  }

  pub fn my_invitations(&self, caller: &Caller) -> StoreResult<Vec<InvitationPreview>> {
    self.with_conn(|conn| {
      let email = caller_email(conn, caller)?;
      let ids: Vec<i64> = invitations::table
        .filter(invitations::email.eq(&email))
        .filter(invitations::state.eq("pending"))
        .filter(invitations::expires_at.gt(diesel::dsl::now))
        .select(invitations::id)
        .order(invitations::id)
        .load(conn)?;
      let mut previews = Vec::new();
      for id in ids {
        if let Some((preview, _)) = load_preview(conn, InvitationFilter::Id(id))? {
          previews.push(preview);
        }
      }
      Ok(previews)
    })
  }

  pub fn accept_invitation(
    &self,
    caller: &Caller,
    token: &str,
    email_verified: bool,
  ) -> StoreResult<()> {
    self.with_tx(|tx| {
      respond(
        tx,
        caller,
        InvitationFilter::Token(token),
        Some(email_verified),
      )
    })
  }

  pub fn accept_pending_invitation(
    &self,
    caller: &Caller,
    id: i64,
    email_verified: bool,
  ) -> StoreResult<()> {
    self.with_tx(|tx| respond(tx, caller, InvitationFilter::Id(id), Some(email_verified)))
  }

  pub fn decline_invitation(&self, caller: &Caller, token: &str) -> StoreResult<()> {
    self.with_tx(|tx| respond(tx, caller, InvitationFilter::Token(token), None))
  }

  pub fn decline_pending_invitation(&self, caller: &Caller, id: i64) -> StoreResult<()> {
    self.with_tx(|tx| respond(tx, caller, InvitationFilter::Id(id), None))
  }
}

fn respond(
  conn: &mut PgConnection,
  caller: &Caller,
  filter: InvitationFilter<'_>,
  accept_with_verified_email: Option<bool>,
) -> StoreResult<()> {
  let (preview, organization_id) =
    load_preview(conn, filter)?.ok_or_else(|| StoreError::NotFound("invitation".into()))?;
  invitations::table
    .filter(invitations::id.eq(preview.id))
    .select(invitations::id)
    .for_update()
    .first::<i64>(conn)?;
  let (preview, _) = load_preview(conn, InvitationFilter::Id(preview.id))?
    .ok_or_else(|| StoreError::NotFound("invitation".into()))?;
  check_usable(&preview)?;
  let email = caller_email(conn, caller)?;
  if let Some(verified) = accept_with_verified_email
    && !verified
  {
    return Err(refusal(
      "your email address must be verified before you accept an invitation",
    ));
  }
  if email != preview.email {
    return Err(refusal("this invitation was sent to another email address"));
  }
  let (state, action) = match accept_with_verified_email {
    Some(_) => ("accepted", "invitation.accept"),
    None => ("declined", "invitation.decline"),
  };
  if state == "accepted" {
    admit_member(
      conn,
      &preview.organization,
      caller.user_id,
      OrgRole::Member,
      &caller.via,
    )?;
  }
  diesel::update(invitations::table.filter(invitations::id.eq(preview.id)))
    .set((
      invitations::state.eq(state),
      invitations::accepted_by.eq(Some(caller.user_id)),
      invitations::updated_at.eq(diesel::dsl::now),
    ))
    .execute(conn)?;
  audit_in(
    conn,
    caller,
    Some(organization_id),
    None,
    action,
    &format!("{}#{}", preview.email, preview.id),
  )
}
