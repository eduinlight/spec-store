mod common;

use sdd_vault_core::model::{Caller, DocumentKind, Role, StoreError};

struct Fixture {
  store: common::TestStore,
  root: Caller,
  owner: Caller,
  editor: Caller,
  reader: Caller,
  guest: Caller,
}

fn fixture() -> Fixture {
  let store = common::test_store();
  let root = common::person(&store, "root");
  common::organization(&store, &root, common::ORG);
  let people: Vec<Caller> = ["owner", "editor", "reader"]
    .iter()
    .map(|h| {
      let caller = common::person(&store, h);
      common::join(&store, &root, common::ORG, &caller);
      caller
    })
    .collect();
  store
    .create_project(&root, "acme/demo", "Demo project")
    .unwrap();
  for (handle, role) in [
    ("owner", Role::Owner),
    ("editor", Role::Editor),
    ("reader", Role::Reader),
  ] {
    store.set_member(&root, "acme/demo", handle, role).unwrap();
  }
  let guest = common::person(&store, "guest");
  common::exec(
    &store,
    "INSERT INTO memberships (project_id, user_id, role)
     SELECT p.id, u.id, 'reader' FROM projects p, users u WHERE p.slug = 'demo' AND u.handle = 'guest'",
  );
  Fixture {
    owner: people[0].clone(),
    editor: people[1].clone(),
    reader: people[2].clone(),
    root,
    guest,
    store,
  }
}

#[test]
fn migration_adds_project_invitation_columns_and_membership_creation_time() {
  let f = fixture();
  let created = common::int(
    &f.store,
    "SELECT count(*) FROM information_schema.columns WHERE table_name = 'memberships' AND column_name = 'created_at' AND is_nullable = 'NO'",
  );
  assert_eq!(created, 1);
  let columns = common::int(
    &f.store,
    "SELECT count(*) FROM information_schema.columns WHERE table_name = 'invitations' AND column_name IN ('project_id', 'project_role')",
  );
  assert_eq!(columns, 2);
}

#[test]
fn project_role_is_set_if_and_only_if_the_project_is_set() {
  let f = fixture();
  let project = common::int(&f.store, "SELECT id FROM projects WHERE slug = 'demo'");
  let insert = |extra_columns: &str, extra_values: &str| {
    f.store.with_tx(|c| {
      use diesel::connection::SimpleConnection;
      c.batch_execute(&format!(
        "INSERT INTO invitations (organization_id, email, token_hash, inviter_id, expires_at{extra_columns})
         VALUES ((SELECT id FROM organizations LIMIT 1), 'x@example.com', md5(random()::text), (SELECT id FROM users LIMIT 1), now(){extra_values})"
      ))?;
      Ok(())
    })
  };
  assert!(insert(", project_id", &format!(", {project}")).is_err());
  assert!(insert(", project_role", ", 'reader'").is_err());
  assert!(
    insert(
      ", project_id, project_role",
      &format!(", {project}, 'admin'")
    )
    .is_err()
  );
  assert!(
    insert(
      ", project_id, project_role",
      &format!(", {project}, 'editor'")
    )
    .is_ok()
  );
  assert!(insert("", "").is_ok());
}

#[test]
fn members_have_joined_guest_flag_and_owners_come_first() {
  let f = fixture();
  let members = f.store.project_members(&f.reader, "acme/demo").unwrap();
  let listed: Vec<(&str, Role, bool)> = members
    .iter()
    .map(|m| (m.handle.as_str(), m.role, m.guest))
    .collect();
  assert_eq!(
    listed,
    vec![
      ("owner", Role::Owner, false),
      ("editor", Role::Editor, false),
      ("guest", Role::Reader, true),
      ("reader", Role::Reader, false),
    ]
  );
  assert_eq!(members[1].name, "editor");
  assert!(members.iter().all(|m| m.joined_at.ends_with('Z')));
  assert!(members.iter().all(|m| m.last_active_at.is_none()));
}

