use axum::http::HeaderMap;
use axum::http::header::COOKIE;

pub const SESSION_COOKIE: &str = "spec_store_session";
const MAX_AGE_SECONDS: i64 = 30 * 24 * 60 * 60;

pub fn session_from_headers(headers: &HeaderMap) -> Option<String> {
  headers
    .get_all(COOKIE)
    .iter()
    .filter_map(|value| value.to_str().ok())
    .flat_map(|value| value.split(';'))
    .filter_map(|pair| {
      let (name, value) = pair.trim().split_once('=')?;
      (name.trim() == SESSION_COOKIE && !value.trim().is_empty()).then(|| value.trim().to_string())
    })
    .next()
}

pub fn set_session_cookie(secret: &str, secure: bool) -> String {
  format!(
    "{SESSION_COOKIE}={secret}; Path=/; HttpOnly; SameSite=Lax; Max-Age={MAX_AGE_SECONDS}{}",
    secure_suffix(secure)
  )
}

pub fn clear_session_cookie(secure: bool) -> String {
  format!(
    "{SESSION_COOKIE}=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0{}",
    secure_suffix(secure)
  )
}

fn secure_suffix(secure: bool) -> &'static str {
  if secure { "; Secure" } else { "" }
}

#[cfg(test)]
mod tests {
  use super::*;
  use axum::http::{HeaderMap, HeaderValue, header::COOKIE};

  #[test]
  fn reads_the_session_cookie_among_others() {
    let mut headers = HeaderMap::new();
    headers.insert(
      COOKIE,
      HeaderValue::from_static("theme=dark; spec_store_session=sss_abc; other=1"),
    );
    assert_eq!(session_from_headers(&headers).as_deref(), Some("sss_abc"));
  }

  #[test]
  fn reads_from_a_second_cookie_header_and_tolerates_missing_or_empty() {
    let mut headers = HeaderMap::new();
    headers.append(COOKIE, HeaderValue::from_static("theme=dark"));
    headers.append(
      COOKIE,
      HeaderValue::from_static("spec_store_session=sss_xyz"),
    );
    assert_eq!(session_from_headers(&headers).as_deref(), Some("sss_xyz"));

    let mut none = HeaderMap::new();
    none.insert(COOKIE, HeaderValue::from_static("theme=dark"));
    assert_eq!(session_from_headers(&none), None);
    assert_eq!(session_from_headers(&HeaderMap::new()), None);

    let mut empty = HeaderMap::new();
    empty.insert(COOKIE, HeaderValue::from_static("spec_store_session="));
    assert_eq!(session_from_headers(&empty), None);
  }

  #[test]
  fn set_cookie_carries_the_protective_attributes() {
    let plain = set_session_cookie("sss_abc", false);
    assert!(plain.starts_with("spec_store_session=sss_abc;"));
    for attribute in ["Path=/", "HttpOnly", "SameSite=Lax", "Max-Age=2592000"] {
      assert!(plain.contains(attribute), "{plain} lacks {attribute}");
    }
    assert!(!plain.contains("Secure"));
    let secure = set_session_cookie("sss_abc", true);
    assert!(secure.contains("; Secure"));
  }

  #[test]
  fn clear_cookie_empties_the_value_and_expires_it() {
    let clear = clear_session_cookie(true);
    assert!(clear.starts_with("spec_store_session=;"));
    assert!(clear.contains("Max-Age=0"));
    assert!(clear.contains("Path=/"));
    assert!(clear.contains("HttpOnly"));
    assert!(clear.contains("Secure"));
    assert!(!clear_session_cookie(false).contains("Secure"));
  }
}
