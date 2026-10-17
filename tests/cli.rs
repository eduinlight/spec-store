mod common;

use std::path::Path;
use std::process::Command;

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
    "no stdout on failure, got {}",
    out.stdout
  );
  assert!(
    !out.stderr.trim().is_empty(),
    "failure must explain itself on stderr"
  );
  out
}

#[test]
fn first_user_is_admin_then_as_is_required() {
  let db = common::temp_db_path("cli-users");
  let out = ok(&db, &["user", "add", "alice"]);
  assert!(
    out.stdout.contains("alice") && out.stdout.contains("admin"),
    "{}",
    out.stdout
  );

  let out = fails(&db, &["user", "add", "bob"]);
  assert!(out.stderr.contains("--as"), "{}", out.stderr);

  fails(&db, &["user", "add", "alice", "--as", "alice"]);
  ok(&db, &["user", "add", "bob", "--as", "alice"]);
  fails(&db, &["user", "add", "carol", "--as", "bob"]);
  fails(&db, &["user", "add", "carol", "--as", "nobody"]);
  let out = ok(&db, &["user", "add", "carol", "--admin", "--as", "alice"]);
  assert!(out.stdout.contains("admin"), "{}", out.stdout);
}

#[test]
fn tokens_are_printed_once_and_listed_without_secret() {
  let db = common::temp_db_path("cli-tokens");
  ok(&db, &["user", "add", "alice"]);
  ok(&db, &["user", "add", "bob", "--as", "alice"]);

  let out = ok(
    &db,
    &[
      "token", "issue", "bob", "--label", "laptop", "--as", "alice",
    ],
  );
  let token = out.stdout.trim().to_string();
  assert!(
    token.starts_with("sst_"),
    "stdout must be the token only, got {:?}",
    out.stdout
  );
  assert_eq!(out.stdout.lines().count(), 1);

  let listed = ok(&db, &["token", "list", "bob", "--as", "alice"]);
  assert!(listed.stdout.contains("laptop"), "{}", listed.stdout);
  assert!(!listed.stdout.contains(&token));
  let id = listed
    .stdout
    .lines()
    .find(|l| l.contains("laptop"))
    .and_then(|l| l.split_whitespace().next())
    .expect("token id first on the line")
    .to_string();

  ok(&db, &["token", "revoke", &id, "--as", "alice"]);
  let listed = ok(&db, &["token", "list", "bob", "--as", "alice"]);
  assert!(listed.stdout.contains("revoked"), "{}", listed.stdout);

  fails(
    &db,
    &["token", "issue", "bob", "--label", "x", "--as", "bob"],
  );
  fails(&db, &["token", "revoke", "999", "--as", "alice"]);
}

#[test]
fn projects_members_and_audit() {
  let db = common::temp_db_path("cli-projects");
  ok(&db, &["user", "add", "alice"]);
  for h in ["bob", "carol"] {
    ok(&db, &["user", "add", h, "--as", "alice"]);
  }
  ok(
    &db,
    &["project", "add", "demo", "Demo project", "--as", "alice"],
  );
  fails(&db, &["project", "add", "demo", "Again", "--as", "alice"]);
  fails(&db, &["project", "add", "other", "Other", "--as", "bob"]);

  ok(
    &db,
    &["member", "set", "demo", "bob", "editor", "--as", "alice"],
  );
  fails(
    &db,
    &["member", "set", "demo", "carol", "reader", "--as", "bob"],
  );
  ok(
    &db,
    &["member", "set", "demo", "bob", "owner", "--as", "alice"],
  );
  ok(
    &db,
    &["member", "set", "demo", "carol", "reader", "--as", "bob"],
  );
  fails(
    &db,
    &["member", "set", "demo", "carol", "admin", "--as", "bob"],
  );
  ok(&db, &["member", "remove", "demo", "carol", "--as", "bob"]);
  fails(&db, &["member", "remove", "demo", "carol", "--as", "bob"]);

  let audit = ok(&db, &["audit", "list", "--as", "alice"]);
  for action in [
    "user.create",
    "project.create",
    "member.set",
    "member.remove",
  ] {
    assert!(
      audit.stdout.contains(action),
      "missing {action} in\n{}",
      audit.stdout
    );
  }
  assert!(audit.stdout.contains("bob"), "{}", audit.stdout);

  let scoped = ok(
    &db,
    &[
      "audit",
      "list",
      "--project",
      "demo",
      "--limit",
      "2",
      "--as",
      "alice",
    ],
  );
  assert_eq!(scoped.stdout.lines().count(), 2, "{}", scoped.stdout);
  assert!(!scoped.stdout.contains("user.create"));

  fails(&db, &["audit", "list", "--as", "bob"]);
}

#[test]
fn help_and_unknown_command() {
  let db = common::temp_db_path("cli-help");
  let out = run(&db, &["--help"]);
  assert_eq!(out.code, 0);
  for cmd in [
    "serve", "stdio", "user", "token", "project", "member", "audit",
  ] {
    assert!(
      out.stdout.contains(cmd),
      "help should mention {cmd}\n{}",
      out.stdout
    );
  }
  let out = run(&db, &["frobnicate"]);
  assert_ne!(out.code, 0);
}