#[test]
fn last_active_is_the_latest_audit_entry_of_that_person_in_the_project() {
  let f = fixture();
  common::exec(
    &f.store,
    "INSERT INTO specs (project_id, number, slug, status, branch)
     VALUES ((SELECT id FROM projects WHERE slug = 'demo'), 1, 'first', 'draft', '001-first')",
  );
  let revision = f
    .store
    .put_document(
      &f.editor,
      "acme/demo",
      1,
      DocumentKind::Spec,
      "# One\n",
      "first",
      None,
    )
    .unwrap();
  let first = common::text(
    &f.store,
    "SELECT iso_utc(max(created_at)) FROM audit_log WHERE action = 'document.put'",
  );
  f.store
    .set_repository_url(&f.owner, "acme/demo", Some("https://example.com/repo"))
    .unwrap();
  let members = f.store.project_members(&f.guest, "acme/demo").unwrap();
  let editor = members.iter().find(|m| m.handle == "editor").unwrap();
  assert_eq!(editor.last_active_at.as_deref(), Some(first.as_str()));
  let owner = members.iter().find(|m| m.handle == "owner").unwrap();
  assert!(owner.last_active_at.is_some());
  let reader = members.iter().find(|m| m.handle == "reader").unwrap();
  assert!(reader.last_active_at.is_none());
  f.store
    .put_document(
      &f.editor,
      "acme/demo",
      1,
      DocumentKind::Spec,
      "# Two\n",
      "second",
      Some(revision),
    )
    .unwrap();
  let later = f.store.project_members(&f.guest, "acme/demo").unwrap();
  let editor_later = later.iter().find(|m| m.handle == "editor").unwrap();
  assert!(editor_later.last_active_at.as_deref() > Some(first.as_str()));
}

#[test]
fn membership_joined_time_is_the_membership_creation_time() {
  let f = fixture();
  let joined = common::text(
    &f.store,
    "SELECT iso_utc(created_at) FROM memberships m WHERE user_id = (SELECT id FROM users WHERE handle = 'editor')",
  );
  let members = f.store.project_members(&f.reader, "acme/demo").unwrap();
  assert_eq!(
    members
      .iter()
      .find(|m| m.handle == "editor")
      .unwrap()
      .joined_at,
    joined
  );
}

#[test]
fn only_people_with_access_read_the_members() {
  let f = fixture();
  let outsider = common::person(&f.store, "outsider");
  assert!(matches!(
    f.store.project_members(&outsider, "acme/demo"),
    Err(StoreError::NotFound(_))
  ));
  assert_eq!(
    f.store.project_members(&f.root, "acme/demo").unwrap().len(),
    4
  );
}

#[test]
fn the_last_owner_cannot_be_demoted_or_removed() {
  let f = fixture();
  for result in [
    f.store
      .set_member(&f.root, "acme/demo", "owner", Role::Editor),
    f.store.remove_member(&f.root, "acme/demo", "owner"),
  ] {
    match result {
      Err(StoreError::Validation(message)) => {
        assert!(message.contains("without an owner"), "{message}")
      }
      other => panic!("expected the last-owner rule, got {other:?}"),
    }
  }
  f.store
    .set_member(&f.owner, "acme/demo", "owner", Role::Owner)
    .unwrap();
  f.store
    .set_member(&f.root, "acme/demo", "editor", Role::Owner)
    .unwrap();
  f.store
    .set_member(&f.root, "acme/demo", "owner", Role::Reader)
    .unwrap();
  f.store
    .remove_member(&f.root, "acme/demo", "editor")
    .unwrap_err();
  f.store
    .set_member(&f.root, "acme/demo", "owner", Role::Owner)
    .unwrap();
  f.store
    .remove_member(&f.root, "acme/demo", "editor")
    .unwrap();
}

#[test]
fn guests_keep_their_role_changes_and_removal_by_owners() {
  let f = fixture();
  f.store
    .set_member(&f.owner, "acme/demo", "guest", Role::Editor)
    .unwrap();
  assert!(f.store.access(&f.guest, "acme/demo").unwrap().can_write());
  f.store
    .remove_member(&f.owner, "acme/demo", "guest")
    .unwrap();
  assert!(matches!(
    f.store.access(&f.guest, "acme/demo"),
    Err(StoreError::NotFound(_))
  ));
  assert!(matches!(
    f.store
      .set_member(&f.owner, "acme/demo", "guest", Role::Reader),
    Err(StoreError::Validation(_))
  ));
}
