mod common;

use std::fs;
use std::path::Path;

use spec_store::model::{Caller, DocumentKind, Role, SpecStatus, StoreError};
use spec_store::store::Store;
use spec_store::store::transfer::{ImportOptions, ImportReport};
use spec_store::tree::{ProjectTree, scan};

struct Fixture {
  store: Store,
  admin: Caller,
  owner: Caller,
  editor: Caller,
  reader: Caller,
  outsider: Caller,
}

fn fixture() -> Fixture {
  let store = Store::open(common::temp_db_path("store-transfer")).unwrap();
  store.create_user(None, "root", false).unwrap();
  let admin = store.caller_by_handle("root").unwrap();
  for handle in ["olivia", "eddie", "rita", "otto"] {
    store.create_user(Some(&admin), handle, false).unwrap();
  }
  store.create_project(&admin, "demo", "Demo").unwrap();
  store
    .set_member(&admin, "demo", "olivia", Role::Owner)
    .unwrap();
  store
    .set_member(&admin, "demo", "eddie", Role::Editor)
    .unwrap();
  store
    .set_member(&admin, "demo", "rita", Role::Reader)
    .unwrap();
  Fixture {
    owner: store.caller_by_handle("olivia").unwrap(),
    editor: store.caller_by_handle("eddie").unwrap(),
    reader: store.caller_by_handle("rita").unwrap(),
    outsider: store.caller_by_handle("otto").unwrap(),
    admin,
    store,
  }
}

fn put(root: &Path, relative: &str, body: &str) {
  let path = root.join(relative);
  fs::create_dir_all(path.parent().unwrap()).unwrap();
  fs::write(path, body).unwrap();
}

const CONSTITUTION: &str = "# Constitution\r\n\r\nVersion 1.2 · Ratified 2026-01-01\r\n";
const ALPHA_SPEC: &str = "\u{feff}# Spec 001\n\nStatus: approved · Created: 2026-01-01\n";
const ALPHA_TASKS: &str = "# Tasks\n\n- [x] T01 (FR-1) First — Done when: ok\nnot a task line";

fn sample_tree(label: &str) -> ProjectTree {
  let root = common::temp_dir(label);
  put(&root, "docs/constitution.md", CONSTITUTION);
  put(&root, "specs/001-alpha/spec.md", ALPHA_SPEC);
  put(&root, "specs/001-alpha/plan.md", "# Plan\n");
  put(&root, "specs/001-alpha/tasks.md", ALPHA_TASKS);
  put(
    &root,
    "specs/002-beta/spec.md",
    "# Spec 002\n\nno status line\n",
  );
  put(&root, "specs/005-gamma/spec.md", "Status: archived\n");
  let tree = scan(&root).unwrap();
  assert!(tree.problems.is_empty(), "{:?}", tree.problems);
  tree
}

fn counts(store: &Store) -> Vec<i64> {
  store
    .with_conn(|conn| {
      let mut out = Vec::new();
      for table in ["specs", "documents", "revisions", "audit_log"] {
        out.push(conn.query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0))?);
      }
      Ok(out)
    })
    .unwrap()
}

fn import(f: &Fixture, tree: &ProjectTree) -> ImportReport {
  f.store
    .import_tree(&f.owner, "demo", tree, &ImportOptions::default())
    .unwrap()
}

fn validation_message(result: Result<ImportReport, StoreError>) -> String {
  match result {
    Err(StoreError::Validation(message)) => message,
    other => panic!("expected a validation error, got {other:?}"),
  }
}

