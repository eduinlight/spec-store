mod common;

use std::fs;
use std::path::Path;

use sdd_vault_core::model::{OrgRole, Role, SpecStatus};

struct Run {
  code: i32,
  stdout: String,
  stderr: String,
}

async fn run(env: &common::TestEnv, as_handle: &str, args: &[&str]) -> Run {
  env.use_identity(as_handle);
  let out = env.run(args).await;
  Run {
    code: out.status.code().unwrap_or(-1),
    stdout: common::text(&out.stdout),
    stderr: common::text(&out.stderr),
  }
}

async fn ok(env: &common::TestEnv, as_handle: &str, args: &[&str]) -> Run {
  let out = run(env, as_handle, args).await;
  assert_eq!(
    out.code, 0,
    "args {args:?}\nstdout: {}\nstderr: {}",
    out.stdout, out.stderr
  );
  out
}

async fn fails(env: &common::TestEnv, as_handle: &str, args: &[&str]) -> Run {
  let out = run(env, as_handle, args).await;
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

async fn setup(label: &str) -> common::TestEnv {
  let env = common::start(label).await;
  let alice = env.person("alice");
  env
    .store
    .create_organization(&alice, "acme", "Acme")
    .unwrap();
  for handle in ["olivia", "eddie"] {
    let person = env.person(handle);
    env
      .store
      .admit_organization_member("acme", person.user_id, OrgRole::Member)
      .unwrap();
  }
  env.person("otto");
  env
    .store
    .create_project(&alice, "acme/demo", "Demo")
    .unwrap();
  env
    .store
    .set_member(&alice, "acme/demo", "olivia", Role::Owner)
    .unwrap();
  env
    .store
    .set_member(&alice, "acme/demo", "eddie", Role::Editor)
    .unwrap();
  env
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

fn spec_count(env: &common::TestEnv) -> usize {
  let alice = env.store.caller_by_handle("alice", false).unwrap();
  env.store.list_specs(&alice, "acme/demo").unwrap().len()
}

#[tokio::test]
async fn import_prints_a_report_and_notices() {
  let env = setup("cli-import").await;
  let dir = sample_dir("cli-import-dir");
  let out = ok(
    &env,
    "olivia",
    &["import", "acme/demo", dir.to_str().unwrap()],
  )
  .await;
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
  assert_eq!(spec_count(&env), 3);
  let audit = ok(
    &env,
    "alice",
    &["audit", "list", "acme", "--project", "demo"],
  )
  .await;
  assert_eq!(audit.stdout.matches("spec.import").count(), 3);
}

#[tokio::test]
async fn dry_run_reports_and_writes_nothing() {
  let env = setup("cli-dry").await;
  let dir = sample_dir("cli-dry-dir");
  let out = ok(
    &env,
    "olivia",
    &["import", "acme/demo", dir.to_str().unwrap(), "--dry-run"],
  )
  .await;
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
  assert_eq!(spec_count(&env), 0);
}

#[tokio::test]
async fn refusal_lists_every_problem_and_writes_nothing() {
  let env = setup("cli-refuse").await;
  let dir = sample_dir("cli-refuse-dir");
  put(&dir, "specs/01-short/spec.md", "x\n");
  put(&dir, "specs/004-alpha/spec.md", "x\n");
  let out = fails(
    &env,
    "olivia",
    &["import", "acme/demo", dir.to_str().unwrap()],
  )
  .await;
  assert!(out.stderr.contains("import refused"), "{}", out.stderr);
  assert!(out.stderr.contains("specs/01-short"), "{}", out.stderr);
  assert!(out.stderr.contains("specs/004-alpha"), "{}", out.stderr);
  assert_eq!(spec_count(&env), 0);
  let missing = dir.join("missing");
  fails(
    &env,
    "olivia",
    &["import", "acme/demo", missing.to_str().unwrap()],
  )
  .await;
}

#[tokio::test]
async fn only_an_owner_imports_and_the_version_can_be_given() {
  let env = setup("cli-owner").await;
  let dir = sample_dir("cli-owner-dir");
  let out = fails(
    &env,
    "eddie",
    &["import", "acme/demo", dir.to_str().unwrap()],
  )
  .await;
  assert!(out.stderr.contains("owner"), "{}", out.stderr);
  let out = fails(
    &env,
    "otto",
    &["import", "acme/demo", dir.to_str().unwrap()],
  )
  .await;
  assert!(out.stderr.contains("not found"), "{}", out.stderr);
  assert_eq!(spec_count(&env), 0);
  let out = ok(
    &env,
    "olivia",
    &[
      "import",
      "acme/demo",
      dir.to_str().unwrap(),
      "--constitution-version",
      "2.0",
    ],
  )
  .await;
  assert!(
    out
      .stdout
      .starts_with("constitution v2.0 from docs/constitution.md\n"),
    "{}",
    out.stdout
  );
}

async fn imported(label: &str) -> common::TestEnv {
  let env = setup(label).await;
  let dir = sample_dir(&format!("{label}-src"));
  ok(
    &env,
    "olivia",
    &["import", "acme/demo", dir.to_str().unwrap()],
  )
  .await;
  env
}

#[tokio::test]
async fn export_writes_lists_and_then_reports_unchanged() {
  let env = imported("cli-export").await;
  let out_dir = common::temp_dir("cli-export-out").join("copy");
  let target = out_dir.to_str().unwrap();
  let out = ok(&env, "eddie", &["export", "acme/demo", target]).await;
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
  let out = ok(&env, "alice", &["export", "acme/demo", target]).await;
  assert!(
    out
      .stdout
      .ends_with("exported 5 files: 0 written, 5 unchanged\n"),
    "{}",
    out.stdout
  );
  assert!(out.stdout.contains("unchanged specs/002-beta/spec.md\n"));
  let audit = ok(
    &env,
    "alice",
    &["audit", "list", "acme", "--project", "demo"],
  )
  .await;
  assert!(!audit.stdout.contains("export"), "{}", audit.stdout);
}

#[tokio::test]
async fn differing_files_block_the_export_unless_forced() {
  let env = imported("cli-force").await;
  let out_dir = common::temp_dir("cli-force-out");
  let target = out_dir.to_str().unwrap();
  put(&out_dir, "specs/001-alpha/plan.md", "local edit\n");
  put(&out_dir, "specs/002-beta/spec.md", "another edit\n");
  let out = fails(&env, "eddie", &["export", "acme/demo", target]).await;
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
  let out = ok(&env, "eddie", &["export", "acme/demo", target, "--force"]).await;
  assert!(out.stdout.contains("written specs/001-alpha/plan.md\n"));
  assert_eq!(
    fs::read_to_string(out_dir.join("specs/001-alpha/plan.md")).unwrap(),
    "# Plan\n"
  );
}

#[tokio::test]
async fn export_of_one_spec_with_a_status_notice() {
  let env = imported("cli-one").await;
  let owner = env.store.caller_by_handle("olivia", false).unwrap();
  env
    .store
    .set_spec_status(&owner, "acme/demo", 1, SpecStatus::Draft)
    .unwrap();
  let out_dir = common::temp_dir("cli-one-out");
  let out = ok(
    &env,
    "eddie",
    &[
      "export",
      "acme/demo",
      out_dir.to_str().unwrap(),
      "--spec",
      "1",
    ],
  )
  .await;
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
    &env,
    "eddie",
    &[
      "export",
      "acme/demo",
      out_dir.to_str().unwrap(),
      "--spec",
      "9",
    ],
  )
  .await;
}

#[tokio::test]
async fn outsiders_cannot_export() {
  let env = imported("cli-outsider").await;
  let out_dir = common::temp_dir("cli-outsider-out").join("x");
  let out = fails(
    &env,
    "otto",
    &["export", "acme/demo", out_dir.to_str().unwrap()],
  )
  .await;
  assert!(out.stderr.contains("not found"), "{}", out.stderr);
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

#[tokio::test]
async fn this_repository_round_trips_byte_for_byte() {
  let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
  let source = common::temp_dir("cli-round-trip-src");
  fs::create_dir_all(source.join("docs")).unwrap();
  fs::copy(
    repo.join("docs/constitution.md"),
    source.join("docs/constitution.md"),
  )
  .unwrap();
  copy_tree(&repo.join("specs"), &source.join("specs"));
  let env = setup("cli-round-trip").await;
  let out = ok(
    &env,
    "olivia",
    &["import", "acme/demo", source.to_str().unwrap()],
  )
  .await;
  let files = imported_files(&source);
  assert!(files.len() > 10, "{files:?}");
  assert!(out.stdout.ends_with(" documents\n"), "{}", out.stdout);
  let target = common::temp_dir("cli-round-trip-out");
  let out = ok(
    &env,
    "olivia",
    &["export", "acme/demo", target.to_str().unwrap()],
  )
  .await;
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
