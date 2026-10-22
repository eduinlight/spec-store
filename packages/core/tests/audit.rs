mod common;

use spec_store_core::model::{DocumentKind, OrgRole, Role, StoreError};

#[test]
fn every_change_is_audited_with_organization_and_project_and_visible_to_the_right_people() {
  let store = common::test_store();
  let owner = common::person(&store, "owner");
  let invitee = common::person(&store, "invitee");
  let outsider = common::person(&store, "outsider");
  let operator = common::operator(&store, "ops");
  common::organization(&store, &owner, "acme");
  let invitation = store
    .create_invitation(
      &owner,
      "acme",
      "invitee@example.com",
      "en",
      "https://app/invitations",
    )
    .unwrap();
  store
    .accept_invitation(&invitee, &invitation.token, true)
    .unwrap();
  store
    .set_organization_member(&owner, "acme", "invitee", OrgRole::Member)
    .unwrap();
  store.create_project(&owner, "acme/demo", "Demo").unwrap();
  store
    .set_member(&owner, "acme/demo", "invitee", Role::Editor)
    .unwrap();
  store.create_spec(&invitee, "acme/demo", "first").unwrap();
  store
    .put_document(
      &invitee,
      "acme/demo",
      1,
      DocumentKind::Spec,
      "# S",
      "m",
      None,
    )
    .unwrap();
  let other = common::person(&store, "other");
  common::organization(&store, &other, "beta");

  let page = store
    .list_organization_audit(&owner, "acme", None, None, 50)
    .unwrap();
  let actions: Vec<&str> = page
    .entries
    .iter()
    .rev()
    .map(|e| e.action.as_str())
    .collect();
  assert_eq!(
    actions,
    vec![
      "organization.create",
      "invitation.create",
      "organization.member.admit",
      "invitation.accept",
      "organization.member.set",
      "project.create",
      "member.set",
      "spec.create",
      "document.put",
    ]
  );
  assert!(
    page
      .entries
      .iter()
      .all(|e| e.organization.as_deref() == Some("acme"))
  );
  for entry in &page.entries {
    let project_scoped = [
      "project.create",
      "member.set",
      "spec.create",
      "document.put",
    ]
    .contains(&entry.action.as_str());
    assert_eq!(
      entry.project.as_deref(),
      project_scoped.then_some("acme/demo"),
      "{entry:?}"
    );
  }
  let only_project = store
    .list_organization_audit(&owner, "acme", Some("demo"), None, 50)
    .unwrap();
  assert_eq!(only_project.entries.len(), 4);

  assert!(matches!(
    store.list_organization_audit(&invitee, "acme", None, None, 50),
    Err(StoreError::Forbidden)
  ));
  assert!(matches!(
    store.list_organization_audit(&outsider, "acme", None, None, 50),
    Err(StoreError::NotFound(_))
  ));
  assert_eq!(
    store
      .list_organization_audit(&operator, "acme", None, None, 50)
      .unwrap()
      .entries
      .len(),
    9
  );

  let global = store.list_global_audit(&operator, None, None, 100).unwrap();
  let orgs: Vec<Option<&str>> = global
    .entries
    .iter()
    .map(|e| e.organization.as_deref())
    .collect();
  assert!(orgs.contains(&Some("acme")) && orgs.contains(&Some("beta")));
  let filtered = store
    .list_global_audit(&operator, Some("beta"), None, 100)
    .unwrap();
  assert!(
    filtered
      .entries
      .iter()
      .all(|e| e.organization.as_deref() == Some("beta"))
  );
  assert_eq!(filtered.entries.len(), 1);
  assert!(matches!(
    store.list_global_audit(&owner, None, None, 10),
    Err(StoreError::Forbidden)
  ));
}

#[test]
fn organization_audit_is_paged_newest_first() {
  let store = common::test_store();
  let owner = common::person(&store, "owner");
  common::organization(&store, &owner, "acme");
  store.create_project(&owner, "acme/demo", "Demo").unwrap();
  for i in 0..5 {
    store
      .create_spec(&owner, "acme/demo", &format!("spec-{i}"))
      .unwrap();
  }
  let first = store
    .list_organization_audit(&owner, "acme", None, None, 3)
    .unwrap();
  assert_eq!(first.entries.len(), 3);
  assert!(first.has_more);
  assert_eq!(first.entries[0].target, "005-spec-4");
  let rest = store
    .list_organization_audit(&owner, "acme", None, first.entries.last().map(|e| e.id), 10)
    .unwrap();
  assert!(!rest.has_more);
  assert_eq!(first.entries.len() + rest.entries.len(), 7);
}