#[test]
fn owner_imports_numbers_slugs_statuses_and_exact_bodies() {
  let f = fixture();
  let report = import(&f, &sample_tree("transfer-import"));

  assert_eq!(report.constitution_version.as_deref(), Some("1.2"));
  assert_eq!(report.documents, 6);
  let branches: Vec<&str> = report.specs.iter().map(|s| s.branch.as_str()).collect();
  assert_eq!(branches, vec!["001-alpha", "002-beta", "005-gamma"]);
  assert_eq!(
    report.specs[0].kinds,
    vec![DocumentKind::Spec, DocumentKind::Plan, DocumentKind::Tasks]
  );
  assert_eq!(report.status_fallbacks, vec!["002-beta".to_string()]);

  let specs = f.store.list_specs(&f.owner, "demo").unwrap();
  let summary: Vec<(i64, &str, SpecStatus, &str)> = specs
    .iter()
    .map(|s| (s.number, s.slug.as_str(), s.status, s.branch.as_str()))
    .collect();
  assert_eq!(
    summary,
    vec![
      (1, "alpha", SpecStatus::Approved, "001-alpha"),
      (2, "beta", SpecStatus::Draft, "002-beta"),
      (5, "gamma", SpecStatus::Archived, "005-gamma"),
    ]
  );

  let spec = f
    .store
    .get_document(&f.reader, "demo", 1, DocumentKind::Spec)
    .unwrap();
  assert_eq!(spec.body, ALPHA_SPEC);
  assert_eq!(spec.revision.author, "olivia");
  assert_eq!(spec.revision.message, "import specs/001-alpha/spec.md");
  let tasks = f
    .store
    .get_document(&f.reader, "demo", 1, DocumentKind::Tasks)
    .unwrap();
  assert_eq!(tasks.body, ALPHA_TASKS);
  let archived = f
    .store
    .get_document(&f.reader, "demo", 5, DocumentKind::Spec)
    .unwrap();
  assert_eq!(archived.body, "Status: archived\n");

  let constitution = f.store.get_constitution(&f.reader, "demo").unwrap();
  assert_eq!(constitution.body, CONSTITUTION);
  assert_eq!(constitution.version, "1.2");
  assert_eq!(constitution.revision.message, "import docs/constitution.md");

  let audit = f.store.list_audit(Some("demo"), 100).unwrap();
  let count = |action: &str| audit.iter().filter(|e| e.action == action).count();
  assert_eq!(count("spec.import"), 3);
  assert_eq!(count("document.put"), 5);
  assert_eq!(count("constitution.put"), 1);
  assert!(
    audit
      .iter()
      .filter(|e| e.action == "spec.import")
      .all(|e| e.actor == "olivia")
  );
  assert!(audit.iter().any(|e| e.target == "001-alpha"));
}

#[test]
fn next_spec_after_import_is_one_above_the_highest() {
  let f = fixture();
  import(&f, &sample_tree("transfer-next"));
  let created = f.store.create_spec(&f.editor, "demo", "delta").unwrap();
  assert_eq!(created.number, 6);
  assert_eq!(created.branch, "006-delta");
}

#[test]
fn constitution_version_comes_from_the_option_or_is_required() {
  let f = fixture();
  let root = common::temp_dir("transfer-version");
  put(
    &root,
    "docs/constitution.md",
    "# Constitution without a version\n",
  );
  let tree = scan(&root).unwrap();

  let before = counts(&f.store);
  let message =
    validation_message(
      f.store
        .import_tree(&f.owner, "demo", &tree, &ImportOptions::default()),
    );
  assert!(message.contains("docs/constitution.md"), "{message}");
  assert_eq!(counts(&f.store), before);

  let report = f
    .store
    .import_tree(
      &f.owner,
      "demo",
      &tree,
      &ImportOptions {
        constitution_version: Some("3.0".into()),
        dry_run: false,
      },
    )
    .unwrap();
  assert_eq!(report.constitution_version.as_deref(), Some("3.0"));
  assert_eq!(
    f.store.get_constitution(&f.owner, "demo").unwrap().version,
    "3.0"
  );

  let f = fixture();
  let report = f
    .store
    .import_tree(
      &f.owner,
      "demo",
      &sample_tree("transfer-override"),
      &ImportOptions {
        constitution_version: Some("9.9".into()),
        dry_run: false,
      },
    )
    .unwrap();
  assert_eq!(report.constitution_version.as_deref(), Some("9.9"));
}

