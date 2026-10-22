use axum::Extension;
use axum::extract::{Path, Query, State};
use axum::response::{Html, IntoResponse};
use serde::Deserialize;

use crate::model::StoreError;
use crate::store::AuditPage;
use crate::ui::html::{escape, layout, link};
use crate::ui::{UiError, UiResult, UiState, Viewer};

pub const PAGE_SIZE: usize = 50;

#[derive(Debug, Deserialize)]
pub struct AuditQuery {
  pub before: Option<i64>,
  pub project: Option<String>,
}

fn audit_table(page: &AuditPage, with_project: bool) -> String {
  if page.entries.is_empty() {
    return "<p class=\"empty\">No audit entries on this page.</p>".to_string();
  }
  let project_header = if with_project { "<th>Project</th>" } else { "" };
  let mut rows = String::new();
  for entry in &page.entries {
    let project = if with_project {
      format!(
        "<td>{}</td>",
        escape(entry.project.as_deref().unwrap_or("—"))
      )
    } else {
      String::new()
    };
    rows.push_str(&format!(
      "<tr class=\"entry\"><td>{}</td><td>{}</td><td><code>{}</code></td><td>{}</td>{project}</tr>",
      escape(&entry.created_at),
      escape(&entry.actor),
      escape(&entry.action),
      escape(&entry.target)
    ));
  }
  format!(
    "<table class=\"audit\"><thead><tr><th>Time</th><th>Actor</th><th>Action</th><th>Target</th>{project_header}</tr></thead><tbody>{rows}</tbody></table>"
  )
}

fn pagination(base: &str, filter: Option<&str>, page: &AuditPage, before: Option<i64>) -> String {
  let filter_query = filter
    .filter(|f| !f.is_empty())
    .map(|f| format!("project={}", crate::ui::percent_encode(f)));
  let mut links = Vec::new();
  if before.is_some() {
    let newer = match &filter_query {
      Some(q) => format!("{base}?{q}"),
      None => base.to_string(),
    };
    links.push(link(&newer, "Newer entries"));
  }
  if page.has_more
    && let Some(last) = page.entries.last()
  {
    let older = match &filter_query {
      Some(q) => format!("{base}?{q}&before={}", last.id),
      None => format!("{base}?before={}", last.id),
    };
    links.push(link(&older, "Older entries"));
  }
  if links.is_empty() {
    String::new()
  } else {
    format!("<p class=\"pagination\">{}</p>", links.join(" · "))
  }
}

pub async fn project_audit(
  State(state): State<UiState>,
  Extension(viewer): Extension<Viewer>,
  Path(slug): Path<String>,
  Query(query): Query<AuditQuery>,
) -> UiResult {
  let caller = &viewer.caller;
  let access = state.store.access(caller, &slug)?;
  if !access.can_manage_members() {
    return Err(UiError::NotAllowed);
  }
  let project = state.store.project(caller, &slug)?;
  let page = state
    .store
    .list_audit_page(caller, Some(&slug), query.before, PAGE_SIZE)?;
  let base = format!("/ui/p/{slug}/audit");
  let title = format!("Audit trail of {}", project.name);
  let body = format!(
    "<p class=\"meta\">{} › audit</p><h1>{}</h1>{}{}",
    link(&format!("/ui/p/{slug}"), &project.name),
    escape(&title),
    audit_table(&page, false),
    pagination(&base, None, &page, query.before)
  );
  Ok(Html(layout(&title, Some(caller), &body)).into_response())
}

pub async fn global_audit(
  State(state): State<UiState>,
  Extension(viewer): Extension<Viewer>,
  Query(query): Query<AuditQuery>,
) -> UiResult {
  let caller = &viewer.caller;
  if !caller.is_admin {
    return Err(UiError::NotAllowed);
  }
  let filter = query
    .project
    .as_deref()
    .map(str::trim)
    .filter(|p| !p.is_empty());
  let page = match state
    .store
    .list_audit_page(caller, filter, query.before, PAGE_SIZE)
  {
    Ok(page) => page,
    Err(StoreError::NotFound(_)) => return Err(UiError::NotFound),
    Err(error) => return Err(error.into()),
  };
  let form = format!(
    "<form method=\"get\" action=\"/ui/audit\" class=\"filter\"><label for=\"project\">Project slug</label> <input id=\"project\" name=\"project\" value=\"{}\"> <button type=\"submit\">Filter</button></form>",
    escape(filter.unwrap_or(""))
  );
  let body = format!(
    "<h1>Audit trail</h1>{form}{}{}",
    audit_table(&page, true),
    pagination("/ui/audit", filter, &page, query.before)
  );
  Ok(Html(layout("Audit trail", Some(caller), &body)).into_response())
}
