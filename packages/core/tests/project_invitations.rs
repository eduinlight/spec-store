mod common;

use sdd_vault_core::model::{Caller, OrgRole, Role, StoreError};
use sdd_vault_core::store::invitations::{InvitationState, NewInvitation, ProjectInvitation};

const ACCEPT_BASE: &str = "https://app.example.test/invitations";
const PROJECT: &str = "acme/demo";

struct Fixture {
  store: common::TestStore,
  root: Caller,
  owner: Caller,
  editor: Caller,
  member: Caller,
  outside: Caller,
}

fn fixture() -> Fixture {
  let store = common::test_store();
  let root = common::person(&store, "root");
  common::organization(&store, &root, common::ORG);
  let people: Vec<Caller> = ["owner", "editor", "member"]
    .iter()
    .map(|h| {
      let caller = common::person(&store, h);
      common::join(&store, &root, common::ORG, &caller);
      caller
    })
    .collect();
  store.create_project(&root, PROJECT, "Demo").unwrap();
  store
    .set_member(&root, PROJECT, "owner", Role::Owner)
    .unwrap();
  store
    .set_member(&root, PROJECT, "editor", Role::Editor)
    .unwrap();
  Fixture {
    outside: common::person(&store, "outside"),
    owner: people[0].clone(),
    editor: people[1].clone(),
    member: people[2].clone(),
    root,
    store,
  }
}

fn invited(result: ProjectInvitation) -> NewInvitation {
  match result {
    ProjectInvitation::Invited(created) => created,
    other => panic!("expected a pending invitation, got {other:?}"),
  }
}

fn invite_outside(f: &Fixture, email: &str, role: Role) -> NewInvitation {
  invited(
    f.store
      .invite_to_project(&f.owner, PROJECT, email, role, "es", ACCEPT_BASE)
      .unwrap(),
  )
}

fn refused<T: std::fmt::Debug>(result: Result<T, StoreError>, cause: &str) {
  match result {
    Err(StoreError::Validation(message)) => assert!(message.contains(cause), "{message}"),
    other => panic!("expected a refusal naming {cause:?}, got {other:?}"),
  }
}

#[test]
fn an_organization_member_gets_the_role_at_once_with_an_audit_entry() {
  let f = fixture();
  let result = f
    .store
    .invite_to_project(
      &f.owner,
      PROJECT,
      "Member@Example.com",
      Role::Editor,
      "en",
      ACCEPT_BASE,
    )
    .unwrap();
  assert_eq!(
    result,
    ProjectInvitation::Added {
      handle: "member".into()
    }
  );
  assert!(f.store.access(&f.member, PROJECT).unwrap().can_write());
  assert_eq!(common::int(&f.store, "SELECT count(*) FROM invitations"), 0);
  assert_eq!(
    common::int(&f.store, "SELECT count(*) FROM email_outbox"),
    0
  );
  assert_eq!(
    common::texts(
      &f.store,
      "SELECT target AS value FROM audit_log WHERE action = 'member.set' AND actor_id = (SELECT id FROM users WHERE handle = 'owner')"
    ),
    vec!["member:editor"]
  );
}

#[test]
fn an_outside_email_gets_a_pending_invitation_for_seven_days_and_an_email() {
  let f = fixture();
  let created = invite_outside(&f, "New.Person@Example.com", Role::Reader);
  assert_eq!(created.email, "new.person@example.com");
  let row = common::text(
    &f.store,
    &format!(
      "SELECT project_role || ':' || (SELECT slug FROM projects WHERE id = project_id) FROM invitations WHERE id = {}",
      created.id
    ),
  );
  assert_eq!(row, "reader:demo");
  let days = common::real(
    &f.store,
    &format!(
      "SELECT (EXTRACT(EPOCH FROM (expires_at - created_at)) / 86400)::float8 FROM invitations WHERE id = {}",
      created.id
    ),
  );
  assert!((days - 7.0).abs() < 0.001, "{days}");
  assert_eq!(
    common::text(&f.store, "SELECT recipient FROM email_outbox"),
    "new.person@example.com"
  );
  assert_eq!(
    common::text(&f.store, "SELECT language FROM email_outbox"),
    "es"
  );
  let payload = common::text(&f.store, "SELECT payload FROM email_outbox");
  assert!(payload.contains("\"project\":\"acme/demo\""), "{payload}");
  assert!(payload.contains("\"project_name\":\"Demo\""), "{payload}");
  assert!(payload.contains("\"inviter\":\"owner\""), "{payload}");
  assert!(payload.contains("\"project_role\":\"reader\""), "{payload}");
  assert!(
    payload.contains(&format!("{ACCEPT_BASE}/{}", created.token)),
    "{payload}"
  );
  let rendered = sdd_vault_core::email::render("invitation", "es", &payload).unwrap();
  assert!(
    rendered.subject.contains("proyecto Demo"),
    "{}",
    rendered.subject
  );
  assert!(rendered.text.contains("owner"));
  assert_eq!(
    common::texts(
      &f.store,
      "SELECT action || ' ' || project_id::text AS value FROM audit_log WHERE action = 'invitation.create'"
    ),
    vec![format!(
      "invitation.create {}",
      common::int(&f.store, "SELECT id FROM projects WHERE slug = 'demo'")
    )]
  );
}

