use diesel::pg::PgConnection;
use diesel::prelude::*;
use serde_json::json;

use crate::model::{Caller, OrgRole, Role, StoreError, StoreResult};
use crate::schema::{invitations, iso_utc, memberships, organizations, projects, users};
use crate::secrets::{hash_token, random_secret};
use crate::store::organizations::{admit_member, org_role, owned_organization};
use crate::store::outbox::enqueue;
use crate::store::projects::{access_in, lock_project, role_of};
use crate::store::refusals::Attempt;
use crate::store::{Store, audit, audit_in};

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
  pub role: Option<Role>,
  pub state: InvitationState,
  pub inviter: String,
  pub expires_at: String,
  pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectInvitation {
  Added { handle: String },
  Invited(NewInvitation),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvitationPreview {
  pub id: i64,
  pub organization: String,
  pub organization_name: String,
  pub project: Option<String>,
  pub project_name: Option<String>,
  pub project_role: Option<Role>,
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
  Option<String>,
  Option<String>,
  Option<String>,
  String,
  String,
  String,
  bool,
  String,
  i64,
  Option<i64>,
);

struct Loaded {
  preview: InvitationPreview,
  organization_id: i64,
  project_id: Option<i64>,
}

fn load_preview(
  conn: &mut PgConnection,
  filter: InvitationFilter<'_>,
) -> StoreResult<Option<Loaded>> {
  let mut query = invitations::table
    .inner_join(organizations::table)
    .left_join(projects::table.on(projects::id.nullable().eq(invitations::project_id)))
    .inner_join(users::table.on(users::id.eq(invitations::inviter_id)))
    .select((
      invitations::id,
      organizations::slug,
      organizations::name,
      projects::slug.nullable(),
      projects::name.nullable(),
      invitations::project_role,
      users::handle,
      invitations::email,
      invitations::state,
      invitations::expires_at.lt(diesel::dsl::now),
      iso_utc(invitations::expires_at),
      invitations::organization_id,
      invitations::project_id,
    ))
    .into_boxed();
  query = match filter {
    InvitationFilter::Token(token) => query.filter(invitations::token_hash.eq(hash_token(token))),
    InvitationFilter::Id(id) => query.filter(invitations::id.eq(id)),
  };
  let row: Option<PreviewRow> = query.first(conn).optional()?;
  row
    .map(
      |(
        id,
        organization,
        organization_name,
        project_slug,
        project_name,
        project_role,
        inviter,
        email,
        state,
        expired,
        expires_at,
        organization_id,
        project_id,
      )| {
        Ok(Loaded {
          preview: InvitationPreview {
            id,
            project: project_slug.map(|slug| format!("{organization}/{slug}")),
            organization,
            organization_name,
            project_name,
            project_role: project_role.map(|r| r.parse()).transpose()?,
            inviter,
            email,
            state: InvitationState::from_row(&state, expired),
            expires_at,
          },
          organization_id,
          project_id,
        })
      },
    )
    .transpose()
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
  project: Option<(i64, Role)>,
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
    project,
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
  let mut payload = json!({
    "organization": slug,
    "organization_name": name,
    "inviter": actor.handle,
    "inviter_name": inviter_name,
    "accept_url": format!("{}/{token}", accept_base_url.trim_end_matches('/')),
    "expires_at": expires_at,
  });
  if let Some((project_id, role)) = project {
    let (project_slug, project_name): (String, String) = projects::table
      .filter(projects::id.eq(project_id))
      .select((projects::slug, projects::name))
      .first(conn)?;
    payload["project"] = json!(format!("{slug}/{project_slug}"));
    payload["project_name"] = json!(project_name);
    payload["project_role"] = json!(role.as_str());
  }
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
  let owner = if preview.project.is_some() {
    "project owner"
  } else {
    "organization owner"
  };
  match preview.state {
    InvitationState::Pending => Ok(()),
    InvitationState::Expired => Err(refusal(&format!(
      "this invitation expired; ask the {owner} to invite you again"
    ))),
    InvitationState::Revoked => Err(refusal(&format!(
      "this invitation was revoked by the {owner}"
    ))),
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
        .filter(invitations::project_id.is_null())
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
          project: None,
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
      invitation_infos(conn, InvitationScope::Organization(organization_id))
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
      let loaded = load_preview(tx, InvitationFilter::Id(id))?
        .filter(|l| l.organization_id == organization_id && l.project_id.is_none())
        .ok_or_else(|| StoreError::NotFound(format!("invitation {id}")))?;
      let new = reissue(tx, actor, loaded, accept_base_url)?;
      audit_in(
        tx,
        actor,
        Some(organization_id),
        None,
        "invitation.resend",
        &format!("{}#{id}", new.email),
      )?;
      Ok(new)
    })
  }

  pub fn revoke_invitation(&self, actor: &Caller, organization: &str, id: i64) -> StoreResult<()> {
    let attempt = Attempt::organization("invitation.revoke", organization, format!("#{id}"));
    self.audited(actor, attempt, |tx| {
      let organization_id = owned_organization(tx, actor, organization)?;
      let loaded = load_preview(tx, InvitationFilter::Id(id))?
        .filter(|l| l.organization_id == organization_id && l.project_id.is_none())
        .ok_or_else(|| StoreError::NotFound(format!("invitation {id}")))?;
      let email = close_revoked(tx, loaded)?;
      audit_in(
        tx,
        actor,
        Some(organization_id),
        None,
        "invitation.revoke",
        &format!("{email}#{id}"),
      )
    })
  }

  pub fn invitation_preview(&self, token: &str) -> StoreResult<InvitationPreview> {
    self.with_conn(|conn| {
      load_preview(conn, InvitationFilter::Token(token))?
        .map(|loaded| loaded.preview)
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
        if let Some(loaded) = load_preview(conn, InvitationFilter::Id(id))? {
          previews.push(loaded.preview);
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
  let found =
    load_preview(conn, filter)?.ok_or_else(|| StoreError::NotFound("invitation".into()))?;
  invitations::table
    .filter(invitations::id.eq(found.preview.id))
    .select(invitations::id)
    .for_update()
    .first::<i64>(conn)?;
  let Loaded {
    preview,
    organization_id,
    project_id,
  } = load_preview(conn, InvitationFilter::Id(found.preview.id))?
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
    match (project_id, preview.project_role) {
      (Some(project_id), Some(role)) => {
        diesel::insert_into(memberships::table)
          .values((
            memberships::project_id.eq(project_id),
            memberships::user_id.eq(caller.user_id),
            memberships::role.eq(role.as_str()),
          ))
          .on_conflict_do_nothing()
          .execute(conn)?;
      }
      _ => admit_member(
        conn,
        &preview.organization,
        caller.user_id,
        OrgRole::Member,
        &caller.via,
      )?,
    }
  }
  diesel::update(invitations::table.filter(invitations::id.eq(preview.id)))
    .set((
      invitations::state.eq(state),
      invitations::accepted_by.eq(Some(caller.user_id)),
      invitations::updated_at.eq(diesel::dsl::now),
    ))
    .execute(conn)?;
  let target = format!("{}#{}", preview.email, preview.id);
  match project_id {
    Some(project_id) => audit(conn, caller, Some(project_id), action, &target),
    None => audit_in(conn, caller, Some(organization_id), None, action, &target),
  }
}

#[derive(Debug, Clone, Copy)]
enum InvitationScope {
  Organization(i64),
  Project(i64),
}

type InfoRow = (
  i64,
  String,
  Option<String>,
  String,
  bool,
  String,
  String,
  String,
);

fn invitation_infos(
  conn: &mut PgConnection,
  scope: InvitationScope,
) -> StoreResult<Vec<InvitationInfo>> {
  let mut query = invitations::table
    .inner_join(users::table.on(users::id.eq(invitations::inviter_id)))
    .select((
      invitations::id,
      invitations::email,
      invitations::project_role,
      invitations::state,
      invitations::expires_at.lt(diesel::dsl::now),
      users::handle,
      iso_utc(invitations::expires_at),
      iso_utc(invitations::created_at),
    ))
    .order(invitations::id.desc())
    .into_boxed();
  query = match scope {
    InvitationScope::Organization(id) => query
      .filter(invitations::organization_id.eq(id))
      .filter(invitations::project_id.is_null()),
    InvitationScope::Project(id) => query.filter(invitations::project_id.eq(id)),
  };
  let rows: Vec<InfoRow> = query.load(conn)?;
  rows
    .into_iter()
    .map(
      |(id, email, role, state, expired, inviter, expires_at, created_at)| {
        Ok(InvitationInfo {
          id,
          email,
          role: role.map(|r| r.parse()).transpose()?,
          state: InvitationState::from_row(&state, expired),
          inviter,
          expires_at,
          created_at,
        })
      },
    )
    .collect()
}

fn reissue(
  conn: &mut PgConnection,
  actor: &Caller,
  loaded: Loaded,
  accept_base_url: &str,
) -> StoreResult<NewInvitation> {
  let Loaded {
    preview,
    organization_id,
    project_id,
  } = loaded;
  check_usable(&preview)?;
  let language: String = invitations::table
    .filter(invitations::id.eq(preview.id))
    .select(invitations::language)
    .first(conn)?;
  let token = random_secret(TOKEN_PREFIX);
  diesel::update(invitations::table.filter(invitations::id.eq(preview.id)))
    .set((
      invitations::token_hash.eq(hash_token(&token)),
      invitations::updated_at.eq(diesel::dsl::now),
    ))
    .execute(conn)?;
  queue_email(
    conn,
    actor,
    InvitationEmail {
      organization_id,
      project: project_id.zip(preview.project_role),
      email: &preview.email,
      language: &language,
      token: &token,
      expires_at: &preview.expires_at,
      accept_base_url,
    },
  )?;
  Ok(NewInvitation {
    id: preview.id,
    email: preview.email,
    token,
    expires_at: preview.expires_at,
  })
}

fn close_revoked(conn: &mut PgConnection, loaded: Loaded) -> StoreResult<String> {
  check_usable(&loaded.preview)?;
  diesel::update(invitations::table.filter(invitations::id.eq(loaded.preview.id)))
    .set((
      invitations::state.eq("revoked"),
      invitations::updated_at.eq(diesel::dsl::now),
    ))
    .execute(conn)?;
  Ok(loaded.preview.email)
}

impl Store {
  pub fn invite_to_project(
    &self,
    actor: &Caller,
    project: &str,
    email: &str,
    role: Role,
    language: &str,
    accept_base_url: &str,
  ) -> StoreResult<ProjectInvitation> {
    let attempt = Attempt::project("invitation.create", project, email);
    self.audited(actor, attempt, |tx| {
      let access = access_in(tx, actor, project)?;
      if !access.can_manage_members() {
        return Err(StoreError::Forbidden);
      }
      lock_project(tx, access.project_id)?;
      let email = validate_email(email)?;
      let known: Option<(i64, String)> = users::table
        .filter(users::email.eq(&email))
        .select((users::id, users::handle))
        .first(tx)
        .optional()?;
      if let Some((user_id, handle)) = known {
        let organization_role = org_role(tx, access.organization_id, user_id)?;
        if organization_role == Some(OrgRole::Owner) {
          return Err(StoreError::Validation(format!(
            "{handle:?} owns the organization and already has the owner role in project {project:?}"
          )));
        }
        if role_of(tx, access.project_id, user_id)?.is_some() {
          return Err(StoreError::Validation(format!(
            "{handle:?} is already a member of project {project:?}; change their role instead"
          )));
        }
        if organization_role.is_some() {
          diesel::insert_into(memberships::table)
            .values((
              memberships::project_id.eq(access.project_id),
              memberships::user_id.eq(user_id),
              memberships::role.eq(role.as_str()),
            ))
            .execute(tx)?;
          audit(
            tx,
            actor,
            Some(access.project_id),
            "member.set",
            &format!("{handle}:{role}"),
          )?;
          return Ok(ProjectInvitation::Added { handle });
        }
      }
      let pending: i64 = invitations::table
        .filter(invitations::project_id.eq(access.project_id))
        .filter(invitations::email.eq(&email))
        .filter(invitations::state.eq("pending"))
        .filter(invitations::expires_at.gt(diesel::dsl::now))
        .count()
        .get_result(tx)?;
      if pending > 0 {
        return Err(StoreError::Validation(format!(
          "{email} already has a pending invitation to project {project:?}; resend it instead"
        )));
      }
      let token = random_secret(TOKEN_PREFIX);
      let (id, expires_at): (i64, String) = diesel::insert_into(invitations::table)
        .values((
          invitations::organization_id.eq(access.organization_id),
          invitations::project_id.eq(access.project_id),
          invitations::project_role.eq(role.as_str()),
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
          organization_id: access.organization_id,
          project: Some((access.project_id, role)),
          email: &email,
          language,
          token: &token,
          expires_at: &expires_at,
          accept_base_url,
        },
      )?;
      audit(
        tx,
        actor,
        Some(access.project_id),
        "invitation.create",
        &format!("{email}#{id}"),
      )?;
      Ok(ProjectInvitation::Invited(NewInvitation {
        id,
        email,
        token,
        expires_at,
      }))
    })
  }

  pub fn list_project_invitations(
    &self,
    actor: &Caller,
    project: &str,
  ) -> StoreResult<Vec<InvitationInfo>> {
    self.with_conn(|conn| {
      let access = access_in(conn, actor, project)?;
      if !access.is_owner() {
        return Err(StoreError::Forbidden);
      }
      invitation_infos(conn, InvitationScope::Project(access.project_id))
    })
  }

  pub fn resend_project_invitation(
    &self,
    actor: &Caller,
    project: &str,
    id: i64,
    accept_base_url: &str,
  ) -> StoreResult<NewInvitation> {
    let attempt = Attempt::project("invitation.resend", project, format!("#{id}"));
    self.audited(actor, attempt, |tx| {
      let access = access_in(tx, actor, project)?;
      if !access.can_manage_members() {
        return Err(StoreError::Forbidden);
      }
      let loaded = load_preview(tx, InvitationFilter::Id(id))?
        .filter(|l| l.project_id == Some(access.project_id))
        .ok_or_else(|| StoreError::NotFound(format!("invitation {id}")))?;
      let new = reissue(tx, actor, loaded, accept_base_url)?;
      audit(
        tx,
        actor,
        Some(access.project_id),
        "invitation.resend",
        &format!("{}#{id}", new.email),
      )?;
      Ok(new)
    })
  }

  pub fn revoke_project_invitation(
    &self,
    actor: &Caller,
    project: &str,
    id: i64,
  ) -> StoreResult<()> {
    let attempt = Attempt::project("invitation.revoke", project, format!("#{id}"));
    self.audited(actor, attempt, |tx| {
      let access = access_in(tx, actor, project)?;
      if !access.can_manage_members() {
        return Err(StoreError::Forbidden);
      }
      let loaded = load_preview(tx, InvitationFilter::Id(id))?
        .filter(|l| l.project_id == Some(access.project_id))
        .ok_or_else(|| StoreError::NotFound(format!("invitation {id}")))?;
      let email = close_revoked(tx, loaded)?;
      audit(
        tx,
        actor,
        Some(access.project_id),
        "invitation.revoke",
        &format!("{email}#{id}"),
      )
    })
  }
}
