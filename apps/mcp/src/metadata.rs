use axum::Json;
use axum::extract::State;
use serde_json::{Value, json};

use crate::state::AppState;

pub async fn protected_resource(State(state): State<AppState>) -> Json<Value> {
  Json(json!({
    "resource": state.config.resource,
    "authorization_servers": [state.config.authorization_server],
    "scopes_supported": state.config.scopes,
    "bearer_methods_supported": ["header"],
    "resource_name": "sdd-vault",
  }))
}
