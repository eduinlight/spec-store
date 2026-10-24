mod common;

use sdd_vault_core::model::{Caller, OrgRole, Role, StoreError};
use sdd_vault_core::store::invitations::ProjectInvitation;

const ACCEPT_BASE: &str = "https://app.example.test/invitations";

struct Fixture {
  store: common::TestStore,
  root: Caller,
  member: Caller,
  guest: Caller,
  operator: Caller,
  stranger: Caller,
}

fn fixture() -> Fixture {
  let store = common::test_store();
  let root = common::person(&store, "root");
  common::organization(&store, &root, common::ORG);
  let member = common::person(&store, "member");
  common::join(&store, &root, common::ORG, &member);
  store.create_project(&root, "acme/demo", "Demo").unwrap();
  store.create_project(&root, "acme/other", "Other").unwrap();
  store
    .set_member(&root, "acme/demo", "root", Role::Owner)
    .unwrap();
  let guest = common::person(&store, "guest");
  let created = store
    .invite_to_project(
      &root,
      "acme/demo",
      "guest@example.com",
      Role::Editor,
      "en",
      ACCEPT_BASE,
    )
    .unwrap();
  let ProjectInvitation::Invited(created) = created else {
    panic!("expected an invitation");
  };
  store
    .accept_invitation(&guest, &created.token, true)
    .unwrap();
  Fixture {
    operator: common::operator(&store, "ops"),
    stranger: common::person(&store, "stranger"),
    member,
    guest,
    root,
    store,
  }
}

fn not_found<T: std::fmt::Debug>(result: Result<T, StoreError>) {
  assert!(matches!(result, Err(StoreError::NotFound(_))), "{result:?}");
}

#[test]
fn a_guest_sees_the_organization_as_name_and_slug_without_a_role() {
  let f = fixture();
  let listed = f.store.list_organizations(&f.guest).unwrap();
  assert_eq!(listed.len(), 1);
  assert_eq!(listed[0].slug, "acme");
  assert_eq!(listed[0].name, "acme");
  assert_eq!(listed[0].role, None);
  assert!(listed[0].guest);
  let organization = f.store.organization(&f.guest, "acme").unwrap();
  assert_eq!(organization.slug, "acme");

  let members = f.store.list_organizations(&f.member).unwrap();
  assert_eq!(members[0].role, Some(OrgRole::Member));
  assert!(!members[0].guest);
  assert!(f.store.list_organizations(&f.stranger).unwrap().is_empty());
  not_found(f.store.organization(&f.stranger, "acme"));
  let operator_view = f.store.list_organizations(&f.operator).unwrap();
  assert_eq!(operator_view.len(), 1);
  assert!(!operator_view[0].guest);
}

#[test]
fn a_guest_reads_only_the_projects_they_belong_to() {
  let f = fixture();
  let projects = f.store.list_projects(&f.guest).unwrap();
  assert_eq!(projects.len(), 1);
  assert_eq!(projects[0].slug, "acme/demo");
  assert_eq!(projects[0].role, "editor");
  let summaries = f.store.project_summaries(&f.guest, "acme").unwrap();
  assert_eq!(summaries.len(), 1);
  assert_eq!(summaries[0].slug, "demo");
  assert_eq!(summaries[0].role, Some(Role::Editor));
  assert_eq!(f.store.project(&f.guest, "acme/demo").unwrap().name, "Demo");
  let members = f.store.project_members(&f.guest, "acme/demo").unwrap();
  assert_eq!(members.len(), 2);
  not_found(f.store.access(&f.guest, "acme/other"));
  not_found(f.store.project(&f.guest, "acme/other"));
  not_found(f.store.project_members(&f.guest, "acme/other"));
  not_found(f.store.project_summaries(&f.stranger, "acme"));
}

#[test]
fn everything_else_of_the_organization_stays_not_found_for_a_guest() {
  let f = fixture();
  not_found(f.store.list_organization_members(&f.guest, "acme"));
  not_found(f.store.list_invitations(&f.guest, "acme"));
  not_found(
    f.store
      .list_organization_audit(&f.guest, "acme", None, None, 10),
  );
  not_found(
    f.store
      .create_invitation(&f.guest, "acme", "x@example.com", "en", ACCEPT_BASE),
  );
  not_found(f.store.create_project(&f.guest, "acme/mine", "Mine"));
  not_found(
    f.store
      .set_organization_member(&f.guest, "acme", "member", OrgRole::Owner),
  );
  not_found(
    f.store
      .remove_organization_member(&f.guest, "acme", "guest"),
  );
  assert!(matches!(
    f.store.list_global_audit(&f.guest, None, None, 10),
    Err(StoreError::Forbidden)
  ));
}

#[test]
fn a_guest_loses_the_organization_with_their_last_project() {
  let f = fixture();
  f.store
    .remove_member(&f.root, "acme/demo", "guest")
    .unwrap();
  assert!(f.store.list_organizations(&f.guest).unwrap().is_empty());
  assert!(f.store.list_projects(&f.guest).unwrap().is_empty());
  not_found(f.store.organization(&f.guest, "acme"));
  not_found(f.store.project_summaries(&f.guest, "acme"));
}

#[test]
fn a_person_who_is_a_member_of_one_organization_is_a_guest_of_another() {
  let f = fixture();
  common::organization(&f.store, &f.guest, "beta");
  let listed = f.store.list_organizations(&f.guest).unwrap();
  let view: Vec<(&str, Option<OrgRole>, bool)> = listed
    .iter()
    .map(|o| (o.slug.as_str(), o.role, o.guest))
    .collect();
  assert_eq!(
    view,
    vec![("acme", None, true), ("beta", Some(OrgRole::Owner), false)]
  );
}