#[test]
fn conflicts_with_existing_data_are_all_listed_and_nothing_is_written() {
  let f = fixture();
  f.store.create_spec(&f.owner, "demo", "beta").unwrap();
  f.store
    .put_constitution(&f.owner, "demo", "# Existing\n", "1.0", "init", None)
    .unwrap();
  let before = counts(&f.store);

  let message = validation_message(f.store.import_tree(
    &f.owner,
    "demo",
    &sample_tree("transfer-conflict"),
    &ImportOptions::default(),
  ));
  for needle in ["001-alpha", "002-beta", "constitution"] {
    assert!(message.contains(needle), "{needle} missing in {message}");
  }
  assert_eq!(message.lines().count(), 3, "{message}");
  assert_eq!(counts(&f.store), before);
}

#[test]
fn tree_problems_refuse_the_import() {
  let f = fixture();
  let mut tree = sample_tree("transfer-tree-problem");
  tree
    .problems
    .push("specs/01-x: folder name is not valid".into());
  let before = counts(&f.store);
  let message =
    validation_message(
      f.store
        .import_tree(&f.owner, "demo", &tree, &ImportOptions::default()),
    );
  assert!(message.contains("specs/01-x"), "{message}");
  assert_eq!(counts(&f.store), before);
}

#[test]
fn only_owners_import() {
  let f = fixture();
  let tree = sample_tree("transfer-roles");
  let before = counts(&f.store);
  for caller in [&f.editor, &f.reader, &f.admin, &f.outsider] {
    let result = f
      .store
      .import_tree(caller, "demo", &tree, &ImportOptions::default());
    assert!(
      matches!(result, Err(StoreError::Forbidden)),
      "{}: {result:?}",
      caller.handle
    );
  }
  assert!(matches!(
    f.store
      .import_tree(&f.admin, "nope", &tree, &ImportOptions::default()),
    Err(StoreError::NotFound(_))
  ));
  assert_eq!(counts(&f.store), before);
}

#[test]
fn dry_run_reports_without_writing() {
  let f = fixture();
  let before = counts(&f.store);
  let report = f
    .store
    .import_tree(
      &f.owner,
      "demo",
      &sample_tree("transfer-dry"),
      &ImportOptions {
        constitution_version: None,
        dry_run: true,
      },
    )
    .unwrap();
  assert_eq!(report.specs.len(), 3);
  assert_eq!(report.documents, 6);
  assert_eq!(report.status_fallbacks, vec!["002-beta".to_string()]);
  assert_eq!(counts(&f.store), before);
}

#[test]
fn a_failure_in_the_middle_leaves_the_store_unchanged() {
  let f = fixture();
  let root = common::temp_dir("transfer-abort");
  put(&root, "docs/constitution.md", CONSTITUTION);
  put(&root, "specs/001-alpha/spec.md", ALPHA_SPEC);
  put(&root, "specs/002-beta/spec.md", "ABORT-IMPORT\n");
  let tree = scan(&root).unwrap();
  f.store
    .with_conn(|conn| {
      conn.execute_batch(
        "CREATE TRIGGER abort_marked BEFORE INSERT ON revisions
         WHEN NEW.body LIKE '%ABORT-IMPORT%'
         BEGIN SELECT RAISE(ABORT, 'forced failure'); END;",
      )?;
      Ok(())
    })
    .unwrap();
  let before = counts(&f.store);
  let result = f
    .store
    .import_tree(&f.owner, "demo", &tree, &ImportOptions::default());
  assert!(matches!(result, Err(StoreError::Database(_))), "{result:?}");
  assert_eq!(counts(&f.store), before);
  assert!(f.store.list_specs(&f.owner, "demo").unwrap().is_empty());
}
