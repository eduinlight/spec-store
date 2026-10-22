mod common;

use spec_store_core::model::{Caller, OrgRole, StoreError};
use spec_store_core::store::invitations::{InvitationState, NewInvitation};

const ACCEPT_BASE: &str = "https://app.example.test/invitations";

struct Fixture {
  store: common::TestStore,
  owner: Caller,
  member: Caller,
  invitee: Caller,
}

fn fixture() -> Fixture {
  let store = common::test_store();
  let owner = common::person(&store, "owner");
  common::organization(&store, &owner, "acme");
  let member = common::person(&store, "member");
  common::join(&store, &owner, "acme", &member);
  Fixture {
    invitee: common::person(&store, "invitee"),
    owner,
    member,
    store,
  }
}

fn invite(f: &Fixture, email: &str) -> NewInvitation {
  f.store
    .create_invitation(&f.owner, "acme", email, "es", ACCEPT_BASE)
    .expect("invitation")
}

#[test]
fn creating_records_a_pending_invitation_for_seven_days_and_queues_an_email() {
  let f = fixture();
  let created = invite(&f, "Invitee@Example.com");
  assert_eq!(created.email, "invitee@example.com");
  assert!(!created.token.is_empty());
  let days = common::real(
    &f.store,
    &format!(
      "SELECT (EXTRACT(EPOCH FROM (expires_at - created_at)) / 86400)::float8 FROM invitations WHERE id = {}",
      created.id
    ),
  );
  assert!((days - 7.0).abs() < 0.001, "{days}");
  let listed = f.store.list_invitations(&f.owner, "acme").unwrap();
  assert_eq!(listed.len(), 1);
  assert_eq!(listed[0].state, InvitationState::Pending);
  assert_eq!(listed[0].inviter, "owner");
  let stored_hash = common::text(&f.store, "SELECT token_hash FROM invitations");
  assert_ne!(
    stored_hash, created.token,
    "only a hash of the token is stored"
  );
  let outbox = common::text(&f.store, "SELECT payload FROM email_outbox");
  assert!(
    outbox.contains(&format!("{ACCEPT_BASE}/{}", created.token)),
    "{outbox}"
  );
  assert!(outbox.contains("\"organization\":\"acme\""), "{outbox}");
  assert!(outbox.contains("\"inviter\":\"owner\""), "{outbox}");
  assert_eq!(
    common::text(&f.store, "SELECT language FROM email_outbox"),
    "es"
  );
  assert_eq!(
    common::text(&f.store, "SELECT recipient FROM email_outbox"),
    "invitee@example.com"
  );
}

#[test]
fn only_owners_invite_and_emails_are_validated() {
  let f = fixture();
  assert!(matches!(
    f.store
      .create_invitation(&f.member, "acme", "x@example.com", "en", ACCEPT_BASE),
    Err(StoreError::Forbidden)
  ));
  assert!(matches!(
    f.store
      .create_invitation(&f.invitee, "acme", "x@example.com", "en", ACCEPT_BASE),
    Err(StoreError::NotFound(_))
  ));
  for bad in ["", "no-at-sign", "two words@example.com", "a@"] {
    assert!(
      matches!(
        f.store
          .create_invitation(&f.owner, "acme", bad, "en", ACCEPT_BASE),
        Err(StoreError::Validation(_))
      ),
      "{bad:?}"
    );
  }
  invite(&f, "x@example.com");
  assert!(matches!(
    f.store
      .create_invitation(&f.owner, "acme", "x@example.com", "en", ACCEPT_BASE),
    Err(StoreError::Validation(_))
  ));
}

#[test]
fn accepting_with_the_invited_email_admits_the_person() {
  let f = fixture();
  let created = invite(&f, "invitee@example.com");
  let preview = f.store.invitation_preview(&created.token).unwrap();
  assert_eq!(preview.organization, "acme");
  assert_eq!(preview.inviter, "owner");
  assert_eq!(preview.state, InvitationState::Pending);
  let pending = f.store.my_invitations(&f.invitee).unwrap();
  assert_eq!(pending.len(), 1);
  assert_eq!(pending[0].organization, "acme");
  f.store
    .accept_invitation(&f.invitee, &created.token, true)
    .unwrap();
  let orgs = f.store.list_organizations(&f.invitee).unwrap();
  assert_eq!(orgs[0].slug, "acme");
  assert_eq!(orgs[0].role, Some(OrgRole::Member));
  assert!(f.store.my_invitations(&f.invitee).unwrap().is_empty());
  assert_eq!(
    f.store.list_invitations(&f.owner, "acme").unwrap()[0].state,
    InvitationState::Accepted
  );
}

fn refused(result: Result<(), StoreError>, cause: &str) {
  match result {
    Err(StoreError::Validation(message)) => assert!(message.contains(cause), "{message}"),
    other => panic!("expected refusal naming {cause:?}, got {other:?}"),
  }
}