#[test]
fn only_project_owners_invite_and_the_inputs_are_checked() {
  let f = fixture();
  for caller in [&f.editor, &f.member] {
    let result = f.store.invite_to_project(
      caller,
      PROJECT,
      "x@example.com",
      Role::Reader,
      "en",
      ACCEPT_BASE,
    );
    assert!(
      matches!(result, Err(StoreError::Forbidden | StoreError::NotFound(_))),
      "{result:?}"
    );
  }
  assert!(matches!(
    f.store.invite_to_project(
      &f.editor,
      PROJECT,
      "x@example.com",
      Role::Reader,
      "en",
      ACCEPT_BASE
    ),
    Err(StoreError::Forbidden)
  ));
  assert!(matches!(
    f.store.invite_to_project(
      &f.outside,
      PROJECT,
      "x@example.com",
      Role::Reader,
      "en",
      ACCEPT_BASE
    ),
    Err(StoreError::NotFound(_))
  ));
  assert_eq!(
    common::int(
      &f.store,
      "SELECT count(*) FROM audit_log WHERE action = 'invitation.create' AND result = 'denied'"
    ),
    2
  );
  for bad in ["", "no-at-sign", "a@"] {
    refused(
      f.store
        .invite_to_project(&f.owner, PROJECT, bad, Role::Reader, "en", ACCEPT_BASE),
      "invalid email",
    );
  }
  f.store.archive_project(&f.root, PROJECT).unwrap();
  assert!(matches!(
    f.store.invite_to_project(
      &f.owner,
      PROJECT,
      "x@example.com",
      Role::Reader,
      "en",
      ACCEPT_BASE
    ),
    Err(StoreError::Forbidden)
  ));
}

#[test]
fn people_who_already_belong_to_the_project_and_repeats_are_refused() {
  let f = fixture();
  refused(
    f.store.invite_to_project(
      &f.owner,
      PROJECT,
      "editor@example.com",
      Role::Reader,
      "en",
      ACCEPT_BASE,
    ),
    "already a member",
  );
  refused(
    f.store.invite_to_project(
      &f.owner,
      PROJECT,
      "root@example.com",
      Role::Reader,
      "en",
      ACCEPT_BASE,
    ),
    "owns the organization",
  );
  invite_outside(&f, "x@example.com", Role::Reader);
  refused(
    f.store.invite_to_project(
      &f.owner,
      PROJECT,
      "x@example.com",
      Role::Editor,
      "en",
      ACCEPT_BASE,
    ),
    "pending invitation",
  );
  let created = invite_outside(&f, "outside@example.com", Role::Reader);
  f.store
    .accept_invitation(&f.outside, &created.token, true)
    .unwrap();
  refused(
    f.store.invite_to_project(
      &f.owner,
      PROJECT,
      "outside@example.com",
      Role::Reader,
      "en",
      ACCEPT_BASE,
    ),
    "already a member",
  );
}

fn is_organization_member(f: &Fixture, handle: &str) -> bool {
  common::int(
    &f.store,
    &format!(
      "SELECT count(*) FROM organization_members WHERE user_id = (SELECT id FROM users WHERE handle = '{handle}')"
    ),
  ) > 0
}

