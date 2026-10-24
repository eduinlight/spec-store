use std::net::SocketAddr;

use axum::body::Body;
use axum::extract::{ConnectInfo, Request, State};
use axum::middleware::Next;
use axum::response::Response;
use sdd_vault_core::origin::client_origin;

use crate::state::AppState;

pub const FORWARDED_FOR: &str = "x-forwarded-for";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientOrigin(pub Option<String>);

pub async fn attach_origin(
  State(state): State<AppState>,
  mut request: Request<Body>,
  next: Next,
) -> Response {
  let peer = request
    .extensions()
    .get::<ConnectInfo<SocketAddr>>()
    .map(|ConnectInfo(address)| address.ip());
  let forwarded_for = request
    .headers()
    .get_all(FORWARDED_FOR)
    .iter()
    .filter_map(|value| value.to_str().ok())
    .collect::<Vec<_>>()
    .join(",");
  let origin = client_origin(
    peer,
    Some(forwarded_for.as_str()).filter(|header| !header.is_empty()),
    state.config.trusted_proxy_hops,
  );
  request.extensions_mut().insert(ClientOrigin(origin));
  next.run(request).await
}
