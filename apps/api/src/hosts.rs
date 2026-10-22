use axum::body::Body;
use axum::extract::{Request, State};
use axum::http::StatusCode;
use axum::http::header::HOST;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

use crate::state::AppState;

pub const LOOPBACK_HOSTS: [&str; 3] = ["localhost", "127.0.0.1", "[::1]"];
pub const UNCHECKED_PATHS: [&str; 2] = ["/healthz", "/readyz"];

fn without_port(host: &str) -> &str {
  if host.starts_with('[') {
    return host
      .split(']')
      .next()
      .map_or(host, |h| &host[..h.len() + 1]);
  }
  host.rsplit_once(':').map_or(host, |(name, _)| name)
}

pub fn host_allowed(allowed: &[String], host: &str) -> bool {
  let name = without_port(host);
  if allowed.is_empty() {
    return LOOPBACK_HOSTS.contains(&name);
  }
  allowed
    .iter()
    .any(|entry| entry.eq_ignore_ascii_case(host) || entry.eq_ignore_ascii_case(name))
}

pub async fn check_host(
  State(state): State<AppState>,
  request: Request<Body>,
  next: Next,
) -> Response {
  if UNCHECKED_PATHS.contains(&request.uri().path()) {
    return next.run(request).await;
  }
  let host = request
    .headers()
    .get(HOST)
    .and_then(|v| v.to_str().ok())
    .or_else(|| request.uri().authority().map(|a| a.as_str()))
    .unwrap_or_default();
  if host_allowed(&state.config.allowed_hosts, host) {
    next.run(request).await
  } else {
    (StatusCode::MISDIRECTED_REQUEST, "unknown host").into_response()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn matches_hosts_with_and_without_ports() {
    let allowed = vec!["api.example.test".to_string()];
    assert!(host_allowed(&allowed, "api.example.test"));
    assert!(host_allowed(&allowed, "API.example.test:443"));
    assert!(!host_allowed(&allowed, "api.example.test.evil"));
    assert!(host_allowed(&[], "localhost:4000"));
    assert!(host_allowed(&[], "[::1]:4000"));
    assert!(!host_allowed(&[], "example.test"));
  }
}
