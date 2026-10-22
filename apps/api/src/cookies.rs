use axum::http::HeaderMap;
use axum::http::header::COOKIE;

pub const SESSION_COOKIE: &str = "__Host-spec_store_session";
pub const LANGUAGE_COOKIE: &str = "spec_store_lang";
pub const REQUEST_HEADER: &str = "x-spec-store-request";

pub fn cookie(headers: &HeaderMap, name: &str) -> Option<String> {
  headers
    .get_all(COOKIE)
    .iter()
    .filter_map(|value| value.to_str().ok())
    .flat_map(|value| value.split(';'))
    .filter_map(|pair| pair.trim().split_once('='))
    .find(|(key, _)| *key == name)
    .map(|(_, value)| value.to_string())
}

pub fn language(headers: &HeaderMap) -> &'static str {
  match cookie(headers, LANGUAGE_COOKIE).as_deref() {
    Some("es") => "es",
    _ => "en",
  }
}

pub fn session_cookie(secret: &str) -> String {
  format!("{SESSION_COOKIE}={secret}; Path=/; Secure; HttpOnly; SameSite=Strict")
}

pub fn cleared_session_cookie() -> String {
  format!("{SESSION_COOKIE}=; Path=/; Secure; HttpOnly; SameSite=Strict; Max-Age=0")
}

#[cfg(test)]
mod tests {
  use axum::http::HeaderValue;

  use super::*;

  #[test]
  fn reads_named_cookies_and_the_language() {
    let mut headers = HeaderMap::new();
    headers.append(COOKIE, HeaderValue::from_static("a=1; spec_store_lang=es"));
    headers.append(
      COOKIE,
      HeaderValue::from_static("__Host-spec_store_session=abc"),
    );
    assert_eq!(cookie(&headers, SESSION_COOKIE).as_deref(), Some("abc"));
    assert_eq!(language(&headers), "es");
    assert_eq!(language(&HeaderMap::new()), "en");
  }
}
