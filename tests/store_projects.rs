mod common;

use spec_store::model::{Caller, Role, StoreError};
use spec_store::store::Store;

struct Fixture {
  store: Store,
  admin: Caller,
  owner: Caller,
  editor: Caller,
  reader: Caller,
  outsider: Caller,
}

fn fixture() -> Fixture {
  let store = Store::open(common::temp_db_path("store-projects")).unwrap();
  store.create_user(None, "root", false).unwrap();
  let admin = store.caller_by_handle("root").unwrap();
  for h in ["owner", "editor", "reader", "outsider"] {
    store.create_user(Some(&admin), h, false).unwrap();
  }
  store
    .create_project(&admin, "demo", "Demo project")
    .unwrap();
  store
    .set_member(&admin, "demo", "owner", Role::Owner)
    .unwrap();
  store
    .set_member(&admin, "demo", "editor", Role::Editor)
    .unwrap();
  store
    .set_member(&admin, "demo", "reader", Role::Reader)
    .unwrap();
  Fixture {
    owner: store.caller_by_handle("owner").unwrap(),
    editor: store.caller_by_handle("editor").unwrap(),
    reader: store.caller_by_handle("reader").unwrap(),
    outsider: store.caller_by_handle("outsider").unwrap(),
    admin,
    store,
  }
}

#[test]
fn creates_project_and_rejects_duplicate_slug() {
  let f = fixture();
  let project = f
    .store
    .create_project(&f.admin, "second", "Second")
    .unwrap();
  assert_eq!(project.slug, "second");
  match f
    .store
    .create_project(&f.admin, "demo", "Again")
    .unwrap_err()
  {
    StoreError::Validation(msg) => assert!(msg.contains("demo"), "{msg}"),
    other => panic!("expected validation, got {other:?}"),
  }
  assert!(matches!(
    f.store.create_project(&f.admin, "Bad Slug", "x"),
    Err(StoreError::Validation(_))
  ));
  assert!(matches!(
    f.store.create_project(&f.owner, "third", "x"),
    Err(StoreError::Forbidden)
  ));
}

#[test]
fn set_member_replaces_role_and_remove_ends_access() {
  let f = fixture();
  f.store
    .set_member(&f.owner, "demo", "outsider", Role::Reader)
    .unwrap();
  f.store
    .set_member(&f.owner, "demo", "outsider", Role::Editor)
    .unwrap();
  let access = f.store.access(&f.outsider, "demo").unwrap();
  assert_eq!(access.role, Some(Role::Editor));
  let count: i64 = f
    .store
    .with_conn(|c| {
      Ok(c.query_row(
        "SELECT count(*) FROM memberships WHERE user_id = ?1",
        [f.outsider.user_id],
        |r| r.get(0),
      )?)
    })
    .unwrap();
  assert_eq!(count, 1);

  f.store.remove_member(&f.owner, "demo", "outsider").unwrap();
  assert!(matches!(
    f.store.access(&f.outsider, "demo"),
    Err(StoreError::Forbidden)
  ));

  assert!(matches!(
    f.store
      .set_member(&f.editor, "demo", "outsider", Role::Reader),
    Err(StoreError::Forbidden)
  ));
  assert!(matches!(
    f.store.set_member(&f.owner, "demo", "nobody", Role::Reader),
    Err(StoreError::NotFound(_))
  ));
  assert!(matches!(
    f.store.remove_member(&f.owner, "demo", "nobody"),
    Err(StoreError::NotFound(_))
  ));
  assert!(matches!(
    f.store
      .set_member(&f.admin, "missing", "outsider", Role::Reader),
    Err(StoreError::NotFound(_))
  ));
}

#[test]
fn list_projects_by_membership_and_admin_sees_all() {
  let f = fixture();
  f.store.create_project(&f.admin, "other", "Other").unwrap();

  let mine = f.store.list_projects(&f.editor).unwrap();
  assert_eq!(mine.len(), 1);
  assert_eq!(mine[0].slug, "demo");
  assert_eq!(mine[0].name, "Demo project");
  assert_eq!(mine[0].role, "editor");

  assert!(f.store.list_projects(&f.outsider).unwrap().is_empty());

  let all = f.store.list_projects(&f.admin).unwrap();
  let slugs: Vec<&str> = all.iter().map(|p| p.slug.as_str()).collect();
  assert_eq!(slugs, vec!["demo", "other"]);
  assert!(all.iter().all(|p| p.role == "admin"));

  f.store
    .set_member(&f.admin, "other", "root", Role::Reader)
    .unwrap();
  let all = f.store.list_projects(&f.admin).unwrap();
  assert_eq!(
    all.iter().find(|p| p.slug == "other").unwrap().role,
    "reader"
  );
}

