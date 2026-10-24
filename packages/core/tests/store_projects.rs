mod common;

use sdd_vault_core::model::{Caller, Role, StoreError};

struct Fixture {
  store: common::TestStore,
  root: Caller,
  operator: Caller,
  owner: Caller,
  editor: Caller,
  reader: Caller,
  member: Caller,
  outsider: Caller,
}

fn fixture() -> Fixture {
  let store = common::test_store();
  let root = common::person(&store, "root");
  common::organization(&store, &root, common::ORG);
  let people: Vec<Caller> = ["owner", "editor", "reader", "member"]
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
  store
    .set_member(&root, "acme/demo", "owner", Role::Owner)
    .unwrap();
  store
    .set_member(&root, "acme/demo", "editor", Role::Editor)
    .unwrap();
  store
    .set_member(&root, "acme/demo", "reader", Role::Reader)
    .unwrap();
  Fixture {
    operator: common::operator(&store, "ops"),
    outsider: common::person(&store, "outsider"),
    owner: people[0].clone(),
    editor: people[1].clone(),
    reader: people[2].clone(),
    member: people[3].clone(),
    root,
    store,
  }
}

#[test]
fn creates_project_and_rejects_duplicate_slug() {
  let f = fixture();
  let project = f
    .store
    .create_project(&f.root, "acme/second", "Second")
    .unwrap();
  assert_eq!(project.slug, "second");
  assert_eq!(project.address.to_string(), "acme/second");
  match f
    .store
    .create_project(&f.root, "acme/demo", "Again")
    .unwrap_err()
  {
    StoreError::Validation(msg) => assert!(msg.contains("demo"), "{msg}"),
    other => panic!("expected validation, got {other:?}"),
  }
  assert!(matches!(
    f.store.create_project(&f.root, "acme/Bad Slug", "x"),
    Err(StoreError::Validation(_))
  ));
  assert!(matches!(
    f.store.create_project(&f.owner, "acme/third", "x"),
    Err(StoreError::Forbidden)
  ));
  assert!(matches!(
    f.store.create_project(&f.operator, "acme/third", "x"),
    Err(StoreError::Forbidden)
  ));
  assert!(matches!(
    f.store.create_project(&f.outsider, "acme/third", "x"),
    Err(StoreError::NotFound(_))
  ));
}

#[test]
fn set_member_replaces_role_and_remove_ends_access() {
  let f = fixture();
  f.store
    .set_member(&f.owner, "acme/demo", "member", Role::Reader)
    .unwrap();
  f.store
    .set_member(&f.owner, "acme/demo", "member", Role::Editor)
    .unwrap();
  let access = f.store.access(&f.member, "acme/demo").unwrap();
  assert_eq!(access.role, Some(Role::Editor));
  let count: i64 = common::int(
    &f.store,
    &format!(
      "SELECT count(*) FROM memberships WHERE user_id = {}",
      f.member.user_id
    ),
  );
  assert_eq!(count, 1);

  f.store
    .remove_member(&f.owner, "acme/demo", "member")
    .unwrap();
  assert!(matches!(
    f.store.access(&f.member, "acme/demo"),
    Err(StoreError::NotFound(_))
  ));

  assert!(matches!(
    f.store
      .set_member(&f.editor, "acme/demo", "member", Role::Reader),
    Err(StoreError::Forbidden)
  ));
  assert!(matches!(
    f.store
      .set_member(&f.owner, "acme/demo", "nobody", Role::Reader),
    Err(StoreError::NotFound(_))
  ));
  assert!(matches!(
    f.store.remove_member(&f.owner, "acme/demo", "nobody"),
    Err(StoreError::NotFound(_))
  ));
  assert!(matches!(
    f.store
      .set_member(&f.root, "acme/missing", "member", Role::Reader),
    Err(StoreError::NotFound(_))
  ));
}

#[test]
fn project_roles_go_only_to_organization_members() {
  let f = fixture();
  match f
    .store
    .set_member(&f.owner, "acme/demo", "outsider", Role::Reader)
    .unwrap_err()
  {
    StoreError::Validation(msg) => {
      assert!(msg.contains("not a member of the organization"), "{msg}")
    }
    other => panic!("expected validation, got {other:?}"),
  }
}

#[test]
fn list_projects_by_membership_and_operator_sees_all() {
  let f = fixture();
  f.store
    .create_project(&f.root, "acme/other", "Other")
    .unwrap();

  let mine = f.store.list_projects(&f.editor).unwrap();
  assert_eq!(mine.len(), 1);
  assert_eq!(mine[0].slug, "acme/demo");
  assert_eq!(mine[0].name, "Demo project");
  assert_eq!(mine[0].role, "editor");

  assert!(f.store.list_projects(&f.outsider).unwrap().is_empty());
  assert!(f.store.list_projects(&f.member).unwrap().is_empty());

  let owned = f.store.list_projects(&f.root).unwrap();
  assert!(
    owned.iter().all(|p| p.role == "owner"),
    "organization owners own every project"
  );
  assert_eq!(owned.len(), 2);

  let all = f.store.list_projects(&f.operator).unwrap();
  let slugs: Vec<&str> = all.iter().map(|p| p.slug.as_str()).collect();
  assert_eq!(slugs, vec!["acme/demo", "acme/other"]);
  assert!(all.iter().all(|p| p.role == "operator"));
}

