use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde_json::json;

use crate::state::AppState;

pub async fn live() -> &'static str {
  "ok"
}

fn word(ok: bool) -> &'static str {
  if ok { "ok" } else { "unavailable" }
}

pub async fn ready(State(state): State<AppState>) -> impl IntoResponse {
  let database = state.blocking(|store| store.ping()).await.is_ok();
  let identity_provider = state.verifier.reachable().await;
  let status = if database && identity_provider {
    StatusCode::OK
  } else {
    StatusCode::SERVICE_UNAVAILABLE
  };
  (
    status,
    Json(json!({ "database": word(database), "identity_provider": word(identity_provider) })),
  )
}
