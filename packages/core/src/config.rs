use crate::identity::IdentityConfig;
use crate::keycloak::AdminConfig;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ConfigError {
  #[error("{0} must be set")]
  Missing(String),
  #[error("{name} is invalid: {detail}")]
  Invalid { name: String, detail: String },
}

pub type ConfigResult<T> = Result<T, ConfigError>;

type Lookup<'a> = Box<dyn Fn(&str) -> Option<String> + 'a>;

pub struct Env<'a> {
  lookup: Lookup<'a>,
}

impl<'a> Env<'a> {
  pub fn system() -> Env<'static> {
    Env {
      lookup: Box::new(|name| std::env::var(name).ok().filter(|v| !v.is_empty())),
    }
  }

  pub fn from_fn(lookup: impl Fn(&str) -> Option<String> + 'a) -> Env<'a> {
    Env {
      lookup: Box::new(lookup),
    }
  }

  pub fn optional(&self, name: &str) -> Option<String> {
    (self.lookup)(name)
  }

  pub fn required(&self, name: &str) -> ConfigResult<String> {
    self
      .optional(name)
      .ok_or_else(|| ConfigError::Missing(name.to_string()))
  }

  pub fn parsed<T: std::str::FromStr>(&self, name: &str, default: T) -> ConfigResult<T>
  where
    T::Err: std::fmt::Display,
  {
    match self.optional(name) {
      None => Ok(default),
      Some(value) => value.parse().map_err(|e: T::Err| ConfigError::Invalid {
        name: name.to_string(),
        detail: e.to_string(),
      }),
    }
  }

  pub fn list(&self, name: &str) -> Vec<String> {
    self
      .optional(name)
      .map(|v| {
        v.split(',')
          .map(str::trim)
          .filter(|s| !s.is_empty())
          .map(str::to_string)
          .collect()
      })
      .unwrap_or_default()
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatabaseSettings {
  pub url: String,
  pub pool_size: u32,
}

impl DatabaseSettings {
  pub fn from_env(env: &Env) -> ConfigResult<DatabaseSettings> {
    Ok(DatabaseSettings {
      url: env.required("DATABASE_URL")?,
      pool_size: env.parsed("DATABASE_POOL_SIZE", crate::db::POOL_SIZE)?,
    })
  }
}

pub fn identity_from_env(env: &Env) -> ConfigResult<IdentityConfig> {
  let issuer = env.required("KEYCLOAK_ISSUER")?;
  let mut config = IdentityConfig::for_issuer(&issuer);
  if let Some(jwks_url) = env.optional("KEYCLOAK_JWKS_URL") {
    config.jwks_url = jwks_url;
  }
  Ok(config)
}

pub fn split_issuer(issuer: &str) -> ConfigResult<(String, String)> {
  let issuer = issuer.trim_end_matches('/');
  issuer
    .rsplit_once("/realms/")
    .filter(|(base, realm)| !base.is_empty() && !realm.is_empty() && !realm.contains('/'))
    .map(|(base, realm)| (base.to_string(), realm.to_string()))
    .ok_or_else(|| ConfigError::Invalid {
      name: "KEYCLOAK_ISSUER".into(),
      detail: format!("{issuer:?} is not of the form <keycloak url>/realms/<realm>"),
    })
}

pub fn admin_from_env(env: &Env) -> ConfigResult<Option<AdminConfig>> {
  let Some(client_secret) = env.optional("KEYCLOAK_ADMIN_CLIENT_SECRET") else {
    return Ok(None);
  };
  let (public_base, realm) = split_issuer(&env.required("KEYCLOAK_ISSUER")?)?;
  Ok(Some(AdminConfig {
    base_url: env.optional("KEYCLOAK_INTERNAL_URL").unwrap_or(public_base),
    realm,
    client_id: env
      .optional("KEYCLOAK_ADMIN_CLIENT_ID")
      .unwrap_or_else(|| "sdd-vault-api-admin".into()),
    client_secret,
  }))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmtpSettings {
  pub host: String,
  pub port: u16,
  pub from: String,
  pub from_name: String,
  pub username: Option<String>,
  pub password: Option<String>,
  pub starttls: bool,
}

impl SmtpSettings {
  pub fn from_env(env: &Env) -> ConfigResult<SmtpSettings> {
    Ok(SmtpSettings {
      host: env.required("SMTP_HOST")?,
      port: env.parsed("SMTP_PORT", 587)?,
      from: env.required("SMTP_FROM")?,
      from_name: env
        .optional("SMTP_FROM_NAME")
        .unwrap_or_else(|| "sdd-vault".into()),
      username: env.optional("SMTP_USERNAME"),
      password: env.optional("SMTP_PASSWORD"),
      starttls: env.parsed("SMTP_STARTTLS", false)?,
    })
  }
}

#[cfg(test)]
mod tests {
  use std::collections::HashMap;

  use super::*;

  fn env(pairs: &[(&str, &str)]) -> HashMap<String, String> {
    pairs
      .iter()
      .map(|(k, v)| (k.to_string(), v.to_string()))
      .collect()
  }

  #[test]
  fn reads_required_parsed_and_listed_values() {
    let values = env(&[
      ("DATABASE_URL", "postgres://x/y"),
      ("DATABASE_POOL_SIZE", "7"),
      ("ALLOWED_HOSTS", "a.example, b.example ,"),
      ("KEYCLOAK_ISSUER", "https://auth.example/realms/sdd-vault/"),
    ]);
    let env = Env::from_fn(|k| values.get(k).cloned());
    assert_eq!(
      DatabaseSettings::from_env(&env).unwrap(),
      DatabaseSettings {
        url: "postgres://x/y".into(),
        pool_size: 7
      }
    );
    assert_eq!(env.list("ALLOWED_HOSTS"), vec!["a.example", "b.example"]);
    let identity = identity_from_env(&env).unwrap();
    assert_eq!(identity.issuer, "https://auth.example/realms/sdd-vault");
    assert_eq!(
      identity.jwks_url,
      "https://auth.example/realms/sdd-vault/protocol/openid-connect/certs"
    );
  }

  #[test]
  fn admin_settings_derive_from_the_issuer() {
    let values = env(&[
      ("KEYCLOAK_ISSUER", "https://auth.example/realms/sdd-vault"),
      ("KEYCLOAK_ADMIN_CLIENT_SECRET", "s"),
    ]);
    let env = Env::from_fn(|k| values.get(k).cloned());
    let admin = admin_from_env(&env).unwrap().unwrap();
    assert_eq!(admin.base_url, "https://auth.example");
    assert_eq!(admin.realm, "sdd-vault");
    assert_eq!(admin.client_id, "sdd-vault-api-admin");
    assert!(split_issuer("https://auth.example/sdd-vault").is_err());
  }

  #[test]
  fn missing_and_invalid_values_are_named() {
    let values = env(&[("DATABASE_URL", "x"), ("DATABASE_POOL_SIZE", "many")]);
    let env = Env::from_fn(|k| values.get(k).cloned());
    assert_eq!(
      identity_from_env(&env).unwrap_err(),
      ConfigError::Missing("KEYCLOAK_ISSUER".into())
    );
    assert!(matches!(
      DatabaseSettings::from_env(&env).unwrap_err(),
      ConfigError::Invalid { name, .. } if name == "DATABASE_POOL_SIZE"
    ));
  }
}
