use axum::http::HeaderMap;
use axum::http::header::COOKIE;

pub const SESSION_COOKIE: &str = "__Host-sdd_vault_session";
pub const LANGUAGE_COOKIE: &str = "sdd_vault_lang";
pub const REQUEST_HEADER: &str = "x-sdd-vault-request";

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

pub const ONE_YEAR_SECONDS: u32 = 31_536_000;

pub fn parent_domain(url: &str) -> Option<String> {
  let host = url.split("://").nth(1)?.split(['/', ':']).next()?;
  if host == "localhost" || host.chars().all(|c| c.is_ascii_digit() || c == '.') {
    return None;
  }
  let labels: Vec<&str> = host.split('.').collect();
  (labels.len() >= 2).then(|| labels[labels.len() - 2..].join("."))
}

pub fn language_cookie(language: &str, app_url: &str) -> String {
  let mut cookie =
    format!("{LANGUAGE_COOKIE}={language}; Path=/; Max-Age={ONE_YEAR_SECONDS}; SameSite=Lax");
  if let Some(domain) = parent_domain(app_url) {
    cookie.push_str(&format!("; Domain={domain}"));
  }
  if app_url.starts_with("https://") {
    cookie.push_str("; Secure");
  }
  cookie
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
  fn language_cookies_cover_the_parent_domain() {
    assert_eq!(
      parent_domain("https://app-sddvault.eduindev.com").as_deref(),
      Some("eduindev.com")
    );
    assert_eq!(parent_domain("http://localhost:5173"), None);
    let cookie = language_cookie("es", "https://app-sddvault.eduindev.com");
    assert!(cookie.contains("Domain=eduindev.com") && cookie.contains("Secure"));
  }

  #[test]
  fn reads_named_cookies_and_the_language() {
    let mut headers = HeaderMap::new();
    headers.append(COOKIE, HeaderValue::from_static("a=1; sdd_vault_lang=es"));
    headers.append(
      COOKIE,
      HeaderValue::from_static("__Host-sdd_vault_session=abc"),
    );
    assert_eq!(cookie(&headers, SESSION_COOKIE).as_deref(), Some("abc"));
    assert_eq!(language(&headers), "es");
    assert_eq!(language(&HeaderMap::new()), "en");
  }
}
