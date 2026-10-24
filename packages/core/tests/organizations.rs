mod common;

use sdd_vault_core::model::{Caller, OrgRole, Role, StoreError};

struct Fixture {
  store: common::TestStore,
  owner: Caller,
  member: Caller,
  outsider: Caller,
  operator: Caller,
}

fn fixture() -> Fixture {
  let store = common::test_store();
  let owner = common::person(&store, "owner");
  common::organization(&store, &owner, "acme");
  let member = common::person(&store, "member");
  common::join(&store, &owner, "acme", &member);
  Fixture {
    outsider: common::person(&store, "outsider"),
    operator: common::operator(&store, "ops"),
    owner,
    member,
    store,
  }
}

#[test]
fn creating_an_organization_makes_the_creator_its_owner() {
  let f = fixture();
  let created = f
    .store
    .create_organization(&f.outsider, "beta", "Beta Inc")
    .unwrap();
  assert_eq!(created.slug, "beta");
  assert_eq!(created.name, "Beta Inc");
  let mine = f.store.list_organizations(&f.outsider).unwrap();
  assert_eq!(mine.len(), 1);
  assert_eq!(mine[0].slug, "beta");
  assert_eq!(mine[0].role, Some(OrgRole::Owner));
}

#[test]
fn duplicate_and_invalid_slugs_are_rejected() {
  let f = fixture();
  match f
    .store
    .create_organization(&f.outsider, "acme", "Again")
    .unwrap_err()
  {
    StoreError::Validation(message) => assert!(message.contains("acme"), "{message}"),
    other => panic!("expected validation, got {other:?}"),
  }
  assert!(matches!(
    f.store.create_organization(&f.outsider, "Not Valid", "x"),
    Err(StoreError::Validation(_))
  ));
}

#[test]
fn lists_organizations_by_membership_and_operator_sees_all() {
  let f = fixture();
  f.store
    .create_organization(&f.outsider, "beta", "Beta")
    .unwrap();
  let member_view = f.store.list_organizations(&f.member).unwrap();
  assert_eq!(
    member_view
      .iter()
      .map(|o| o.slug.as_str())
      .collect::<Vec<_>>(),
    vec!["acme"]
  );
  assert_eq!(member_view[0].role, Some(OrgRole::Member));
  let operator_view = f.store.list_organizations(&f.operator).unwrap();
  assert_eq!(
    operator_view
      .iter()
      .map(|o| o.slug.as_str())
      .collect::<Vec<_>>(),
    vec!["acme", "beta"]
  );
  assert!(operator_view.iter().all(|o| o.role.is_none()));
}

#[test]
fn members_are_listed_owners_first() {
  let f = fixture();
  let members = f
    .store
    .list_organization_members(&f.member, "acme")
    .unwrap();
  let listed: Vec<(String, OrgRole)> = members.into_iter().map(|m| (m.handle, m.role)).collect();
  assert_eq!(
    listed,
    vec![
      ("owner".to_string(), OrgRole::Owner),
      ("member".to_string(), OrgRole::Member)
    ]
  );
  assert!(matches!(
    f.store.list_organization_members(&f.outsider, "acme"),
    Err(StoreError::NotFound(_))
  ));
  assert_eq!(
    f.store
      .list_organization_members(&f.operator, "acme")
      .unwrap()
      .len(),
    2
  );
}

#[test]
fn owners_change_roles_of_existing_members_only() {
  let f = fixture();
  f.store
    .set_organization_member(&f.owner, "acme", "member", OrgRole::Owner)
    .unwrap();
  assert_eq!(
    f.store.list_organizations(&f.member).unwrap()[0].role,
    Some(OrgRole::Owner)
  );
  assert!(matches!(
    f.store
      .set_organization_member(&f.owner, "acme", "outsider", OrgRole::Member),
    Err(StoreError::NotFound(_))
  ));
  let other = common::person(&f.store, "other");
  common::join(&f.store, &f.owner, "acme", &other);
  assert!(matches!(
    f.store
      .set_organization_member(&other, "acme", "member", OrgRole::Member),
    Err(StoreError::Forbidden)
  ));
  assert!(matches!(
    f.store
      .set_organization_member(&f.operator, "acme", "member", OrgRole::Member),
    Err(StoreError::Forbidden)
  ));
}

#[test]
fn removing_a_member_drops_their_project_roles() {
  let f = fixture();
  f.store
    .create_project(&f.owner, "acme/demo", "Demo")
    .unwrap();
  f.store
    .set_member(&f.owner, "acme/demo", "member", Role::Editor)
    .unwrap();
  assert!(f.store.access(&f.member, "acme/demo").is_ok());
  f.store
    .remove_organization_member(&f.owner, "acme", "member")
    .unwrap();
  assert!(matches!(
    f.store.access(&f.member, "acme/demo"),
    Err(StoreError::NotFound(_))
  ));
  assert_eq!(
    common::int(
      &f.store,
      &format!(
        "SELECT count(*) FROM memberships WHERE user_id = {}",
        f.member.user_id
      )
    ),
    0
  );
  assert!(f.store.list_organizations(&f.member).unwrap().is_empty());
  assert!(matches!(
    f.store
      .remove_organization_member(&f.owner, "acme", "member"),
    Err(StoreError::NotFound(_))
  ));
}

#[test]
fn members_may_leave_but_not_remove_others() {
  let f = fixture();
  let other = common::person(&f.store, "other");
  common::join(&f.store, &f.owner, "acme", &other);
  assert!(matches!(
    f.store
      .remove_organization_member(&f.member, "acme", "other"),
    Err(StoreError::Forbidden)
  ));
  f.store
    .remove_organization_member(&f.member, "acme", "member")
    .unwrap();
  assert!(f.store.list_organizations(&f.member).unwrap().is_empty());
}

#[test]
fn the_last_owner_cannot_leave_step_down_or_be_removed() {
  let f = fixture();
  let refused = |result: Result<(), StoreError>| match result {
    Err(StoreError::Validation(message)) => {
      assert!(message.contains("without an owner"), "{message}")
    }
    other => panic!("expected the last-owner refusal, got {other:?}"),
  };
  refused(
    f.store
      .remove_organization_member(&f.owner, "acme", "owner"),
  );
  refused(
    f.store
      .set_organization_member(&f.owner, "acme", "owner", OrgRole::Member),
  );
  f.store
    .set_organization_member(&f.owner, "acme", "member", OrgRole::Owner)
    .unwrap();
  f.store
    .set_organization_member(&f.owner, "acme", "owner", OrgRole::Member)
    .unwrap();
  refused(
    f.store
      .remove_organization_member(&f.member, "acme", "member"),
  );
}

#[test]
fn organization_changes_are_audited() {
  let f = fixture();
  f.store
    .set_organization_member(&f.owner, "acme", "member", OrgRole::Owner)
    .unwrap();
  f.store
    .remove_organization_member(&f.owner, "acme", "member")
    .unwrap();
  let actions = common::texts(
    &f.store,
    "SELECT action AS value FROM audit_log WHERE organization_id IS NOT NULL ORDER BY id",
  );
  assert_eq!(
    actions,
    vec![
      "organization.create",
      "organization.member.admit",
      "organization.member.set",
      "organization.member.remove"
    ]
  );
}