#[test]
fn accepting_by_token_or_by_id_joins_the_project_as_a_guest() {
  let f = fixture();
  let by_token = invite_outside(&f, "outside@example.com", Role::Editor);
  let preview = f.store.invitation_preview(&by_token.token).unwrap();
  assert_eq!(preview.project.as_deref(), Some("acme/demo"));
  assert_eq!(preview.project_name.as_deref(), Some("Demo"));
  assert_eq!(preview.project_role, Some(Role::Editor));
  assert_eq!(preview.organization, "acme");
  let mine = f.store.my_invitations(&f.outside).unwrap();
  assert_eq!(mine.len(), 1);
  assert_eq!(mine[0].project.as_deref(), Some("acme/demo"));
  f.store
    .accept_invitation(&f.outside, &by_token.token, true)
    .unwrap();
  assert!(!is_organization_member(&f, "outside"));
  let access = f.store.access(&f.outside, PROJECT).unwrap();
  assert_eq!(access.role, Some(Role::Editor));
  assert_eq!(
    common::texts(
      &f.store,
      "SELECT action || ' ' || (project_id IS NOT NULL)::text AS value FROM audit_log WHERE actor_id = (SELECT id FROM users WHERE handle = 'outside')"
    ),
    vec!["invitation.accept true"]
  );
  let members = f.store.project_members(&f.owner, PROJECT).unwrap();
  let guest = members.iter().find(|m| m.handle == "outside").unwrap();
  assert!(guest.guest);
  assert!(guest.last_active_at.is_some());

  let other = common::person(&f.store, "second");
  let by_id = invite_outside(&f, "second@example.com", Role::Reader);
  f.store
    .accept_pending_invitation(&other, by_id.id, true)
    .unwrap();
  assert!(!is_organization_member(&f, "second"));
  assert_eq!(
    f.store.access(&other, PROJECT).unwrap().role,
    Some(Role::Reader)
  );
}

#[test]
fn project_invitations_keep_the_acceptance_rules_of_organization_invitations() {
  let f = fixture();
  let created = invite_outside(&f, "outside@example.com", Role::Reader);
  refused(
    f.store.accept_invitation(&f.member, &created.token, true),
    "another email",
  );
  refused(
    f.store.accept_invitation(&f.outside, &created.token, false),
    "verified",
  );
  f.store
    .decline_invitation(&f.outside, &created.token)
    .unwrap();
  assert!(matches!(
    f.store.access(&f.outside, PROJECT),
    Err(StoreError::NotFound(_))
  ));
  refused(
    f.store.accept_invitation(&f.outside, &created.token, true),
    "already used",
  );
  let expired = invite_outside(&f, "late@example.com", Role::Reader);
  common::exec(
    &f.store,
    &format!(
      "UPDATE invitations SET expires_at = now() - interval '1 second' WHERE id = {}",
      expired.id
    ),
  );
  let late = common::person(&f.store, "late");
  refused(
    f.store.accept_invitation(&late, &expired.token, true),
    "project owner",
  );
}

#[test]
fn owners_list_resend_and_revoke_the_pending_invitations_of_a_project() {
  let f = fixture();
  let created = invite_outside(&f, "outside@example.com", Role::Reader);
  f.store
    .invite_to_project(
      &f.root,
      PROJECT,
      "other@example.com",
      Role::Editor,
      "en",
      ACCEPT_BASE,
    )
    .unwrap();
  let listed = f.store.list_project_invitations(&f.owner, PROJECT).unwrap();
  assert_eq!(listed.len(), 2);
  assert_eq!(listed[1].email, "outside@example.com");
  assert_eq!(listed[1].role, Some(Role::Reader));
  assert_eq!(listed[1].inviter, "owner");
  assert_eq!(listed[1].state, InvitationState::Pending);
  assert_eq!(listed[0].inviter, "root");
  assert!(matches!(
    f.store.list_project_invitations(&f.editor, PROJECT),
    Err(StoreError::Forbidden)
  ));
  assert!(matches!(
    f.store.list_project_invitations(&f.outside, PROJECT),
    Err(StoreError::NotFound(_))
  ));
  assert!(
    f.store
      .list_invitations(&f.root, common::ORG)
      .unwrap()
      .is_empty(),
    "organization invitations list only organization invitations"
  );

  let resent = f
    .store
    .resend_project_invitation(&f.owner, PROJECT, created.id, ACCEPT_BASE)
    .unwrap();
  assert_ne!(resent.token, created.token);
  assert!(matches!(
    f.store.invitation_preview(&created.token),
    Err(StoreError::NotFound(_))
  ));
  let payloads = common::texts(
    &f.store,
    "SELECT payload AS value FROM email_outbox ORDER BY id",
  );
  assert_eq!(payloads.len(), 3);
  assert!(payloads[2].contains("\"project_name\":\"Demo\""));
  assert!(payloads[2].contains(&resent.token));

  assert!(matches!(
    f.store
      .resend_project_invitation(&f.editor, PROJECT, created.id, ACCEPT_BASE),
    Err(StoreError::Forbidden)
  ));
  f.store
    .revoke_project_invitation(&f.owner, PROJECT, created.id)
    .unwrap();
  refused(
    f.store.accept_invitation(&f.outside, &resent.token, true),
    "revoked",
  );
  refused(
    f.store
      .resend_project_invitation(&f.owner, PROJECT, created.id, ACCEPT_BASE),
    "revoked",
  );
  assert_eq!(
    common::texts(
      &f.store,
      "SELECT action AS value FROM audit_log WHERE action LIKE 'invitation.%' AND result = 'ok' ORDER BY id"
    ),
    vec![
      "invitation.create",
      "invitation.create",
      "invitation.resend",
      "invitation.revoke"
    ]
  );
}

