mod common;

use sdd_vault_core::store::Store;

use sdd_vault_core::model::{Caller, DocumentKind, Role, SpecStatus, StoreError};

struct Fixture {
  store: common::TestStore,
  root: Caller,
  admin: Caller,
  editor: Caller,
  outsider: Caller,
}

fn fixture() -> Fixture {
  let store = common::test_store();
  let root = common::person(&store, "root");
  common::organization(&store, &root, common::ORG);
  let editor = common::person(&store, "editor");
  common::join(&store, &root, common::ORG, &editor);
  store.create_project(&root, "acme/demo", "Demo").unwrap();
  store
    .set_member(&root, "acme/demo", "editor", Role::Editor)
    .unwrap();
  Fixture {
    admin: common::operator(&store, "ops"),
    outsider: common::person(&store, "outsider"),
    editor,
    root,
    store,
  }
}

fn insert_spec(store: &Store, number: i64, slug: &str, status: &str) -> i64 {
  common::insert(
    store,
    &format!(
      "INSERT INTO specs (project_id, number, slug, status, branch)
       VALUES ((SELECT id FROM projects WHERE slug = 'demo'), {number}, '{slug}', '{status}', lpad('{number}', 3, '0') || '-{slug}') RETURNING id"
    ),
  )
}

fn insert_document(store: &Store, spec_id: i64, kind: &str) {
  common::exec(
    store,
    &format!(
      "INSERT INTO documents (project_id, spec_id, kind)
       VALUES ((SELECT id FROM projects WHERE slug = 'demo'), {spec_id}, '{kind}')"
    ),
  );
}

#[test]
fn list_specs_with_kinds_ordered() {
  let f = fixture();
  let second = insert_spec(&f.store, 2, "second", "clarified");
  let first = insert_spec(&f.store, 1, "first", "approved");
  insert_document(&f.store, first, "spec");
  insert_document(&f.store, first, "tasks");
  insert_document(&f.store, second, "spec");

  let specs = f.store.list_specs(&f.editor, "acme/demo").unwrap();
  assert_eq!(specs.len(), 2);
  assert_eq!(specs[0].number, 1);
  assert_eq!(specs[0].slug, "first");
  assert_eq!(specs[0].status, SpecStatus::Approved);
  assert_eq!(specs[0].branch, "001-first");
  assert_eq!(
    specs[0].kinds,
    vec![DocumentKind::Spec, DocumentKind::Tasks]
  );
  assert_eq!(specs[1].number, 2);
  assert_eq!(specs[1].kinds, vec![DocumentKind::Spec]);

  assert!(f.store.list_specs(&f.admin, "acme/demo").unwrap().len() == 2);
  assert!(matches!(
    f.store.list_specs(&f.outsider, "acme/demo"),
    Err(StoreError::NotFound(_))
  ));
  assert!(f.store.list_specs(&f.admin, "acme/demo").is_ok());
}

#[test]
fn list_specs_is_empty_for_a_project_without_specs() {
  let f = fixture();
  assert!(
    f.store
      .list_specs(&f.editor, "acme/demo")
      .unwrap()
      .is_empty()
  );
}

#[test]
fn active_spec_skips_archived_and_reports_none() {
  let f = fixture();
  assert_eq!(f.store.active_spec(&f.editor, "acme/demo").unwrap(), None);

  insert_spec(&f.store, 1, "first", "approved");
  insert_spec(&f.store, 2, "second", "draft");
  insert_spec(&f.store, 3, "third", "archived");

  let active = f
    .store
    .active_spec(&f.editor, "acme/demo")
    .unwrap()
    .unwrap();
  assert_eq!(active.number, 2);
  assert_eq!(active.slug, "second");
  assert_eq!(active.status, SpecStatus::Draft);
  assert_eq!(active.branch, "002-second");

  assert!(matches!(
    f.store.active_spec(&f.outsider, "acme/demo"),
    Err(StoreError::NotFound(_))
  ));
}

