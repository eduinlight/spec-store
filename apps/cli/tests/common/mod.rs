#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::Output;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use sdd_vault_api::{ApiConfig, AppState, router};
use sdd_vault_core::identity::{IdentityConfig, Verifier};
use sdd_vault_core::store::Store;
use sdd_vault_testkit::{FakeKeycloak, TestDb};
use tokio::process::Command;
use tokio::task::JoinHandle;

static COUNTER: AtomicUsize = AtomicUsize::new(0);

pub fn migrate(url: &str) {
  sdd_vault_core::db::open_with_size(url, 1).expect("migrate template");
}

pub struct TestEnv {
  pub keycloak: FakeKeycloak,
  pub api: String,
  pub store: Store,
  pub config_dir: PathBuf,
  pub db: TestDb,
  server: JoinHandle<()>,
}

impl Drop for TestEnv {
  fn drop(&mut self) {
    self.server.abort();
    let _ = std::fs::remove_dir_all(&self.config_dir);
  }
}

pub fn temp_dir(label: &str) -> PathBuf {
  let n = COUNTER.fetch_add(1, Ordering::SeqCst);
  let dir = std::env::temp_dir()
    .join("sdd-vault-cli-tests")
    .join(format!("{label}-{}-{n}", std::process::id()));
  if dir.exists() {
    std::fs::remove_dir_all(&dir).unwrap();
  }
  std::fs::create_dir_all(&dir).unwrap();
  dir
}

pub async fn start(label: &str) -> TestEnv {
  let keycloak = FakeKeycloak::start().await;
  let db = TestDb::new(migrate);
  let store = Store::open_with_size(&db.url, 4).expect("store");
  let verifier = Verifier::new(IdentityConfig {
    issuer: keycloak.issuer(),
    jwks_url: keycloak.jwks_url(),
  });
  let state = AppState::new(
    store.clone(),
    Arc::new(verifier),
    ApiConfig {
      allowed_hosts: Vec::new(),
      audience: "sdd-vault-api".into(),
      app_url: "https://app.example.test".into(),
      landing_url: "https://example.test".into(),
      app_client_id: "sdd-vault-app".into(),
      app_client_secret: "app-secret".into(),
      session_key: sdd_vault_core::crypto::Sealer::generate_key(),
      trusted_proxy_hops: 0,
    },
  );
  let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
  let address = listener.local_addr().unwrap();
  let server = tokio::spawn(async move {
    axum::serve(listener, router(state)).await.unwrap();
  });
  TestEnv {
    keycloak,
    api: format!("http://{address}"),
    store,
    config_dir: temp_dir(label),
    db,
    server,
  }
}

impl TestEnv {
  pub fn command(&self) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_sddvault"));
    command
      .env("SDD_VAULT_API", &self.api)
      .env("SDD_VAULT_ISSUER", self.keycloak.issuer())
      .env("SDD_VAULT_CLIENT_ID", "sdd-vault-cli")
      .env("SDD_VAULT_CONFIG_DIR", &self.config_dir)
      .env("SDD_VAULT_CREDENTIALS", "file")
      .kill_on_drop(true);
    command
  }

  pub async fn run(&self, args: &[&str]) -> Output {
    self.command().args(args).output().await.expect("run cli")
  }

  pub fn credentials_file(&self) -> PathBuf {
    self.config_dir.join("credentials.json")
  }

  pub async fn sign_in(&self, username: &str) {
    login_as(self, username).await;
  }

  pub fn use_identity(&self, username: &str) {
    let access = self.keycloak.mint(&sdd_vault_testkit::TokenSpec::new(
      username,
      "sdd-vault-api",
    ));
    let refresh = self.keycloak.issue_refresh_token(username);
    let now = std::time::SystemTime::now()
      .duration_since(std::time::UNIX_EPOCH)
      .unwrap()
      .as_secs() as i64;
    let credentials = serde_json::json!({
      "issuer": self.keycloak.issuer(),
      "api": self.api,
      "access_token": access,
      "refresh_token": refresh,
      "expires_at": now + 300,
      "handle": username,
    });
    std::fs::create_dir_all(&self.config_dir).unwrap();
    std::fs::write(self.credentials_file(), credentials.to_string()).unwrap();
  }

  pub fn person(&self, handle: &str) -> sdd_vault_core::model::Caller {
    self
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
      .unwrap()
  }
}

pub async fn login_as(env: &TestEnv, username: &str) -> Output {
  use tokio::io::{AsyncBufReadExt, BufReader};
  let mut child = env
    .command()
    .arg("login")
    .stdout(std::process::Stdio::piped())
    .stderr(std::process::Stdio::piped())
    .spawn()
    .expect("spawn login");
  let stdout = child.stdout.take().unwrap();
  let mut lines = BufReader::new(stdout).lines();
  let mut transcript = String::new();
  let mut approved = false;
  while let Some(line) = lines.next_line().await.unwrap() {
    transcript.push_str(&line);
    transcript.push('\n');
    if !approved && let Some(code) = line.split_whitespace().find(|w| w.starts_with("CODE-")) {
      assert!(
        env.keycloak.approve_device(code, username),
        "unknown code {code}"
      );
      approved = true;
    }
  }
  let mut output = child.wait_with_output().await.unwrap();
  output.stdout = transcript.into_bytes();
  output
}

pub fn text(bytes: &[u8]) -> String {
  String::from_utf8_lossy(bytes).to_string()
}

pub fn file_mode(path: &Path) -> u32 {
  use std::os::unix::fs::PermissionsExt;
  std::fs::metadata(path).unwrap().permissions().mode() & 0o777
}
