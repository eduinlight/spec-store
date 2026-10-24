mod common;

use sdd_vault_core::model::OrgRole;

fn ok(output: &std::process::Output) -> String {
  assert!(
    output.status.success(),
    "stdout: {}\nstderr: {}",
    common::text(&output.stdout),
    common::text(&output.stderr)
  );
  common::text(&output.stdout)
}

fn failed(output: &std::process::Output) -> String {
  assert_eq!(
    output.status.code(),
    Some(1),
    "{}",
    common::text(&output.stdout)
  );
  assert!(output.stdout.is_empty(), "{}", common::text(&output.stdout));
  common::text(&output.stderr)
}

fn admit(env: &common::TestEnv, org: &str, handle: &str) {
  let person = env
    .store
    .caller_for(
      &sdd_vault_core::store::users::Identity {
        subject: format!("subject-{handle}"),
        handle: handle.to_string(),
        email: format!("{handle}@example.com"),
        name: handle.to_string(),
      },
      false,
    )
    .unwrap();
  env
    .store
    .admit_organization_member(org, person.user_id, OrgRole::Member)
    .unwrap();
}

#[tokio::test]
async fn organization_commands() {
  let env = common::start("org-commands").await;
  env.sign_in("alice").await;

  assert!(ok(&env.run(&["org", "create", "acme", "Acme Labs"]).await).contains("acme"));
  let listed = ok(&env.run(&["org", "list"]).await);
  assert_eq!(listed.trim(), "acme\towner\tAcme Labs");

  let invited = ok(
    &env
      .run(&["org", "invite", "acme", "carol@example.com"])
      .await,
  );
  assert!(invited.contains("carol@example.com"), "{invited}");
  let invitations = ok(&env.run(&["org", "invitations", "acme"]).await);
  let line = invitations
    .lines()
    .find(|l| l.contains("carol@example.com"))
    .expect("listed");
  let id = line.split('\t').next().unwrap().to_string();
  assert!(line.contains("pending"), "{line}");
  ok(&env.run(&["org", "resend", "acme", &id]).await);
  ok(&env.run(&["org", "revoke-invite", "acme", &id]).await);
  assert!(ok(&env.run(&["org", "invitations", "acme"]).await).contains("revoked"));

  admit(&env, "acme", "bob");
  ok(
    &env
      .run(&["org", "member", "set", "acme", "bob", "owner"])
      .await,
  );
  let members = ok(&env.run(&["org", "members", "acme"]).await);
  assert!(members.contains("bob\towner"), "{members}");
  ok(&env.run(&["org", "member", "remove", "acme", "bob"]).await);
  assert!(!ok(&env.run(&["org", "members", "acme"]).await).contains("bob"));

  let refused = failed(&env.run(&["org", "member", "remove", "acme", "alice"]).await);
  assert!(refused.contains("without an owner"), "{refused}");
  let invalid = failed(
    &env
      .run(&["org", "member", "set", "acme", "alice", "admin"])
      .await,
  );
  assert!(invalid.contains("unknown organization role"), "{invalid}");
}

#[tokio::test]
async fn project_and_audit_commands() {
  let env = common::start("project-commands").await;
  env.sign_in("alice").await;
  ok(&env.run(&["org", "create", "acme", "Acme"]).await);
  admit(&env, "acme", "bob");

  assert!(ok(&env.run(&["project", "create", "acme/demo", "Demo"]).await).contains("acme/demo"));
  assert_eq!(
    ok(&env.run(&["project", "list"]).await).trim(),
    "acme/demo\towner\tDemo"
  );
  assert_eq!(
    ok(&env.run(&["project", "list", "acme"]).await).trim(),
    "acme/demo\towner\tDemo"
  );
  ok(
    &env
      .run(&["project", "member", "set", "acme/demo", "bob", "editor"])
      .await,
  );
  assert!(ok(&env.run(&["project", "members", "acme/demo"]).await).contains("bob\teditor"));
  ok(
    &env
      .run(&["project", "member", "remove", "acme/demo", "bob"])
      .await,
  );
  assert!(!ok(&env.run(&["project", "members", "acme/demo"]).await).contains("bob"));
  ok(
    &env
      .run(&[
        "project",
        "set-repo",
        "acme/demo",
        "https://github.com/acme/demo",
      ])
      .await,
  );
  assert!(
    ok(&env.run(&["project", "show", "acme/demo"]).await).contains("https://github.com/acme/demo")
  );
  ok(&env.run(&["project", "clear-repo", "acme/demo"]).await);
  assert!(!ok(&env.run(&["project", "show", "acme/demo"]).await).contains("github.com"));
  let bad = failed(
    &env
      .run(&["project", "set-repo", "acme/demo", "ftp://x"])
      .await,
  );
  assert!(bad.contains("repository URL"), "{bad}");

  let audit = ok(&env.run(&["audit", "list", "acme"]).await);
  assert!(audit.lines().count() >= 6, "{audit}");
  assert!(
    audit.lines().next().unwrap().contains("project.repo"),
    "{audit}"
  );
  let limited = ok(&env.run(&["audit", "list", "acme", "--limit", "2"]).await);
  assert_eq!(limited.lines().count(), 2);
  let project_only = ok(
    &env
      .run(&["audit", "list", "acme", "--project", "demo"])
      .await,
  );
  assert!(
    project_only.lines().all(|l| l.contains("acme/demo")),
    "{project_only}"
  );

  let missing = failed(&env.run(&["project", "show", "acme/missing"]).await);
  assert!(missing.contains("not found"), "{missing}");
}
