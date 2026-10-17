mod common;

use std::fs;
use std::path::Path;

use spec_store::model::{DocumentKind, MAX_BODY_BYTES};
use spec_store::tree::{ExportFile, ProjectTree, TargetState, plan_export, scan, write_export};

fn put(root: &Path, relative: &str, bytes: &[u8]) {
  let path = root.join(relative);
  fs::create_dir_all(path.parent().unwrap()).unwrap();
  fs::write(path, bytes).unwrap();
}

fn problem_mentions(tree: &ProjectTree, needle: &str) -> bool {
  tree.problems.iter().any(|p| p.contains(needle))
}

#[test]
fn reads_constitution_and_spec_documents_in_kind_order_with_exact_bytes() {
  let root = common::temp_dir("tree-read");
  put(&root, "docs/constitution.md", b"# C\r\n\r\nVersion 1.0\r\n");
  put(&root, "specs/001-alpha/tasks.md", b"# Tasks");
  put(
    &root,
    "specs/001-alpha/spec.md",
    "\u{feff}# Spec\nStatus: approved\n".as_bytes(),
  );
  put(&root, "specs/001-alpha/plan.md", b"# Plan\n");
  put(&root, "specs/002-beta/validation.md", b"# V\n");
  put(&root, "specs/002-beta/research.md", b"# R\n");

  let tree = scan(&root).unwrap();
  assert!(tree.problems.is_empty(), "{:?}", tree.problems);
  assert!(tree.skipped.is_empty(), "{:?}", tree.skipped);

  let constitution = tree.constitution.as_ref().expect("constitution read");
  assert_eq!(constitution.relative_path, "docs/constitution.md");
  assert_eq!(constitution.body, "# C\r\n\r\nVersion 1.0\r\n");

  assert_eq!(tree.specs.len(), 2);
  let alpha = &tree.specs[0];
  assert_eq!(alpha.folder, "001-alpha");
  assert_eq!(alpha.number, 1);
  assert_eq!(alpha.slug, "alpha");
  let kinds: Vec<DocumentKind> = alpha.documents.iter().map(|(k, _)| *k).collect();
  assert_eq!(
    kinds,
    vec![DocumentKind::Spec, DocumentKind::Plan, DocumentKind::Tasks]
  );
  assert_eq!(
    alpha.documents[0].1.body,
    "\u{feff}# Spec\nStatus: approved\n"
  );
  assert_eq!(
    alpha.documents[0].1.relative_path,
    "specs/001-alpha/spec.md"
  );
  assert_eq!(alpha.documents[2].1.body, "# Tasks");

  let beta = &tree.specs[1];
  assert_eq!((beta.number, beta.slug.as_str()), (2, "beta"));
  let kinds: Vec<DocumentKind> = beta.documents.iter().map(|(k, _)| *k).collect();
  assert_eq!(
    kinds,
    vec![DocumentKind::Research, DocumentKind::Validation]
  );
}

#[test]
fn constitution_alone_and_spec_folder_without_documents_are_fine() {
  let root = common::temp_dir("tree-sparse");
  put(&root, "docs/constitution.md", b"Version 1.0\n");
  fs::create_dir_all(root.join("specs")).unwrap();
  let tree = scan(&root).unwrap();
  assert!(tree.problems.is_empty(), "{:?}", tree.problems);
  assert!(tree.specs.is_empty());

  let root = common::temp_dir("tree-empty-spec");
  fs::create_dir_all(root.join("specs/003-gamma")).unwrap();
  let tree = scan(&root).unwrap();
  assert!(tree.problems.is_empty(), "{:?}", tree.problems);
  assert!(tree.constitution.is_none());
  assert_eq!(tree.specs.len(), 1);
  assert!(tree.specs[0].documents.is_empty());
}

#[test]
fn unknown_entries_are_skipped_and_named() {
  let root = common::temp_dir("tree-skip");
  put(&root, "specs/README.md", b"readme");
  put(&root, "specs/001-alpha/spec.md", b"# Spec\n");
  put(&root, "specs/001-alpha/notes.md", b"notes");
  put(&root, "specs/001-alpha/contracts/api.yaml", b"x: 1");

  let tree = scan(&root).unwrap();
  assert!(tree.problems.is_empty(), "{:?}", tree.problems);
  assert_eq!(
    tree.skipped,
    vec![
      "specs/001-alpha/contracts/".to_string(),
      "specs/001-alpha/notes.md".to_string(),
      "specs/README.md".to_string(),
    ]
  );
  assert_eq!(tree.specs[0].documents.len(), 1);
}

