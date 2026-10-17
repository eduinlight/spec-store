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

fn imported(label: &str) -> std::path::PathBuf {
  let db = setup(label);
  let dir = sample_dir(&format!("{label}-src"));
  ok(
    &db,
    &["import", "demo", dir.to_str().unwrap(), "--as", "olivia"],
  );
  db
}

#[test]
fn export_writes_lists_and_then_reports_unchanged() {
  let db = imported("cli-export");
  let out_dir = common::temp_dir("cli-export-out").join("copy");
  let target = out_dir.to_str().unwrap();
  let out = ok(&db, &["export", "demo", target, "--as", "eddie"]);
  assert_eq!(
    out.stdout,
    "written docs/constitution.md\n\
     written specs/001-alpha/spec.md\n\
     written specs/001-alpha/plan.md\n\
     written specs/001-alpha/tasks.md\n\
     written specs/002-beta/spec.md\n\
     exported 5 files: 5 written, 0 unchanged\n"
  );
  assert_eq!(
    fs::read_to_string(out_dir.join("specs/001-alpha/spec.md")).unwrap(),
    "# Spec\n\nStatus: approved · Created\n"
  );

  let out = ok(&db, &["export", "demo", target, "--as", "alice"]);
  assert!(
    out
      .stdout
      .ends_with("exported 5 files: 0 written, 5 unchanged\n"),
    "{}",
    out.stdout
  );
  assert!(out.stdout.contains("unchanged specs/002-beta/spec.md\n"));

  let audit = ok(
    &db,
    &["audit", "list", "--project", "demo", "--as", "alice"],
  );
  assert!(!audit.stdout.contains("export"), "{}", audit.stdout);
}

#[test]
fn differing_files_block_the_export_unless_forced() {
  let db = imported("cli-force");
  let out_dir = common::temp_dir("cli-force-out");
  let target = out_dir.to_str().unwrap();
  put(&out_dir, "specs/001-alpha/plan.md", "local edit\n");
  put(&out_dir, "specs/002-beta/spec.md", "another edit\n");

  let out = fails(&db, &["export", "demo", target, "--as", "eddie"]);
  assert!(out.stderr.contains("--force"), "{}", out.stderr);
  assert!(
    out.stderr.contains("  - specs/001-alpha/plan.md"),
    "{}",
    out.stderr
  );
  assert!(
    out.stderr.contains("  - specs/002-beta/spec.md"),
    "{}",
    out.stderr
  );
  assert!(!out_dir.join("docs/constitution.md").exists());

  let out = ok(&db, &["export", "demo", target, "--force", "--as", "eddie"]);
  assert!(out.stdout.contains("written specs/001-alpha/plan.md\n"));
  assert_eq!(
    fs::read_to_string(out_dir.join("specs/001-alpha/plan.md")).unwrap(),
    "# Plan\n"
  );
}

#[test]
fn export_of_one_spec_with_a_status_notice() {
  let db = imported("cli-one");
  {
    let store = Store::open(&db).unwrap();
    let owner = store.caller_by_handle("olivia").unwrap();
    store
      .set_spec_status(&owner, "demo", 1, spec_store::model::SpecStatus::Draft)
      .unwrap();
  }
  let out_dir = common::temp_dir("cli-one-out");
  let out = ok(
    &db,
    &[
      "export",
      "demo",
      out_dir.to_str().unwrap(),
      "--spec",
      "1",
      "--as",
      "eddie",
    ],
  );
  assert_eq!(
    out.stdout,
    "written specs/001-alpha/spec.md\n\
     written specs/001-alpha/plan.md\n\
     written specs/001-alpha/tasks.md\n\
     exported 3 files: 3 written, 0 unchanged\n"
  );
  assert!(
    out
      .stderr
      .contains("notice: spec 001-alpha is draft in the store but its spec.md says approved"),
    "{}",
    out.stderr
  );
  fails(
    &db,
    &[
      "export",
      "demo",
      out_dir.to_str().unwrap(),
      "--spec",
      "9",
      "--as",
      "eddie",
    ],
  );
}

#[test]
fn outsiders_cannot_export() {
  let db = imported("cli-outsider");
  let out_dir = common::temp_dir("cli-outsider-out").join("x");
  let out = fails(
    &db,
    &["export", "demo", out_dir.to_str().unwrap(), "--as", "otto"],
  );
  assert!(out.stderr.contains("member"), "{}", out.stderr);
  assert!(!out_dir.exists());
}

fn copy_tree(from: &Path, to: &Path) {
  fs::create_dir_all(to).unwrap();
  for entry in fs::read_dir(from).unwrap() {
    let entry = entry.unwrap();
    let target = to.join(entry.file_name());
    if entry.path().is_dir() {
      copy_tree(&entry.path(), &target);
    } else {
      fs::copy(entry.path(), target).unwrap();
    }
  }
}

fn imported_files(root: &Path) -> Vec<String> {
  let mut files = vec!["docs/constitution.md".to_string()];
  let mut folders: Vec<_> = fs::read_dir(root.join("specs"))
    .unwrap()
    .map(|e| e.unwrap())
    .filter(|e| e.path().is_dir())
    .collect();
  folders.sort_by_key(|e| e.file_name());
  for folder in folders {
    let name = folder.file_name().to_string_lossy().into_owned();
    for kind in ["spec", "plan", "tasks", "research", "validation"] {
      if folder.path().join(format!("{kind}.md")).is_file() {
        files.push(format!("specs/{name}/{kind}.md"));
      }
    }
  }
  files
}

#[test]
fn this_repository_round_trips_byte_for_byte() {
  let repo = Path::new(env!("CARGO_MANIFEST_DIR"));
  let source = common::temp_dir("cli-round-trip-src");
  fs::create_dir_all(source.join("docs")).unwrap();
  fs::copy(
    repo.join("docs/constitution.md"),
    source.join("docs/constitution.md"),
  )
  .unwrap();
  copy_tree(&repo.join("specs"), &source.join("specs"));

  let db = setup("cli-round-trip");
  let out = ok(
    &db,
    &["import", "demo", source.to_str().unwrap(), "--as", "olivia"],
  );
  let files = imported_files(&source);
  assert!(files.len() > 10, "{files:?}");
  assert!(out.stdout.ends_with(" documents\n"), "{}", out.stdout);

  let target = common::temp_dir("cli-round-trip-out");
  let out = ok(
    &db,
    &["export", "demo", target.to_str().unwrap(), "--as", "olivia"],
  );
  assert!(
    out.stdout.ends_with(&format!(
      "exported {n} files: {n} written, 0 unchanged\n",
      n = files.len()
    )),
    "{}",
    out.stdout
  );
  for file in &files {
    assert_eq!(
      fs::read(target.join(file)).unwrap(),
      fs::read(source.join(file)).unwrap(),
      "{file} differs after the round trip"
    );
  }
}
