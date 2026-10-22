use axum::Json;
use serde::Serialize;

use crate::auth::Principal;

#[derive(Debug, Serialize)]
pub struct Me {
  pub handle: String,
  pub email: String,
  pub name: String,
  pub is_operator: bool,
}

pub async fn me(principal: Principal) -> Json<Me> {
  Json(Me {
    handle: principal.caller.handle,
    email: principal.identity.email,
    name: principal.identity.name,
    is_operator: principal.caller.is_operator,
  })
}
