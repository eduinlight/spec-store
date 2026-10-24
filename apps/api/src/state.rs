use std::sync::Arc;

use sdd_vault_core::clients::{CachedResolver, ClientNameResolver, KeycloakConsentNames};
use sdd_vault_core::crypto::{CryptoError, Sealer};
use sdd_vault_core::identity::Verifier;
use sdd_vault_core::keycloak::{KeycloakAdmin, OidcClient};
use sdd_vault_core::model::StoreResult;
use sdd_vault_core::store::Store;

use crate::error::ApiError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApiConfig {
  pub allowed_hosts: Vec<String>,
  pub audience: String,
  pub app_url: String,
  pub landing_url: String,
  pub app_client_id: String,
  pub app_client_secret: String,
  pub session_key: String,
  pub trusted_proxy_hops: usize,
}

impl ApiConfig {
  pub fn invitation_base_url(&self) -> String {
    format!("{}/invitations", self.app_url.trim_end_matches('/'))
  }

  pub fn callback_url(&self) -> String {
    format!("{}/auth/callback", self.app_url.trim_end_matches('/'))
  }
}

#[derive(Clone)]
pub struct AppState {
  pub store: Store,
  pub verifier: Arc<Verifier>,
  pub config: Arc<ApiConfig>,
  pub keycloak_admin: Option<Arc<KeycloakAdmin>>,
  pub sealer: Arc<Sealer>,
  pub oidc: Arc<OidcClient>,
  pub client_names: Option<Arc<dyn ClientNameResolver>>,
}

impl AppState {
  pub fn try_new(
    store: Store,
    verifier: Arc<Verifier>,
    config: ApiConfig,
  ) -> Result<AppState, CryptoError> {
    let sealer = Sealer::from_base64(&config.session_key)?;
    let oidc = OidcClient::new(
      &verifier.config().issuer,
      &config.app_client_id,
      &config.app_client_secret,
    );
    Ok(AppState {
      store,
      verifier,
      config: Arc::new(config),
      keycloak_admin: None,
      sealer: Arc::new(sealer),
      oidc: Arc::new(oidc),
      client_names: None,
    })
  }

  pub fn new(store: Store, verifier: Arc<Verifier>, config: ApiConfig) -> AppState {
    AppState::try_new(store, verifier, config).expect("a valid SESSION_SEALING_KEY")
  }

  pub fn with_keycloak_admin(mut self, admin: Arc<KeycloakAdmin>) -> AppState {
    let names = KeycloakConsentNames::new(admin.clone());
    self.client_names = Some(Arc::new(CachedResolver::new(Arc::new(names))));
    self.keycloak_admin = Some(admin);
    self
  }

  pub fn with_client_names(mut self, names: Arc<dyn ClientNameResolver>) -> AppState {
    self.client_names = Some(names);
    self
  }

  pub fn admin(&self) -> Result<Arc<KeycloakAdmin>, ApiError> {
    self.keycloak_admin.clone().ok_or_else(|| {
      ApiError::Store(sdd_vault_core::model::StoreError::Unavailable(
        "the identity provider administration".into(),
      ))
    })
  }

  pub async fn blocking<T, F>(&self, work: F) -> Result<T, ApiError>
  where
    T: Send + 'static,
    F: FnOnce(&Store) -> StoreResult<T> + Send + 'static,
  {
    let store = self.store.clone();
    tokio::task::spawn_blocking(move || work(&store))
      .await
      .map_err(|e| ApiError::Internal(e.to_string()))?
      .map_err(ApiError::from)
  }
}
