mod common;

use axum::http::StatusCode;
use serde_json::json;

#[tokio::test]
async fn a_session_cookie_authenticates_reads() {
  let app = common::app().await;
  let cookie = app.browser_sign_in("alice").await;
  let me = app.browser("GET", "/api/v1/me", &cookie, false, None).await;
  assert_eq!(me.status(), StatusCode::OK);
  assert_eq!(
    app
      .browser(
        "GET",
        "/api/v1/me",
        "__Host-sdd_vault_session=forged",
        false,
        None
      )
      .await
      .status(),
    StatusCode::UNAUTHORIZED
  );
}

#[tokio::test]
async fn state_changing_cookie_requests_need_the_request_header() {
  let app = common::app().await;
  let cookie = app.browser_sign_in("alice").await;
  let without = app
    .browser(
      "POST",
      "/api/v1/orgs",
      &cookie,
      false,
      Some(json!({"slug": "acme", "name": "Acme"})),
    )
    .await;
  assert_eq!(without.status(), StatusCode::FORBIDDEN);
  let with = app
    .browser(
      "POST",
      "/api/v1/orgs",
      &cookie,
      true,
      Some(json!({"slug": "acme", "name": "Acme"})),
    )
    .await;
  assert_eq!(with.status(), StatusCode::CREATED);
}

fn expire_access(app: &common::TestApp) {
  app
    .state
    .store
    .with_conn(|conn| {
      use diesel::RunQueryDsl;
      diesel::sql_query(
        "UPDATE browser_sessions SET access_expires_at = now() - interval '1 second'",
      )
      .execute(conn)?;
      Ok(())
    })
    .unwrap();
}

#[tokio::test]
async fn an_expired_access_token_is_refreshed_transparently() {
  let app = common::app().await;
  let cookie = app.browser_sign_in("alice").await;
  expire_access(&app);
  let before = app.keycloak.refresh_grants();
  let me = app.browser("GET", "/api/v1/me", &cookie, false, None).await;
  assert_eq!(me.status(), StatusCode::OK);
  assert_eq!(app.keycloak.refresh_grants(), before + 1);
  let again = app.browser("GET", "/api/v1/me", &cookie, false, None).await;
  assert_eq!(again.status(), StatusCode::OK);
  assert_eq!(
    app.keycloak.refresh_grants(),
    before + 1,
    "the refreshed access stays valid"
  );
}

#[tokio::test]
async fn a_disabled_person_is_refused_at_the_next_refresh_and_the_session_ends() {
  let app = common::app().await;
  let cookie = app.browser_sign_in("alice").await;
  app.keycloak.disable("alice");
  expire_access(&app);
  assert_eq!(
    app
      .browser("GET", "/api/v1/me", &cookie, false, None)
      .await
      .status(),
    StatusCode::UNAUTHORIZED
  );
  let ended = app
    .state
    .store
    .with_conn(|conn| {
      use diesel::RunQueryDsl;
      #[derive(diesel::QueryableByName)]
      struct Count {
        #[diesel(sql_type = diesel::sql_types::BigInt)]
        count: i64,
      }
      Ok(
        diesel::sql_query(
          "SELECT count(*) AS count FROM browser_sessions WHERE ended_at IS NOT NULL",
        )
        .get_result::<Count>(conn)?
        .count,
      )
    })
    .unwrap();
  assert_eq!(ended, 1);
}

#[tokio::test]
async fn operators_signed_in_through_the_browser_keep_their_role() {
  let app = common::app().await;
  app.keycloak.make_operator("ops");
  let cookie = app.browser_sign_in("ops").await;
  let me = common::body_json(app.browser("GET", "/api/v1/me", &cookie, false, None).await).await;
  assert_eq!(me["is_operator"], true);
}
