use axum::Json;
use axum::http::StatusCode;
use axum::http::header::WWW_AUTHENTICATE;
use axum::response::{IntoResponse, Response};
use serde_json::json;
use spec_store_core::identity::IdentityError;
use spec_store_core::model::StoreError;

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
  #[error("authentication required")]
  Unauthenticated,
  #[error("{0}")]
  Store(#[from] StoreError),
  #[error("{0}")]
  Identity(#[from] IdentityError),
  #[error("internal error: {0}")]
  Internal(String),
}

pub fn error_code(error: &StoreError) -> (StatusCode, &'static str) {
  match error {
    StoreError::Unauthenticated => (StatusCode::UNAUTHORIZED, "unauthenticated"),
    StoreError::Forbidden => (StatusCode::FORBIDDEN, "forbidden"),
    StoreError::NotFound(_) => (StatusCode::NOT_FOUND, "not_found"),
    StoreError::Conflict { .. } => (StatusCode::CONFLICT, "conflict"),
    StoreError::Validation(_) | StoreError::Malformed(_) => {
      (StatusCode::UNPROCESSABLE_ENTITY, "validation")
    }
    StoreError::Unavailable(_) => (StatusCode::SERVICE_UNAVAILABLE, "unavailable"),
    StoreError::Database(_) | StoreError::Pool(_) | StoreError::Migration { .. } => {
      (StatusCode::INTERNAL_SERVER_ERROR, "internal")
    }
  }
}

fn unauthenticated() -> Response {
  (
    StatusCode::UNAUTHORIZED,
    [(WWW_AUTHENTICATE, "Bearer realm=\"spec-store\"")],
    Json(json!({ "code": "unauthenticated", "message": "authentication required" })),
  )
    .into_response()
}

impl IntoResponse for ApiError {
  fn into_response(self) -> Response {
    match self {
      ApiError::Unauthenticated => unauthenticated(),
      ApiError::Identity(IdentityError::Unavailable(detail)) => {
        tracing::warn!(%detail, "identity provider unavailable");
        (
          StatusCode::SERVICE_UNAVAILABLE,
          Json(json!({ "code": "unavailable", "message": "the identity provider is unavailable" })),
        )
          .into_response()
      }
      ApiError::Identity(_) => unauthenticated(),
      ApiError::Internal(detail) => {
        tracing::error!(%detail, "internal error");
        (
          StatusCode::INTERNAL_SERVER_ERROR,
          Json(json!({ "code": "internal", "message": "internal error" })),
        )
          .into_response()
      }
      ApiError::Store(error) => {
        let (status, code) = error_code(&error);
        if status == StatusCode::INTERNAL_SERVER_ERROR {
          tracing::error!(%error, "store error");
          return (
            status,
            Json(json!({ "code": code, "message": "internal error" })),
          )
            .into_response();
        }
        let mut body = json!({ "code": code, "message": error.to_string() });
        if let StoreError::Conflict { current_revision } = error {
          body["current_revision"] = json!(current_revision);
        }
        (status, Json(body)).into_response()
      }
    }
  }
}