fn full_fixture() -> (Fixture, Caller, Caller) {
  let f = fixture();
  let owner = common::person(&f.store, "owner");
  let reader = common::person(&f.store, "reader");
  common::join(&f.store, &f.root, common::ORG, &owner);
  common::join(&f.store, &f.root, common::ORG, &reader);
  f.store
    .set_member(&f.root, "acme/demo", "owner", Role::Owner)
    .unwrap();
  f.store
    .set_member(&f.root, "acme/demo", "reader", Role::Reader)
    .unwrap();
  (f, owner, reader)
}

#[test]
fn create_spec_assigns_next_number_and_branch() {
  let (f, owner, _) = full_fixture();
  let alpha = f
    .store
    .create_spec(&f.editor, "acme/demo", "alpha")
    .unwrap();
  assert_eq!(alpha.number, 1);
  assert_eq!(alpha.slug, "alpha");
  assert_eq!(alpha.branch, "001-alpha");
  assert_eq!(alpha.status, SpecStatus::Draft);

  let beta = f
    .store
    .create_spec(&owner, "acme/demo", "beta-two")
    .unwrap();
  assert_eq!(beta.number, 2);
  assert_eq!(beta.branch, "002-beta-two");

  f.store
    .set_spec_status(&owner, "acme/demo", 2, SpecStatus::Archived)
    .unwrap();
  let gamma = f
    .store
    .create_spec(&f.editor, "acme/demo", "gamma")
    .unwrap();
  assert_eq!(gamma.number, 3, "archived specs keep their number");

  let listed = f.store.list_specs(&f.editor, "acme/demo").unwrap();
  assert_eq!(listed.len(), 3);
  assert!(
    listed.iter().all(|s| s.kinds.is_empty()),
    "creation writes no document"
  );
  let audit = f.store.list_audit(Some("acme/demo"), 10).unwrap();
  assert!(
    audit
      .iter()
      .any(|e| e.action == "spec.create" && e.target == "003-gamma")
  );
}

#[test]
fn rejects_bad_or_duplicate_slug() {
  let (f, _, reader) = full_fixture();
  f.store
    .create_spec(&f.editor, "acme/demo", "alpha")
    .unwrap();
  match f
    .store
    .create_spec(&f.editor, "acme/demo", "alpha")
    .unwrap_err()
  {
    StoreError::Validation(msg) => assert!(msg.contains("alpha"), "{msg}"),
    other => panic!("expected validation, got {other:?}"),
  }
  for bad in ["Alpha", "a_b", "one-two-three-four-five", ""] {
    assert!(
      matches!(
        f.store.create_spec(&f.editor, "acme/demo", bad),
        Err(StoreError::Validation(_))
      ),
      "{bad:?}"
    );
  }
  assert!(matches!(
    f.store.create_spec(&reader, "acme/demo", "beta"),
    Err(StoreError::Forbidden)
  ));
  assert!(matches!(
    f.store.create_spec(&f.outsider, "acme/demo", "beta"),
    Err(StoreError::NotFound(_))
  ));
  assert!(matches!(
    f.store.create_spec(&f.admin, "acme/demo", "beta"),
    Err(StoreError::Forbidden)
  ));
  assert_eq!(
    f.store.list_specs(&f.editor, "acme/demo").unwrap().len(),
    1,
    "no number consumed"
  );
}