#[test]
fn project_and_organization_invitations_do_not_cross_over() {
  let f = fixture();
  let project_invitation = invite_outside(&f, "outside@example.com", Role::Reader);
  let organization_invitation = f
    .store
    .create_invitation(
      &f.root,
      common::ORG,
      "orgwide@example.com",
      "en",
      ACCEPT_BASE,
    )
    .unwrap();
  assert!(matches!(
    f.store
      .resend_invitation(&f.root, common::ORG, project_invitation.id, ACCEPT_BASE),
    Err(StoreError::NotFound(_))
  ));
  assert!(matches!(
    f.store
      .revoke_invitation(&f.root, common::ORG, project_invitation.id),
    Err(StoreError::NotFound(_))
  ));
  assert!(matches!(
    f.store
      .resend_project_invitation(&f.owner, PROJECT, organization_invitation.id, ACCEPT_BASE),
    Err(StoreError::NotFound(_))
  ));
  assert!(matches!(
    f.store
      .revoke_project_invitation(&f.owner, PROJECT, organization_invitation.id),
    Err(StoreError::NotFound(_))
  ));
  let listed = f.store.list_invitations(&f.root, common::ORG).unwrap();
  assert_eq!(listed.len(), 1);
  assert_eq!(listed[0].email, "orgwide@example.com");
  assert_eq!(listed[0].role, None);
  let duplicate = f.store.create_invitation(
    &f.root,
    common::ORG,
    "outside@example.com",
    "en",
    ACCEPT_BASE,
  );
  assert!(
    duplicate.is_ok(),
    "a pending project invitation does not block an organization one"
  );
}

#[test]
fn an_organization_owner_adding_a_guest_keeps_their_project_roles_and_drops_the_flag() {
  let f = fixture();
  let created = invite_outside(&f, "outside@example.com", Role::Editor);
  f.store
    .accept_invitation(&f.outside, &created.token, true)
    .unwrap();
  let joined_before = f
    .store
    .project_members(&f.owner, PROJECT)
    .unwrap()
    .into_iter()
    .find(|m| m.handle == "outside")
    .unwrap();
  assert!(joined_before.guest);

  let organization_invitation = f
    .store
    .create_invitation(
      &f.root,
      common::ORG,
      "outside@example.com",
      "en",
      ACCEPT_BASE,
    )
    .unwrap();
  f.store
    .accept_invitation(&f.outside, &organization_invitation.token, true)
    .unwrap();
  assert!(is_organization_member(&f, "outside"));
  let after = f
    .store
    .project_members(&f.owner, PROJECT)
    .unwrap()
    .into_iter()
    .find(|m| m.handle == "outside")
    .unwrap();
  assert!(!after.guest);
  assert_eq!(after.role, Role::Editor);
  assert_eq!(after.joined_at, joined_before.joined_at);
  assert_eq!(
    f.store.access(&f.outside, PROJECT).unwrap().role,
    Some(Role::Editor)
  );
  let organizations = f.store.list_organizations(&f.outside).unwrap();
  assert_eq!(organizations[0].role, Some(OrgRole::Member));
}