#[test]
fn expired_revoked_used_and_mismatched_invitations_are_refused_naming_the_cause() {
  let f = fixture();
  let expired = invite(&f, "invitee@example.com");
  common::exec(
    &f.store,
    &format!(
      "UPDATE invitations SET expires_at = now() - interval '1 second' WHERE id = {}",
      expired.id
    ),
  );
  refused(
    f.store.accept_invitation(&f.invitee, &expired.token, true),
    "expired",
  );
  assert_eq!(
    f.store.list_invitations(&f.owner, "acme").unwrap()[0].state,
    InvitationState::Expired
  );

  let other_org_owner = common::person(&f.store, "other");
  common::organization(&f.store, &other_org_owner, "beta");
  let revoked = f
    .store
    .create_invitation(
      &other_org_owner,
      "beta",
      "invitee@example.com",
      "en",
      ACCEPT_BASE,
    )
    .unwrap();
  f.store
    .revoke_invitation(&other_org_owner, "beta", revoked.id)
    .unwrap();
  refused(
    f.store.accept_invitation(&f.invitee, &revoked.token, true),
    "revoked",
  );

  let third_owner = common::person(&f.store, "third");
  common::organization(&f.store, &third_owner, "gamma");
  let used = f
    .store
    .create_invitation(
      &third_owner,
      "gamma",
      "invitee@example.com",
      "en",
      ACCEPT_BASE,
    )
    .unwrap();
  f.store
    .accept_invitation(&f.invitee, &used.token, true)
    .unwrap();
  refused(
    f.store.accept_invitation(&f.invitee, &used.token, true),
    "already",
  );

  let fourth_owner = common::person(&f.store, "fourth");
  common::organization(&f.store, &fourth_owner, "delta");
  let mismatched = f
    .store
    .create_invitation(
      &fourth_owner,
      "delta",
      "someone-else@example.com",
      "en",
      ACCEPT_BASE,
    )
    .unwrap();
  refused(
    f.store
      .accept_invitation(&f.invitee, &mismatched.token, true),
    "email",
  );
  let unverified = f
    .store
    .create_invitation(
      &fourth_owner,
      "delta",
      "invitee@example.com",
      "en",
      ACCEPT_BASE,
    )
    .unwrap();
  refused(
    f.store
      .accept_invitation(&f.invitee, &unverified.token, false),
    "verified",
  );

  assert!(matches!(
    f.store.accept_invitation(&f.invitee, "no-such-token", true),
    Err(StoreError::NotFound(_))
  ));
}

#[test]
fn resending_keeps_the_expiry_and_issues_a_new_link() {
  let f = fixture();
  let created = invite(&f, "invitee@example.com");
  let expiry_before = common::text(&f.store, "SELECT iso_utc(expires_at) FROM invitations");
  let resent = f
    .store
    .resend_invitation(&f.owner, "acme", created.id, ACCEPT_BASE)
    .unwrap();
  assert_ne!(resent.token, created.token);
  assert_eq!(
    common::text(&f.store, "SELECT iso_utc(expires_at) FROM invitations"),
    expiry_before
  );
  assert_eq!(
    common::int(&f.store, "SELECT count(*) FROM email_outbox"),
    2
  );
  assert!(matches!(
    f.store.accept_invitation(&f.invitee, &created.token, true),
    Err(StoreError::NotFound(_))
  ));
  f.store
    .accept_invitation(&f.invitee, &resent.token, true)
    .unwrap();
  assert!(matches!(
    f.store
      .resend_invitation(&f.owner, "acme", created.id, ACCEPT_BASE),
    Err(StoreError::Validation(_))
  ));
}

#[test]
fn declining_closes_the_invitation_without_admitting() {
  let f = fixture();
  let created = invite(&f, "invitee@example.com");
  f.store
    .decline_invitation(&f.invitee, &created.token)
    .unwrap();
  assert!(f.store.list_organizations(&f.invitee).unwrap().is_empty());
  assert_eq!(
    f.store.list_invitations(&f.owner, "acme").unwrap()[0].state,
    InvitationState::Declined
  );
  refused(
    f.store.accept_invitation(&f.invitee, &created.token, true),
    "already",
  );
}

#[test]
fn invitation_changes_are_audited() {
  let f = fixture();
  let created = invite(&f, "invitee@example.com");
  let resent = f
    .store
    .resend_invitation(&f.owner, "acme", created.id, ACCEPT_BASE)
    .unwrap();
  f.store
    .accept_invitation(&f.invitee, &resent.token, true)
    .unwrap();
  let actions = common::texts(
    &f.store,
    "SELECT action AS value FROM audit_log WHERE action LIKE 'invitation.%' ORDER BY id",
  );
  assert_eq!(
    actions,
    vec![
      "invitation.create",
      "invitation.resend",
      "invitation.accept"
    ]
  );
  let targets = common::texts(
    &f.store,
    "SELECT target AS value FROM audit_log WHERE action LIKE 'invitation.%' ORDER BY id",
  );
  assert!(
    targets.iter().all(|t| !t.contains(&resent.token)),
    "{targets:?}"
  );
}
