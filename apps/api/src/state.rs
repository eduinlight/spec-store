use std::sync::Arc;

use spec_store_core::identity::Verifier;
use spec_store_core::model::StoreResult;
use spec_store_core::store::Store;

use crate::error::ApiError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApiConfig {
  pub allowed_hosts: Vec<String>,
  pub audience: String,
}

#[derive(Clone)]
pub struct AppState {
  pub store: Store,
  pub verifier: Arc<Verifier>,
  pub config: Arc<ApiConfig>,
}

impl AppState {
  pub fn new(store: Store, verifier: Arc<Verifier>, config: ApiConfig) -> AppState {
    AppState {
      store,
      verifier,
      config: Arc::new(config),
    }
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
