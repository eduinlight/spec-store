mod common;

use sdd_vault_core::model::{DocumentKind, Role, StoreError};

#[test]
fn the_same_project_slug_lives_in_two_organizations_without_crossing() {
  let store = common::test_store();
  let alice = common::person(&store, "alice");
  let bob = common::person(&store, "bob");
  common::organization(&store, &alice, "acme");
  common::organization(&store, &bob, "beta");
  store
    .create_project(&alice, "acme/web", "Acme web")
    .unwrap();
  store.create_project(&bob, "beta/web", "Beta web").unwrap();
  store.create_spec(&alice, "acme/web", "first").unwrap();
  store
    .put_document(
      &alice,
      "acme/web",
      1,
      DocumentKind::Spec,
      "acme secret",
      "m",
      None,
    )
    .unwrap();

  assert_eq!(store.list_projects(&alice).unwrap().len(), 1);
  assert_eq!(store.list_projects(&alice).unwrap()[0].slug, "acme/web");
  assert_eq!(store.list_projects(&bob).unwrap()[0].slug, "beta/web");
  assert!(matches!(
    store.get_document(&bob, "acme/web", 1, DocumentKind::Spec),
    Err(StoreError::NotFound(_))
  ));
  assert!(matches!(
    store.list_specs(&bob, "acme/web"),
    Err(StoreError::NotFound(_))
  ));
  assert!(store.list_specs(&bob, "beta/web").unwrap().is_empty());
  assert!(matches!(
    store.create_spec(&bob, "acme/web", "intrusion"),
    Err(StoreError::NotFound(_))
  ));
}

#[test]
fn project_roles_require_organization_membership() {
  let store = common::test_store();
  let alice = common::person(&store, "alice");
  common::person(&store, "bob");
  common::organization(&store, &alice, "acme");
  store.create_project(&alice, "acme/web", "Web").unwrap();
  assert!(matches!(
    store.set_member(&alice, "acme/web", "bob", Role::Reader),
    Err(StoreError::Validation(_))
  ));
}

#[test]
fn outsiders_get_the_same_not_found_for_organizations() {
  let store = common::test_store();
  let alice = common::person(&store, "alice");
  let bob = common::person(&store, "bob");
  common::organization(&store, &alice, "acme");
  let existing = store.organization(&bob, "acme").unwrap_err();
  let missing = store.organization(&bob, "nothing").unwrap_err();
  assert!(matches!(existing, StoreError::NotFound(_)));
  assert!(matches!(missing, StoreError::NotFound(_)));
}

#[test]
fn operators_read_every_organization_but_never_write() {
  let store = common::test_store();
  let alice = common::person(&store, "alice");
  let operator = common::operator(&store, "ops");
  common::organization(&store, &alice, "acme");
  store.create_project(&alice, "acme/web", "Web").unwrap();
  store.create_spec(&alice, "acme/web", "first").unwrap();
  assert_eq!(store.organization(&operator, "acme").unwrap().slug, "acme");
  assert_eq!(store.list_specs(&operator, "acme/web").unwrap().len(), 1);
  assert!(matches!(
    store.create_spec(&operator, "acme/web", "x"),
    Err(StoreError::Forbidden)
  ));
  assert!(matches!(
    store.put_document(&operator, "acme/web", 1, DocumentKind::Plan, "x", "m", None),
    Err(StoreError::Forbidden)
  ));
  assert!(matches!(
    store.create_project(&operator, "acme/other", "Other"),
    Err(StoreError::Forbidden)
  ));
}