#[test]
fn status_transitions_table() {
  let (f, owner, reader) = full_fixture();
  f.store
    .create_spec(&f.editor, "acme/demo", "alpha")
    .unwrap();
  let status = |n: usize| f.store.list_specs(&f.editor, "acme/demo").unwrap()[n].status;

  match f
    .store
    .set_spec_status(&f.editor, "acme/demo", 1, SpecStatus::Approved)
    .unwrap_err()
  {
    StoreError::Validation(msg) => assert!(msg.contains("clarified"), "{msg}"),
    other => panic!("expected validation, got {other:?}"),
  }
  f.store
    .set_spec_status(&f.editor, "acme/demo", 1, SpecStatus::Clarified)
    .unwrap();
  assert_eq!(status(0), SpecStatus::Clarified);
  f.store
    .set_spec_status(&f.editor, "acme/demo", 1, SpecStatus::Approved)
    .unwrap();
  f.store
    .set_spec_status(&f.editor, "acme/demo", 1, SpecStatus::Draft)
    .unwrap();
  assert_eq!(status(0), SpecStatus::Draft);

  assert!(matches!(
    f.store
      .set_spec_status(&f.editor, "acme/demo", 1, SpecStatus::Archived),
    Err(StoreError::Validation(_))
  ));
  assert!(matches!(
    f.store
      .set_spec_status(&reader, "acme/demo", 1, SpecStatus::Clarified),
    Err(StoreError::Forbidden)
  ));
  assert!(matches!(
    f.store
      .set_spec_status(&f.editor, "acme/demo", 9, SpecStatus::Clarified),
    Err(StoreError::NotFound(_))
  ));

  f.store
    .set_spec_status(&owner, "acme/demo", 1, SpecStatus::Archived)
    .unwrap();
  assert_eq!(status(0), SpecStatus::Archived);
  assert!(matches!(
    f.store
      .set_spec_status(&owner, "acme/demo", 1, SpecStatus::Draft),
    Err(StoreError::Validation(_))
  ));
  assert_eq!(f.store.active_spec(&f.editor, "acme/demo").unwrap(), None);
  let audit = f.store.list_audit(Some("acme/demo"), 20).unwrap();
  assert!(
    audit
      .iter()
      .any(|e| e.action == "spec.status" && e.target.contains("archived"))
  );
}

#[test]
fn concurrent_spec_creation_never_collides_or_skips() {
  let (f, owner, _) = full_fixture();
  let threads = 8;
  let barrier = std::sync::Arc::new(std::sync::Barrier::new(threads));
  let handles: Vec<_> = (0..threads)
    .map(|i| {
      let store = f.store.clone();
      let caller = if i % 2 == 0 {
        f.editor.clone()
      } else {
        owner.clone()
      };
      let barrier = barrier.clone();
      std::thread::spawn(move || {
        barrier.wait();
        store
          .create_spec(&caller, "acme/demo", &format!("spec-{i}"))
          .unwrap()
      })
    })
    .collect();
  let mut numbers: Vec<i64> = handles
    .into_iter()
    .map(|h| h.join().unwrap().number)
    .collect();
  numbers.sort_unstable();
  let expected: Vec<i64> = (1..=threads as i64).collect();
  assert_eq!(numbers, expected, "numbers must be dense and unique");
  let listed = f.store.list_specs(&f.editor, "acme/demo").unwrap();
  assert_eq!(listed.len(), threads);
  assert!(
    listed
      .iter()
      .all(|s| s.branch == format!("{:03}-{}", s.number, s.slug))
  );
}

#[test]
fn archiving_the_active_spec_moves_to_the_next_highest() {
  let (f, owner, reader) = full_fixture();
  f.store
    .create_spec(&f.editor, "acme/demo", "alpha")
    .unwrap();
  f.store.create_spec(&f.editor, "acme/demo", "beta").unwrap();
  assert_eq!(
    f.store
      .active_spec(&reader, "acme/demo")
      .unwrap()
      .unwrap()
      .number,
    2
  );

  f.store
    .set_spec_status(&owner, "acme/demo", 2, SpecStatus::Archived)
    .unwrap();
  let active = f.store.active_spec(&reader, "acme/demo").unwrap().unwrap();
  assert_eq!(active.number, 1);
  assert_eq!(active.slug, "alpha");

  f.store
    .set_spec_status(&owner, "acme/demo", 1, SpecStatus::Archived)
    .unwrap();
  assert!(f.store.active_spec(&reader, "acme/demo").unwrap().is_none());
  assert_eq!(f.store.list_specs(&reader, "acme/demo").unwrap().len(), 2);
}
