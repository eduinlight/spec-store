use axum::Extension;
use axum::extract::State;
use axum::response::{Html, IntoResponse};

use crate::ui::html::{escape, layout};
use crate::ui::{UiResult, UiState, Viewer};

pub async fn show(State(state): State<UiState>, Extension(viewer): Extension<Viewer>) -> UiResult {
  let tokens = state.store.list_own_tokens(&viewer.caller)?;
  let mut rows = String::new();
  for token in &tokens {
    let state_text = match &token.revoked_at {
      Some(at) => format!("revoked {}", escape(at)),
      None => "active".to_string(),
    };
    let current = if token.id == viewer.token_id {
      "<span class=\"badge\">current session</span>"
    } else {
      ""
    };
    rows.push_str(&format!(
      "<tr><td>{}</td><td>{}</td><td>{}</td><td>{state_text}</td><td>{current}</td></tr>",
      escape(&token.label),
      escape(&token.created_at),
      escape(token.last_used_at.as_deref().unwrap_or("never")),
    ));
  }
  let admin = if viewer.caller.is_admin {
    " <span class=\"badge badge-admin\">admin</span>"
  } else {
    ""
  };
  let body = format!(
    "<h1>{}{admin}</h1><h2>Your tokens</h2><p class=\"muted\">Token values are never shown; ask a server admin to issue or revoke tokens.</p><table><thead><tr><th>Label</th><th>Created</th><th>Last used</th><th>State</th><th></th></tr></thead><tbody>{rows}</tbody></table>",
    escape(&viewer.caller.handle)
  );
  Ok(Html(layout("Profile", Some(&viewer.caller), &body)).into_response())
}
