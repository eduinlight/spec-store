mod common;

use axum::body::Body;
use axum::http::Request;
use axum::http::StatusCode;

#[tokio::test]
async fn login_redirects_to_the_identity_provider_with_pkce_and_the_chosen_language() {
  let app = common::app().await;
  let response = app
    .start_login("?return_to=%2Fprojects%2Facme", Some("sdd_vault_lang=es"))
    .await;
  assert_eq!(response.status(), StatusCode::FOUND);
  let url = common::location(&response);
  assert!(
    url.starts_with(&format!(
      "{}/protocol/openid-connect/auth?",
      app.keycloak.issuer()
    )),
    "{url}"
  );
  assert_eq!(
    common::query_param(&url, "client_id").as_deref(),
    Some("sdd-vault-app")
  );
  assert_eq!(
    common::query_param(&url, "response_type").as_deref(),
    Some("code")
  );
  assert_eq!(
    common::query_param(&url, "code_challenge_method").as_deref(),
    Some("S256")
  );
  assert_eq!(
    common::query_param(&url, "ui_locales").as_deref(),
    Some("es")
  );
  assert_eq!(
    common::query_param(&url, "redirect_uri").as_deref(),
    Some("https://app.example.test/auth/callback")
  );
  assert!(common::query_param(&url, "state").unwrap().len() >= 32);
  assert!(common::query_param(&url, "code_challenge").unwrap().len() >= 43);

  let register = app.start_login("?action=register", None).await;
  let url = common::location(&register);
  assert!(
    url.contains("/protocol/openid-connect/registrations?"),
    "{url}"
  );
  assert_eq!(
    common::query_param(&url, "ui_locales").as_deref(),
    Some("en")
  );
}

#[tokio::test]
async fn callback_creates_a_hardened_session_and_returns_to_the_requested_screen() {
  let app = common::app().await;
  let started = app.start_login("?return_to=%2Fprojects%2Facme", None).await;
  let finished = app
    .complete_login("alice", &common::location(&started))
    .await;
  assert_eq!(finished.status(), StatusCode::FOUND);
  assert_eq!(common::location(&finished), "/projects/acme");
  let cookie = common::set_cookie(&finished).expect("session cookie");
  for attribute in ["Secure", "HttpOnly", "SameSite=Strict", "Path=/"] {
    assert!(cookie.contains(attribute), "{cookie}");
  }
  assert!(!cookie.contains("Domain"), "{cookie}");
  let pair = cookie.split(';').next().unwrap();
  let me = app.browser("GET", "/api/v1/me", pair, false, None).await;
  assert_eq!(me.status(), StatusCode::OK);
  assert_eq!(common::body_json(me).await["handle"], "alice");

  let state = common::query_param(&common::location(&started), "state").unwrap();
  let replay = app
    .send(
      Request::get(format!("/auth/callback?code=whatever&state={state}"))
        .header("host", "app.example.test")
        .body(Body::empty())
        .unwrap(),
    )
    .await;
  assert_eq!(replay.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn return_to_never_leaves_the_app() {
  let app = common::app().await;
  for unsafe_target in [
    "https%3A%2F%2Fevil.example",
    "%2F%2Fevil.example",
    "javascript%3Aalert(1)",
  ] {
    let started = app
      .start_login(&format!("?return_to={unsafe_target}"), None)
      .await;
    let finished = app
      .complete_login("alice", &common::location(&started))
      .await;
    assert_eq!(common::location(&finished), "/", "{unsafe_target}");
  }
}

#[tokio::test]
async fn logout_ends_the_session_and_hands_back_the_identity_provider_logout_url() {
  let app = common::app().await;
  let cookie = app.browser_sign_in("alice").await;
  let response = app
    .browser(
      "POST",
      "/auth/logout",
      &format!("{cookie}; sdd_vault_lang=es"),
      true,
      None,
    )
    .await;
  assert_eq!(response.status(), StatusCode::OK);
  let cleared = common::set_cookie(&response).expect("cleared cookie");
  assert!(cleared.contains("Max-Age=0"), "{cleared}");
  let body = common::body_json(response).await;
  let url = body["logout_url"].as_str().unwrap();
  assert!(
    url.starts_with(&format!(
      "{}/protocol/openid-connect/logout?",
      app.keycloak.issuer()
    )),
    "{url}"
  );
  assert!(common::query_param(url, "id_token_hint").is_some());
  assert_eq!(
    common::query_param(url, "client_id").as_deref(),
    Some("sdd-vault-app")
  );
  assert_eq!(
    common::query_param(url, "post_logout_redirect_uri").as_deref(),
    Some(common::LANDING_URL)
  );
  assert_eq!(
    common::query_param(url, "ui_locales").as_deref(),
    Some("es")
  );
  let me = app.browser("GET", "/api/v1/me", &cookie, false, None).await;
  assert_eq!(me.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn back_channel_logout_ends_the_matching_sessions() {
  let app = common::app().await;
  let alice = app.browser_sign_in("alice").await;
  let bob = app.browser_sign_in("bob").await;
  let token = app.keycloak.mint_logout_token("alice");
  let response = app
    .send(
      Request::post("/auth/backchannel-logout")
        .header("host", "api.example.test")
        .header("content-type", "application/x-www-form-urlencoded")
        .body(Body::from(format!("logout_token={token}")))
        .unwrap(),
    )
    .await;
  assert_eq!(response.status(), StatusCode::OK);
  assert_eq!(
    app
      .browser("GET", "/api/v1/me", &alice, false, None)
      .await
      .status(),
    StatusCode::UNAUTHORIZED
  );
  assert_eq!(
    app
      .browser("GET", "/api/v1/me", &bob, false, None)
      .await
      .status(),
    StatusCode::OK
  );
  let forged = app
    .send(
      Request::post("/auth/backchannel-logout")
        .header("host", "api.example.test")
        .header("content-type", "application/x-www-form-urlencoded")
        .body(Body::from("logout_token=not-a-token"))
        .unwrap(),
    )
    .await;
  assert_eq!(forged.status(), StatusCode::BAD_REQUEST);
}
