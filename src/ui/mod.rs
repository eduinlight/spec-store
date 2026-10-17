pub mod cookies;
pub mod diff;
pub mod html;
pub mod pages;
pub mod render;

use axum::Router;
use axum::extract::{Request, State};
use axum::http::header::{CONTENT_TYPE, HOST, SET_COOKIE};
use axum::http::{HeaderValue, StatusCode};
use axum::middleware::{self, Next};
use axum::response::{Html, IntoResponse, Redirect, Response};
use axum::routing::{get, post};

use crate::model::{Caller, StoreError};
use crate::store::Store;
use crate::ui::cookies::{clear_session_cookie, session_from_headers};
use crate::ui::html::{escape, layout, link};

#[derive(Debug, Clone, Default)]
pub struct UiConfig {
  pub secure_cookies: bool,
  pub allowed_hosts: Vec<String>,
}

#[derive(Clone)]
pub struct UiState {
  pub store: Store,
  pub config: UiConfig,
}

#[derive(Debug, Clone)]
pub struct Viewer {
  pub caller: Caller,
  pub token_id: i64,
}

#[derive(Debug)]
pub enum UiError {
  NotFound,
  NotAllowed,
  BadRequest(String),
  Internal,
}

impl From<StoreError> for UiError {
  fn from(error: StoreError) -> Self {
    match error {
      StoreError::Unauthenticated | StoreError::Forbidden | StoreError::NotFound(_) => {
        UiError::NotFound
      }
      StoreError::Validation(m) | StoreError::Malformed(m) => UiError::BadRequest(m),
      StoreError::Conflict { .. } => UiError::BadRequest("the document changed meanwhile".into()),
      StoreError::Database(e) => {
        tracing::error!(error = %e, "database error while rendering the UI");
        UiError::Internal
      }
    }
  }
}

impl IntoResponse for UiError {
  fn into_response(self) -> Response {
    let (status, title, message) = match self {
      UiError::NotFound => (
        StatusCode::NOT_FOUND,
        "Not found",
        "There is nothing at this address, or you have no access to it.".to_string(),
      ),
      UiError::NotAllowed => (
        StatusCode::FORBIDDEN,
        "Not allowed",
        "Your role does not allow this page.".to_string(),
      ),
      UiError::BadRequest(message) => (StatusCode::BAD_REQUEST, "Bad request", message),
      UiError::Internal => (
        StatusCode::INTERNAL_SERVER_ERROR,
        "Server error",
        "Something went wrong on the server.".to_string(),
      ),
    };
    let body = format!(
      "<h1>{}</h1><p>{}</p><p>{}</p>",
      escape(title),
      escape(&message),
      link("/ui/", "Back to projects")
    );
    (status, Html(layout(title, None, &body))).into_response()
  }
}

pub type UiResult = Result<Response, UiError>;

pub fn ui_router(store: Store, config: UiConfig) -> Router {
  let state = UiState { store, config };
  let public = Router::new()
    .route(
      "/ui/login",
      get(pages::login::form).post(pages::login::submit),
    )
    .route("/ui/logout", post(pages::login::logout))
    .route("/ui/style.css", get(stylesheet));
  let protected = Router::new()
    .route("/ui/", get(pages::projects::index))
    .route("/ui/profile", get(pages::profile::show))
    .route("/ui/p/{slug}", get(pages::projects::show))
    .route("/ui/p/{slug}/constitution", get(pages::specs::constitution))
    .route("/ui/p/{slug}/specs/{number}", get(pages::specs::spec))
    .route(
      "/ui/p/{slug}/specs/{number}/{kind}",
      get(pages::specs::document),
    )
    .route("/ui/{*rest}", get(not_found))
    .route_layer(middleware::from_fn_with_state(
      state.clone(),
      require_session,
    ));
  let ui = public
    .merge(protected)
    .route_layer(middleware::from_fn_with_state(state.clone(), check_host));
  Router::new()
    .route("/", get(|| async { Redirect::to("/ui/") }))
    .merge(ui)
    .with_state(state)
}

async fn not_found() -> UiError {
  UiError::NotFound
}

async fn stylesheet() -> Response {
  (
    [(CONTENT_TYPE, "text/css; charset=utf-8")],
    include_str!("style.css"),
  )
    .into_response()
}

pub async fn check_host(State(state): State<UiState>, request: Request, next: Next) -> Response {
  let host = request
    .headers()
    .get(HOST)
    .and_then(|v| v.to_str().ok())
    .map(str::trim)
    .unwrap_or_default();
  let allowed = if state.config.allowed_hosts.is_empty() {
    is_loopback_host(host)
  } else {
    state.config.allowed_hosts.iter().any(|h| h == host)
  };
  if allowed {
    next.run(request).await
  } else {
    (StatusCode::MISDIRECTED_REQUEST, "host not allowed").into_response()
  }
}

fn is_loopback_host(host: &str) -> bool {
  let name = if let Some(rest) = host.strip_prefix('[') {
    rest.split(']').next().unwrap_or_default()
  } else {
    host.split(':').next().unwrap_or_default()
  };
  matches!(name, "localhost" | "127.0.0.1" | "::1")
}

pub async fn require_session(
  State(state): State<UiState>,
  mut request: Request,
  next: Next,
) -> Response {
  let cookie = session_from_headers(request.headers());
  if let Some(secret) = &cookie
    && let Ok((caller, token_id)) = state.store.session_caller(secret)
  {
    request.extensions_mut().insert(Viewer { caller, token_id });
    return next.run(request).await;
  }
  let target = request
    .uri()
    .path_and_query()
    .map(|pq| pq.as_str())
    .unwrap_or("/ui/");
  let mut response =
    Redirect::to(&format!("/ui/login?next={}", percent_encode(target))).into_response();
  if cookie.is_some() {
    set_cookie_header(
      &mut response,
      &clear_session_cookie(state.config.secure_cookies),
    );
  }
  response
}

pub fn set_cookie_header(response: &mut Response, value: &str) {
  if let Ok(header) = HeaderValue::from_str(value) {
    response.headers_mut().insert(SET_COOKIE, header);
  }
}

pub fn percent_encode(input: &str) -> String {
  let mut out = String::with_capacity(input.len() * 3);
  for byte in input.bytes() {
    match byte {
      b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => out.push(byte as char),
      _ => out.push_str(&format!("%{byte:02X}")),
    }
  }
  out
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn percent_encoding_keeps_unreserved_characters_only() {
    assert_eq!(
      percent_encode("/ui/p/demo?view=raw"),
      "%2Fui%2Fp%2Fdemo%3Fview%3Draw"
    );
    assert_eq!(percent_encode("abc-._~"), "abc-._~");
    assert_eq!(percent_encode("é"), "%C3%A9");
  }

  #[test]
  fn loopback_hosts_with_or_without_port() {
    for host in [
      "localhost",
      "localhost:7391",
      "127.0.0.1:80",
      "[::1]:7391",
      "[::1]",
    ] {
      assert!(is_loopback_host(host), "{host}");
    }
    for host in [
      "evil.example",
      "localhost.evil.example",
      "",
      "10.0.0.1:7391",
    ] {
      assert!(!is_loopback_host(host), "{host}");
    }
  }
}
