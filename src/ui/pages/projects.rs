use axum::Extension;
use axum::extract::State;
use axum::response::{Html, IntoResponse};

use crate::ui::html::{escape, layout, link};
use crate::ui::{UiResult, UiState, Viewer};

pub async fn index(State(state): State<UiState>, Extension(viewer): Extension<Viewer>) -> UiResult {
  let projects = state.store.list_projects(&viewer.caller)?;
  let body = if projects.is_empty() {
    "<h1>Projects</h1><p class=\"empty\">You are not a member of any project yet.</p>".to_string()
  } else {
    let mut rows = String::new();
    for project in &projects {
      rows.push_str(&format!(
        "<tr><td>{}</td><td>{}</td><td>{}</td></tr>",
        link(&format!("/ui/p/{}", project.slug), &project.name),
        escape(&project.slug),
        escape(&project.role)
      ));
    }
    format!(
      "<h1>Projects</h1><table><thead><tr><th>Project</th><th>Slug</th><th>Your role</th></tr></thead><tbody>{rows}</tbody></table>"
    )
  };
  Ok(Html(layout("Projects", Some(&viewer.caller), &body)).into_response())
}