#[test]
fn non_member_gets_forbidden_without_existence_leak() {
  let f = fixture();
  let existing = f.store.access(&f.outsider, "demo").unwrap_err();
  let missing = f.store.access(&f.outsider, "missing").unwrap_err();
  assert!(matches!(existing, StoreError::Forbidden));
  assert!(matches!(missing, StoreError::Forbidden));
  assert_eq!(existing.to_string(), missing.to_string());

  let admin_access = f.store.access(&f.admin, "demo").unwrap();
  assert!(admin_access.is_admin);
  assert_eq!(admin_access.role, None);
  assert!(!admin_access.can_write());
  assert!(matches!(
    f.store.access(&f.admin, "missing"),
    Err(StoreError::NotFound(_))
  ));

  let reader = f.store.access(&f.reader, "demo").unwrap();
  assert!(!reader.can_write());
  assert!(f.store.access(&f.editor, "demo").unwrap().can_write());
  assert!(f.store.access(&f.owner, "demo").unwrap().is_owner());
  assert!(!f.store.access(&f.editor, "demo").unwrap().is_owner());
}

#[test]
fn list_members_orders_owners_first_and_handles_empty() {
  let f = fixture();
  let members = f.store.list_members(&f.reader, "demo").unwrap();
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
    f.store.list_members(&f.outsider, "demo"),
    Err(StoreError::Forbidden)
  ));
  assert_eq!(f.store.list_members(&f.admin, "demo").unwrap().len(), 3);

  f.store.create_project(&f.admin, "empty", "Empty").unwrap();
  assert!(f.store.list_members(&f.admin, "empty").unwrap().is_empty());
}

#[test]
fn repository_url_is_validated_audited_and_clearable() {
  let f = fixture();
  assert!(
    f.store
      .project(&f.reader, "demo")
      .unwrap()
      .repository_url
      .is_none()
  );

  f.store
    .set_repository_url(&f.owner, "demo", Some("https://github.com/acme/demo"))
    .unwrap();
  let project = f.store.project(&f.reader, "demo").unwrap();
  assert_eq!(
    project.repository_url.as_deref(),
    Some("https://github.com/acme/demo")
  );
  assert_eq!(project.slug, "demo");
  assert_eq!(project.name, "Demo project");

  f.store
    .set_repository_url(&f.admin, "demo", Some("http://git.internal/demo/"))
    .unwrap();
  assert_eq!(
    f.store
      .project(&f.admin, "demo")
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
        f.store.set_repository_url(&f.owner, "demo", Some(bad)),
        Err(StoreError::Validation(_))
      ),
      "{bad:?} must be rejected"
    );
  }
  assert_eq!(
    f.store
      .project(&f.admin, "demo")
      .unwrap()
      .repository_url
      .as_deref(),
    Some("http://git.internal/demo/"),
    "rejected values change nothing"
  );

  assert!(matches!(
    f.store
      .set_repository_url(&f.editor, "demo", Some("https://example.com/r")),
    Err(StoreError::Forbidden)
  ));
  assert!(matches!(
    f.store.set_repository_url(&f.outsider, "demo", None),
    Err(StoreError::Forbidden)
  ));
  assert!(matches!(
    f.store.project(&f.outsider, "demo"),
    Err(StoreError::Forbidden)
  ));
  assert!(matches!(
    f.store.project(&f.outsider, "missing"),
    Err(StoreError::Forbidden)
  ));
  assert!(matches!(
    f.store.project(&f.admin, "missing"),
    Err(StoreError::NotFound(_))
  ));

  f.store.set_repository_url(&f.owner, "demo", None).unwrap();
  assert!(
    f.store
      .project(&f.reader, "demo")
      .unwrap()
      .repository_url
      .is_none()
  );

  let audit = f.store.list_audit(Some("demo"), 20).unwrap();
  let repo_entries: Vec<&spec_store::store::AuditEntry> = audit
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
