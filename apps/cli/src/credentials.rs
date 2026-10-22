use std::fs;
use std::io::Write;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

pub const KEYRING_SERVICE: &str = "spec-store";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Credentials {
  pub issuer: String,
  pub api: String,
  pub access_token: String,
  pub refresh_token: String,
  pub expires_at: i64,
  pub handle: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum CredentialsError {
  #[error("cannot read or write the stored credentials: {0}")]
  Io(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backend {
  Keyring,
  File,
}

pub struct CredentialStore {
  backend: Backend,
  directory: PathBuf,
}

fn config_dir() -> PathBuf {
  if let Ok(dir) = std::env::var("SPEC_STORE_CONFIG_DIR") {
    return PathBuf::from(dir);
  }
  if let Ok(dir) = std::env::var("XDG_CONFIG_HOME") {
    return PathBuf::from(dir).join("spec-store");
  }
  let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
  PathBuf::from(home).join(".config").join("spec-store")
}

fn keyring_works() -> bool {
  keyring::Entry::new(KEYRING_SERVICE, "probe").is_ok() && keyring::Entry::store_status().is_ok()
}

impl CredentialStore {
  pub fn from_env() -> CredentialStore {
    let backend = match std::env::var("SPEC_STORE_CREDENTIALS").as_deref() {
      Ok("file") => Backend::File,
      Ok("keyring") => Backend::Keyring,
      _ if keyring_works() => Backend::Keyring,
      _ => Backend::File,
    };
    CredentialStore {
      backend,
      directory: config_dir(),
    }
  }

  pub fn backend(&self) -> Backend {
    self.backend
  }

  fn file(&self) -> PathBuf {
    self.directory.join("credentials.json")
  }

  fn account(issuer: &str) -> String {
    issuer.to_string()
  }

  pub fn load(&self, issuer: &str) -> Result<Option<Credentials>, CredentialsError> {
    let raw = match self.backend {
      Backend::File => match fs::read_to_string(self.file()) {
        Ok(raw) => Some(raw),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => return Err(CredentialsError::Io(e.to_string())),
      },
      Backend::Keyring => {
        let entry = keyring::Entry::new(KEYRING_SERVICE, &Self::account(issuer))
          .map_err(|e| CredentialsError::Io(e.to_string()))?;
        match entry.get_password() {
          Ok(raw) => Some(raw),
          Err(keyring::Error::NoEntry) => None,
          Err(e) => return Err(CredentialsError::Io(e.to_string())),
        }
      }
    };
    Ok(
      raw
        .and_then(|raw| serde_json::from_str::<Credentials>(&raw).ok())
        .filter(|c| c.issuer == issuer),
    )
  }

  pub fn save(&self, credentials: &Credentials) -> Result<(), CredentialsError> {
    let raw =
      serde_json::to_string(credentials).map_err(|e| CredentialsError::Io(e.to_string()))?;
    match self.backend {
      Backend::File => {
        fs::create_dir_all(&self.directory).map_err(|e| CredentialsError::Io(e.to_string()))?;
        let mut options = fs::OpenOptions::new();
        options.write(true).create(true).truncate(true);
        #[cfg(unix)]
        {
          use std::os::unix::fs::OpenOptionsExt;
          options.mode(0o600);
        }
        let mut file = options
          .open(self.file())
          .map_err(|e| CredentialsError::Io(e.to_string()))?;
        #[cfg(unix)]
        {
          use std::os::unix::fs::PermissionsExt;
          fs::set_permissions(self.file(), fs::Permissions::from_mode(0o600))
            .map_err(|e| CredentialsError::Io(e.to_string()))?;
        }
        file
          .write_all(raw.as_bytes())
          .map_err(|e| CredentialsError::Io(e.to_string()))
      }
      Backend::Keyring => keyring::Entry::new(KEYRING_SERVICE, &Self::account(&credentials.issuer))
        .and_then(|entry| entry.set_password(&raw))
        .map_err(|e| CredentialsError::Io(e.to_string())),
    }
  }

  pub fn remove(&self, issuer: &str) -> Result<(), CredentialsError> {
    match self.backend {
      Backend::File => match fs::remove_file(self.file()) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(CredentialsError::Io(e.to_string())),
      },
      Backend::Keyring => {
        let entry = keyring::Entry::new(KEYRING_SERVICE, &Self::account(issuer))
          .map_err(|e| CredentialsError::Io(e.to_string()))?;
        match entry.delete_credential() {
          Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
          Err(e) => Err(CredentialsError::Io(e.to_string())),
        }
      }
    }
  }
}
