mod common;

use std::fs;
use std::path::Path;
use std::process::Command;

use spec_store::store::Store;

struct Output {
  code: i32,
  stdout: String,
  stderr: String,
}

fn run(db: &Path, args: &[&str]) -> Output {
  let out = Command::new(env!("CARGO_BIN_EXE_spec-store"))
    .arg("--db")
    .arg(db)
    .args(args)
    .output()
    .expect("run binary");
  Output {
    code: out.status.code().unwrap_or(-1),
    stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
    stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
  }
}

fn ok(db: &Path, args: &[&str]) -> Output {
  let out = run(db, args);
  assert_eq!(
    out.code, 0,
    "args {args:?}\nstdout: {}\nstderr: {}",
    out.stdout, out.stderr
  );
  out
}

fn fails(db: &Path, args: &[&str]) -> Output {
  let out = run(db, args);
  assert_eq!(
    out.code, 1,
    "args {args:?} should fail\nstdout: {}\nstderr: {}",
    out.stdout, out.stderr
  );
  assert!(
    out.stdout.is_empty(),
    "no stdout on failure: {}",
    out.stdout
  );
  assert!(!out.stderr.trim().is_empty(), "failure explains itself");
  out
}

fn put(root: &Path, relative: &str, body: &str) {
  let path = root.join(relative);
  fs::create_dir_all(path.parent().unwrap()).unwrap();
  fs::write(path, body).unwrap();
}

fn setup(label: &str) -> std::path::PathBuf {
  let db = common::temp_db_path(label);
  ok(&db, &["user", "add", "alice"]);
  for handle in ["olivia", "eddie", "otto"] {
    ok(&db, &["user", "add", handle, "--as", "alice"]);
  }
  ok(&db, &["project", "add", "demo", "Demo", "--as", "alice"]);
  ok(
    &db,
    &["member", "set", "demo", "olivia", "owner", "--as", "alice"],
  );
  ok(
    &db,
    &["member", "set", "demo", "eddie", "editor", "--as", "alice"],
  );
  db
}

fn sample_dir(label: &str) -> std::path::PathBuf {
  let root = common::temp_dir(label);
  put(
    &root,
    "docs/constitution.md",
    "# C\n\nVersion 1.2 · Ratified\n",
  );
  put(&root, "specs/README.md", "readme\n");
  put(
    &root,
    "specs/001-alpha/spec.md",
    "# Spec\n\nStatus: approved · Created\n",
  );
  put(&root, "specs/001-alpha/plan.md", "# Plan\n");
  put(&root, "specs/001-alpha/tasks.md", "# Tasks\n");
  put(&root, "specs/002-beta/spec.md", "# Spec\n\nno status\n");
  fs::create_dir_all(root.join("specs/003-empty")).unwrap();
  root
}

fn spec_count(db: &Path) -> usize {
  let store = Store::open(db).unwrap();
  let admin = store.caller_by_handle("alice").unwrap();
  store.list_specs(&admin, "demo").unwrap().len()
}

#[test]
fn import_prints_a_report_and_notices() {
  let db = setup("cli-import");
  let dir = sample_dir("cli-import-dir");
  let out = ok(
    &db,
    &["import", "demo", dir.to_str().unwrap(), "--as", "olivia"],
  );
  assert_eq!(
    out.stdout,
    "constitution v1.2 from docs/constitution.md\n\
     spec 001-alpha approved: spec, plan, tasks\n\
     spec 002-beta draft: spec\n\
     spec 003-empty draft: no documents\n\
     imported 3 specs and 5 documents\n"
  );
  assert!(
    out.stderr.contains("skipped specs/README.md"),
    "{}",
    out.stderr
  );
  assert!(
    out
      .stderr
      .contains("notice: spec 002-beta has no recognised Status line; imported as draft"),
    "{}",
    out.stderr
  );
  assert!(out.stderr.contains("003-empty"), "{}", out.stderr);
  assert_eq!(spec_count(&db), 3);

  let audit = ok(
    &db,
    &["audit", "list", "--project", "demo", "--as", "alice"],
  );
  assert_eq!(audit.stdout.matches("spec.import").count(), 3);
}

#[test]
fn dry_run_reports_and_writes_nothing() {
  let db = setup("cli-dry");
  let dir = sample_dir("cli-dry-dir");
  let out = ok(
    &db,
    &[
      "import",
      "demo",
      dir.to_str().unwrap(),
      "--dry-run",
      "--as",
      "olivia",
    ],
  );
  assert!(
    out
      .stdout
      .contains("spec 001-alpha approved: spec, plan, tasks")
  );
  assert!(
    out
      .stdout
      .ends_with("dry run: would import 3 specs and 5 documents\n"),
    "{}",
    out.stdout
  );
  assert_eq!(spec_count(&db), 0);
}

#[test]
fn refusal_lists_every_problem_and_writes_nothing() {
  let db = setup("cli-refuse");
  let dir = sample_dir("cli-refuse-dir");
  put(&dir, "specs/01-short/spec.md", "x\n");
  put(&dir, "specs/004-alpha/spec.md", "x\n");
  let out = fails(
    &db,
    &["import", "demo", dir.to_str().unwrap(), "--as", "olivia"],
  );
  assert!(out.stderr.contains("import refused"), "{}", out.stderr);
  assert!(out.stderr.contains("specs/01-short"), "{}", out.stderr);
  assert!(out.stderr.contains("specs/004-alpha"), "{}", out.stderr);
  assert_eq!(spec_count(&db), 0);

  let missing = dir.join("missing");
  fails(
    &db,
    &[
      "import",
      "demo",
      missing.to_str().unwrap(),
      "--as",
      "olivia",
    ],
  );
}

#[test]
fn only_an_owner_imports_and_the_version_can_be_given() {
  let db = setup("cli-owner");
  let dir = sample_dir("cli-owner-dir");
  for handle in ["eddie", "alice", "otto"] {
    let out = fails(
      &db,
      &["import", "demo", dir.to_str().unwrap(), "--as", handle],
    );
    assert!(out.stderr.contains("owner"), "{handle}: {}", out.stderr);
  }
  fails(
    &db,
    &["import", "demo", dir.to_str().unwrap(), "--as", "nobody"],
  );
  assert_eq!(spec_count(&db), 0);

  let out = ok(
    &db,
    &[
      "import",
      "demo",
      dir.to_str().unwrap(),
      "--constitution-version",
      "2.0",
      "--as",
      "olivia",
    ],
  );
  assert!(
    out
      .stdout
      .starts_with("constitution v2.0 from docs/constitution.md\n"),
    "{}",
    out.stdout
  );
}
