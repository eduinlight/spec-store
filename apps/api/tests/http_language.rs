mod common;

use axum::http::StatusCode;
use serde_json::json;

#[tokio::test]
async fn choosing_a_language_sets_the_shared_cookie_and_the_identity_provider_locale() {
  let app = common::app().await;
  let response = app
    .json(
      "PUT",
      "/api/v1/me/language",
      "alice",
      json!({"language": "es"}),
    )
    .await;
  assert_eq!(response.status(), StatusCode::NO_CONTENT);
  let cookie = response
    .headers()
    .get_all("set-cookie")
    .iter()
    .map(|v| v.to_str().unwrap().to_string())
    .find(|c| c.starts_with("sdd_vault_lang="))
    .expect("language cookie");
  assert!(cookie.starts_with("sdd_vault_lang=es"), "{cookie}");
  assert!(cookie.contains("Domain=example.test"), "{cookie}");
  assert!(cookie.contains("Max-Age=31536000"), "{cookie}");
  let attributes = app
    .keycloak
    .user_attributes("subject-alice")
    .expect("attributes written");
  assert_eq!(attributes["locale"], json!(["es"]));
  assert_eq!(
    attributes["keep"],
    json!(["this"]),
    "other attributes are kept"
  );
}

#[tokio::test]
async fn unsupported_languages_are_refused() {
  let app = common::app().await;
  let response = app
    .json(
      "PUT",
      "/api/v1/me/language",
      "alice",
      json!({"language": "fr"}),
    )
    .await;
  assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn the_switch_from_a_browser_session_also_reaches_the_identity_provider() {
  let app = common::app().await;
  let cookie = app.browser_sign_in("bea").await;
  let response = app
    .browser(
      "PUT",
      "/api/v1/me/language",
      &cookie,
      true,
      Some(json!({"language": "es"})),
    )
    .await;
  assert_eq!(response.status(), StatusCode::NO_CONTENT);
  assert_eq!(
    app.keycloak.user_attributes("subject-bea").unwrap()["locale"],
    json!(["es"])
  );
}