#[test]
fn every_problem_is_collected() {
  let root = common::temp_dir("tree-problems");
  put(&root, "specs/01-short/spec.md", b"x");
  put(&root, "specs/001-alpha/spec.md", b"x");
  put(&root, "specs/001-other/spec.md", b"x");
  put(&root, "specs/002-alpha/spec.md", b"x");
  put(&root, "specs/003-bytes/spec.md", &[0x66, 0xff, 0xfe]);
  put(
    &root,
    "specs/004-big/plan.md",
    &vec![b'a'; MAX_BODY_BYTES + 1],
  );
  put(
    &root,
    "specs/005-limit/plan.md",
    &vec![b'a'; MAX_BODY_BYTES],
  );

  let tree = scan(&root).unwrap();
  assert!(
    problem_mentions(&tree, "specs/01-short"),
    "{:?}",
    tree.problems
  );
  assert!(
    problem_mentions(&tree, "specs/001-other"),
    "{:?}",
    tree.problems
  );
  assert!(
    problem_mentions(&tree, "specs/002-alpha"),
    "{:?}",
    tree.problems
  );
  assert!(
    problem_mentions(&tree, "specs/003-bytes/spec.md") && problem_mentions(&tree, "UTF-8"),
    "{:?}",
    tree.problems
  );
  assert!(
    problem_mentions(&tree, "specs/004-big/plan.md"),
    "{:?}",
    tree.problems
  );
  assert!(
    !problem_mentions(&tree, "005-limit"),
    "a file of exactly the limit is accepted: {:?}",
    tree.problems
  );
  assert_eq!(tree.problems.len(), 5, "{:?}", tree.problems);
}

#[test]
fn an_empty_directory_is_a_problem_and_a_missing_one_an_error() {
  let root = common::temp_dir("tree-nothing");
  let tree = scan(&root).unwrap();
  assert_eq!(tree.problems.len(), 1, "{:?}", tree.problems);
  assert!(
    problem_mentions(&tree, "constitution"),
    "{:?}",
    tree.problems
  );

  assert!(scan(&root.join("missing")).is_err());
}

fn file(relative_path: &str, body: &str) -> ExportFile {
  ExportFile {
    relative_path: relative_path.to_string(),
    body: body.to_string(),
  }
}

fn export_files() -> Vec<ExportFile> {
  vec![
    file("docs/constitution.md", "# C\r\n"),
    file("specs/001-alpha/spec.md", "\u{feff}# Spec\n"),
    file("specs/001-alpha/tasks.md", "# Tasks"),
  ]
}

#[test]
fn export_into_a_missing_directory_writes_every_file() {
  let root = common::temp_dir("export-new").join("out");
  let plan = plan_export(&root, export_files()).unwrap();
  let states: Vec<TargetState> = plan.targets.iter().map(|(_, s)| *s).collect();
  assert_eq!(states, vec![TargetState::New; 3]);
  assert!(plan.different().is_empty());

  write_export(&plan, false).unwrap();
  assert_eq!(
    fs::read(root.join("docs/constitution.md")).unwrap(),
    b"# C\r\n"
  );
  assert_eq!(
    fs::read(root.join("specs/001-alpha/spec.md")).unwrap(),
    "\u{feff}# Spec\n".as_bytes()
  );
  assert_eq!(
    fs::read(root.join("specs/001-alpha/tasks.md")).unwrap(),
    b"# Tasks"
  );
}

#[test]
fn unchanged_files_are_left_alone_and_different_ones_need_force() {
  let root = common::temp_dir("export-existing");
  put(&root, "docs/constitution.md", b"# C\r\n");
  put(&root, "specs/001-alpha/spec.md", b"locally edited\n");
  put(&root, "specs/001-alpha/notes.md", b"keep me\n");
  let unchanged_path = root.join("docs/constitution.md");
  let modified_before = fs::metadata(&unchanged_path).unwrap().modified().unwrap();

  let plan = plan_export(&root, export_files()).unwrap();
  let states: Vec<(&str, TargetState)> = plan
    .targets
    .iter()
    .map(|(f, s)| (f.relative_path.as_str(), *s))
    .collect();
  assert_eq!(
    states,
    vec![
      ("docs/constitution.md", TargetState::Unchanged),
      ("specs/001-alpha/spec.md", TargetState::Different),
      ("specs/001-alpha/tasks.md", TargetState::New),
    ]
  );
  assert_eq!(plan.different(), vec!["specs/001-alpha/spec.md"]);

  write_export(&plan, false).unwrap();
  assert_eq!(
    fs::read(root.join("specs/001-alpha/spec.md")).unwrap(),
    b"locally edited\n"
  );
  assert_eq!(
    fs::read(root.join("specs/001-alpha/tasks.md")).unwrap(),
    b"# Tasks"
  );

  write_export(&plan, true).unwrap();
  assert_eq!(
    fs::read(root.join("specs/001-alpha/spec.md")).unwrap(),
    "\u{feff}# Spec\n".as_bytes()
  );
  assert_eq!(
    fs::metadata(&unchanged_path).unwrap().modified().unwrap(),
    modified_before
  );
  assert_eq!(
    fs::read(root.join("specs/001-alpha/notes.md")).unwrap(),
    b"keep me\n"
  );
}
