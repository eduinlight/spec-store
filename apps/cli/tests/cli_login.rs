mod common;

#[tokio::test]
async fn login_shows_a_code_and_stores_credentials_after_approval() {
  let env = common::start("login").await;
  let output = common::login_as(&env, "alice").await;
  let stdout = common::text(&output.stdout);
  assert!(
    output.status.success(),
    "stdout: {stdout}\nstderr: {}",
    common::text(&output.stderr)
  );
  assert!(
    stdout.contains(&format!("{}/device", env.keycloak.issuer())),
    "{stdout}"
  );
  assert!(stdout.contains("CODE-"), "{stdout}");
  assert!(stdout.contains("Signed in as alice"), "{stdout}");
  let file = env.credentials_file();
  assert!(file.exists());
  assert_eq!(common::file_mode(&file), 0o600);
  let stored = std::fs::read_to_string(&file).unwrap();
  assert!(stored.contains("refresh-alice"), "{stored}");

  let whoami = env.run(&["whoami"]).await;
  assert!(whoami.status.success(), "{}", common::text(&whoami.stderr));
  assert!(common::text(&whoami.stdout).contains("alice"));
}

#[tokio::test]
async fn login_times_out_without_approval() {
  let env = common::start("login-timeout").await;
  env.keycloak.set_device_lifespan(2);
  let output = env.run(&["login"]).await;
  assert_eq!(output.status.code(), Some(1));
  let stderr = common::text(&output.stderr);
  assert!(stderr.contains("timed out"), "{stderr}");
  assert!(!env.credentials_file().exists());
}

#[tokio::test]
async fn logout_ends_the_session_and_removes_the_credentials() {
  let env = common::start("logout").await;
  env.sign_in("alice").await;
  let stored = std::fs::read_to_string(env.credentials_file()).unwrap();
  let output = env.run(&["logout"]).await;
  assert!(output.status.success(), "{}", common::text(&output.stderr));
  assert!(!env.credentials_file().exists());
  let revoked = env.keycloak.revoked_refresh_tokens();
  assert_eq!(revoked.len(), 1);
  assert!(stored.contains(&revoked[0]));
  let whoami = env.run(&["whoami"]).await;
  assert_eq!(whoami.status.code(), Some(1));
  assert!(common::text(&whoami.stderr).contains("sddvault login"));
}

#[tokio::test]
async fn commands_without_credentials_ask_to_sign_in() {
  let env = common::start("no-credentials").await;
  let output = env.run(&["org", "list"]).await;
  assert_eq!(output.status.code(), Some(1));
  assert!(output.stdout.is_empty());
  assert!(common::text(&output.stderr).contains("sddvault login"));
}
