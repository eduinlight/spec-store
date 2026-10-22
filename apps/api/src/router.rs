use axum::Router;
use axum::extract::DefaultBodyLimit;
use axum::middleware;
use axum::routing::{get, post};
use utoipa::openapi::OpenApi;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::bff;
use crate::health::{live, ready};
use crate::hosts::check_host;
use crate::routes::{
  audit, clients, documents, history, invitations, me, members, organizations, projects, specs,
  transfer,
};
use crate::state::AppState;

pub const MAX_REQUEST_BODY_BYTES: usize = 2 * 1024 * 1024;
pub const MAX_IMPORT_BODY_BYTES: usize = 128 * 1024 * 1024;

fn documented() -> OpenApiRouter<AppState> {
  let (import_schemas, import_paths, import_router) = routes!(transfer::import);
  OpenApiRouter::new()
    .routes(routes!(me::me))
    .routes(routes!(clients::list_clients))
    .routes(routes!(clients::revoke_client))
    .routes(routes!(invitations::accept_pending))
    .routes(routes!(invitations::decline_pending))
    .routes(routes!(organizations::list, organizations::create))
    .routes(routes!(organizations::read))
    .routes(routes!(members::list))
    .routes(routes!(members::set_role, members::remove))
    .routes(routes!(invitations::list, invitations::create))
    .routes(routes!(invitations::revoke))
    .routes(routes!(invitations::resend))
    .routes(routes!(projects::list, projects::create))
    .routes(routes!(audit::organization))
    .routes(routes!(audit::global))
    .routes(routes!(projects::read))
    .routes(routes!(projects::members))
    .routes(routes!(projects::set_member, projects::remove_member))
    .routes(routes!(
      projects::set_repository,
      projects::clear_repository
    ))
    .routes(routes!(audit::project))
    .routes(routes!(specs::status))
    .routes(routes!(specs::list))
    .routes(routes!(specs::read))
    .routes(routes!(specs::tasks))
    .routes(routes!(specs::markers))
    .routes(routes!(documents::read))
    .routes(routes!(documents::constitution))
    .routes(routes!(history::list))
    .routes(routes!(history::read))
    .routes(routes!(history::diff))
    .routes((
      import_schemas,
      import_paths,
      import_router.layer(DefaultBodyLimit::max(MAX_IMPORT_BODY_BYTES)),
    ))
    .routes(routes!(transfer::export))
    .routes(routes!(invitations::mine))
    .routes(routes!(invitations::preview))
    .routes(routes!(invitations::accept))
    .routes(routes!(invitations::decline))
    .routes(routes!(bff::logout))
}

pub fn openapi() -> OpenApi {
  let (_, mut api) = documented().split_for_parts();
  api.info.title = "spec-store API".into();
  api.info.version = env!("CARGO_PKG_VERSION").into();
  api.info.description = Some("JSON API of spec-store for the browser app and the CLI".into());
  api
}

pub fn router(state: AppState) -> Router {
  let (api, _) = documented().split_for_parts();
  api
    .route("/healthz", get(live))
    .route("/readyz", get(ready))
    .route("/auth/login", get(bff::login))
    .route("/auth/callback", get(bff::callback))
    .route("/auth/backchannel-logout", post(bff::backchannel_logout))
    .layer(middleware::from_fn_with_state(state.clone(), check_host))
    .layer(DefaultBodyLimit::max(MAX_REQUEST_BODY_BYTES))
    .with_state(state)
}
