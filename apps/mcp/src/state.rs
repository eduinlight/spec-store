use std::sync::Arc;

use sdd_vault_core::clients::{CachedResolver, ClientNameResolver, KeycloakConsentNames};
use sdd_vault_core::identity::Verifier;
use sdd_vault_core::keycloak::KeycloakAdmin;
use sdd_vault_core::store::Store;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpConfig {
  pub allowed_hosts: Vec<String>,
  pub resource: String,
  pub authorization_server: String,
  pub scopes: Vec<String>,
  pub trusted_proxy_hops: usize,
}

impl McpConfig {
  pub fn resource_origin(&self) -> &str {
    let after_scheme = self.resource.find("://").map_or(0, |i| i + 3);
    match self.resource[after_scheme..].find('/') {
      Some(slash) => &self.resource[..after_scheme + slash],
      None => &self.resource,
    }
  }

  pub fn resource_path(&self) -> &str {
    &self.resource[self.resource_origin().len()..]
  }

  pub fn metadata_url(&self) -> String {
    format!(
      "{}/.well-known/oauth-protected-resource{}",
      self.resource_origin(),
      self.resource_path()
    )
  }
}

#[derive(Clone)]
pub struct AppState {
  pub store: Store,
  pub verifier: Arc<Verifier>,
  pub config: Arc<McpConfig>,
  pub client_names: Option<Arc<dyn ClientNameResolver>>,
}

impl AppState {
  pub fn new(store: Store, verifier: Arc<Verifier>, config: McpConfig) -> AppState {
    AppState {
      store,
      verifier,
      config: Arc::new(config),
      client_names: None,
    }
  }

  pub fn with_keycloak_admin(mut self, admin: Arc<KeycloakAdmin>) -> AppState {
    let names = KeycloakConsentNames::new(admin);
    self.client_names = Some(Arc::new(CachedResolver::new(Arc::new(names))));
    self
  }

  pub fn with_client_names(mut self, names: Arc<dyn ClientNameResolver>) -> AppState {
    self.client_names = Some(names);
    self
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn metadata_url_inserts_the_well_known_segment_before_the_path() {
    let config = McpConfig {
      allowed_hosts: Vec::new(),
      resource: "https://mcp-sddvault.eduindev.com/mcp".into(),
      authorization_server: "https://auth/realms/x".into(),
      scopes: Vec::new(),
      trusted_proxy_hops: 0,
    };
    assert_eq!(
      config.resource_origin(),
      "https://mcp-sddvault.eduindev.com"
    );
    assert_eq!(config.resource_path(), "/mcp");
    assert_eq!(
      config.metadata_url(),
      "https://mcp-sddvault.eduindev.com/.well-known/oauth-protected-resource/mcp"
    );
  }
}
