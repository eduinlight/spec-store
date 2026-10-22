use axum::Form;
use axum::extract::{Query, State};
use axum::http::HeaderMap;
use axum::response::{Html, IntoResponse, Redirect, Response};
use serde::Deserialize;

use crate::model::StoreError;
use crate::ui::cookies::{clear_session_cookie, session_from_headers, set_session_cookie};
use crate::ui::html::{escape, layout};
use crate::ui::{UiError, UiState, set_cookie_header};

#[derive(Debug, Deserialize)]
pub struct NextQuery {
  #[serde(default)]
  pub next: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginForm {
  #[serde(default)]
  pub token: String,
  #[serde(default)]
  pub next: String,
}

pub async fn form(Query(query): Query<NextQuery>) -> Html<String> {
  Html(login_page(&safe_next(Some(&query.next)), None))
}

pub async fn submit(State(state): State<UiState>, Form(form): Form<LoginForm>) -> Response {
  let next = safe_next(Some(&form.next));
  match state.store.create_session(form.token.trim()) {
    Ok(secret) => {
      let mut response = Redirect::to(&next).into_response();
      set_cookie_header(
        &mut response,
        &set_session_cookie(&secret, state.config.secure_cookies),
      );
      response
    }
    Err(StoreError::Unauthenticated) => {
      Html(login_page(&next, Some("The token is not valid."))).into_response()
    }
    Err(error) => UiError::from(error).into_response(),
  }
}

pub async fn logout(State(state): State<UiState>, headers: HeaderMap) -> Response {
  if let Some(secret) = session_from_headers(&headers)
    && let Err(error) = state.store.end_session(&secret)
  {
    return UiError::from(error).into_response();
  }
  let mut response = Redirect::to("/ui/login").into_response();
  set_cookie_header(
    &mut response,
    &clear_session_cookie(state.config.secure_cookies),
  );
  response
}

fn login_page(next: &str, error: Option<&str>) -> String {
  let error_html = error
    .map(|e| format!("<p class=\"error\">{}</p>", escape(e)))
    .unwrap_or_default();
  let body = format!(
    "<section class=\"login\"><h1>Sign in</h1>{error_html}<form method=\"post\" action=\"/ui/login\"><label for=\"token\">API token</label><input id=\"token\" name=\"token\" type=\"password\" autocomplete=\"off\" required><input type=\"hidden\" name=\"next\" value=\"{}\"><button type=\"submit\">Sign in</button></form><p class=\"muted\">Paste the API token issued to you by the server admin. It is exchanged for a session cookie and never shown again.</p></section>",
    escape(next)
  );
  layout("Sign in", None, &body)
}

pub const UI_ROOT: &str = "/ui/";

pub fn safe_next(next: Option<&str>) -> String {
  match next {
    Some(path)
      if path.starts_with(UI_ROOT)
        && !path.contains("//")
        && !path.contains("..")
        && !path.contains('\\')
        && !path.chars().any(|c| c.is_control() || c.is_whitespace()) =>
    {
      path.to_string()
    }
    _ => UI_ROOT.to_string(),
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn safe_next_keeps_ui_paths_only() {
    assert_eq!(safe_next(Some("/ui/p/demo")), "/ui/p/demo");
    assert_eq!(safe_next(Some("/ui/")), "/ui/");
    assert_eq!(
      safe_next(Some("/ui/p/demo/specs/1?view=raw")),
      "/ui/p/demo/specs/1?view=raw"
    );
    for bad in [
      "//evil.example",
      "https://evil.example/ui/",
      "/mcp",
      "/ui",
      "",
      "/ui/../mcp",
      "/ui//evil",
      "/ui/x\r\nSet-Cookie: a=b",
      "javascript:alert(1)",
    ] {
      assert_eq!(safe_next(Some(bad)), "/ui/", "{bad:?}");
    }
    assert_eq!(safe_next(None), "/ui/");
  }
}