#[test]
fn outsider_gets_the_same_not_found_whether_or_not_the_project_exists() {
  let f = fixture();
  let existing = f.store.access(&f.outsider, "acme/demo").unwrap_err();
  let missing = f.store.access(&f.outsider, "acme/missing").unwrap_err();
  assert!(matches!(existing, StoreError::NotFound(_)));
  assert!(matches!(missing, StoreError::NotFound(_)));
  assert_eq!(
    existing.to_string().replace("demo", "X"),
    missing.to_string().replace("missing", "X")
  );

  let operator_access = f.store.access(&f.operator, "acme/demo").unwrap();
  assert!(operator_access.is_operator);
  assert_eq!(operator_access.role, None);
  assert!(!operator_access.can_write());
  assert!(!operator_access.can_manage_members());
  assert!(matches!(
    f.store.access(&f.operator, "acme/missing"),
    Err(StoreError::NotFound(_))
  ));

  let reader = f.store.access(&f.reader, "acme/demo").unwrap();
  assert!(!reader.can_write());
  assert!(f.store.access(&f.editor, "acme/demo").unwrap().can_write());
  assert!(f.store.access(&f.owner, "acme/demo").unwrap().is_owner());
  assert!(f.store.access(&f.root, "acme/demo").unwrap().is_owner());
  assert!(!f.store.access(&f.editor, "acme/demo").unwrap().is_owner());
}

#[test]
fn malformed_addresses_are_rejected() {
  let f = fixture();
  assert!(matches!(
    f.store.access(&f.owner, "demo"),
    Err(StoreError::Validation(_))
  ));
}

#[test]
fn list_members_orders_owners_first_and_handles_empty() {
  let f = fixture();
  let members = f.store.list_members(&f.reader, "acme/demo").unwrap();
  let listed: Vec<(String, Role)> = members.iter().map(|m| (m.handle.clone(), m.role)).collect();
  assert_eq!(
    listed,
    vec![
      ("owner".to_string(), Role::Owner),
      ("editor".to_string(), Role::Editor),
      ("reader".to_string(), Role::Reader),
    ]
  );
  assert!(matches!(
    f.store.list_members(&f.outsider, "acme/demo"),
    Err(StoreError::NotFound(_))
  ));
  assert_eq!(
    f.store
      .list_members(&f.operator, "acme/demo")
      .unwrap()
      .len(),
    3
  );

  f.store
    .create_project(&f.root, "acme/empty", "Empty")
    .unwrap();
  assert!(
    f.store
      .list_members(&f.root, "acme/empty")
      .unwrap()
      .is_empty()
  );
}

#[test]
fn repository_url_is_validated_audited_and_clearable() {
  let f = fixture();
  assert!(
    f.store
      .project(&f.reader, "acme/demo")
      .unwrap()
      .repository_url
      .is_none()
  );

  f.store
    .set_repository_url(&f.owner, "acme/demo", Some("https://github.com/acme/demo"))
    .unwrap();
  let project = f.store.project(&f.reader, "acme/demo").unwrap();
  assert_eq!(
    project.repository_url.as_deref(),
    Some("https://github.com/acme/demo")
  );
  assert_eq!(project.slug, "demo");
  assert_eq!(project.name, "Demo project");

  f.store
    .set_repository_url(&f.root, "acme/demo", Some("http://git.internal/demo/"))
    .unwrap();
  assert_eq!(
    f.store
      .project(&f.operator, "acme/demo")
      .unwrap()
      .repository_url
      .as_deref(),
    Some("http://git.internal/demo/")
  );

  for bad in [
    "ftp://x/y",
    "github.com/acme/demo",
    "https://",
    "https:// spaced.example/x",
    "javascript:alert(1)",
    "",
  ] {
    assert!(
      matches!(
        f.store.set_repository_url(&f.owner, "acme/demo", Some(bad)),
        Err(StoreError::Validation(_))
      ),
      "{bad:?} must be rejected"
    );
  }
  assert_eq!(
    f.store
      .project(&f.operator, "acme/demo")
      .unwrap()
      .repository_url
      .as_deref(),
    Some("http://git.internal/demo/"),
    "rejected values change nothing"
  );

  assert!(matches!(
    f.store
      .set_repository_url(&f.editor, "acme/demo", Some("https://example.com/r")),
    Err(StoreError::Forbidden)
  ));
  assert!(matches!(
    f.store.set_repository_url(&f.operator, "acme/demo", None),
    Err(StoreError::Forbidden)
  ));
  assert!(matches!(
    f.store.set_repository_url(&f.outsider, "acme/demo", None),
    Err(StoreError::NotFound(_))
  ));
  assert!(matches!(
    f.store.project(&f.outsider, "acme/demo"),
    Err(StoreError::NotFound(_))
  ));
  assert!(matches!(
    f.store.project(&f.operator, "acme/missing"),
    Err(StoreError::NotFound(_))
  ));

  f.store
    .set_repository_url(&f.owner, "acme/demo", None)
    .unwrap();
  assert!(
    f.store
      .project(&f.reader, "acme/demo")
      .unwrap()
      .repository_url
      .is_none()
  );

  let audit = f.store.list_audit(Some("acme/demo"), 20).unwrap();
  let repo_entries: Vec<&sdd_vault_core::store::AuditEntry> = audit
    .iter()
    .filter(|e| e.action == "project.repo")
    .collect();
  assert_eq!(repo_entries.len(), 3);
  assert!(
    repo_entries[0].target.contains("cleared"),
    "{:?}",
    repo_entries[0]
  );
  assert_eq!(repo_entries[0].actor, "owner");
  assert_eq!(repo_entries[1].actor, "root");
}
