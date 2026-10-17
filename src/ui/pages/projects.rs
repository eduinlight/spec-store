use axum::Extension;
use axum::extract::{Path, State};
use axum::response::{Html, IntoResponse};

use crate::model::{Caller, StoreError};
use crate::store::Store;
use crate::store::specs::SpecSummary;
use crate::store::tasks::ActiveStatus;
use crate::ui::html::{badge, escape, layout, link};
use crate::ui::{UiResult, UiState, Viewer};

pub async fn index(State(state): State<UiState>, Extension(viewer): Extension<Viewer>) -> UiResult {
  let projects = state.store.project_overview(&viewer.caller)?;
  let body = if projects.is_empty() {
    "<h1>Projects</h1><p class=\"empty\">You are not a member of any project yet.</p>".to_string()
  } else {
    let mut rows = String::new();
    for project in &projects {
      let role = match project.role {
        Some(role) => badge(role.as_str()),
        None => "<span class=\"muted\">no membership</span>".to_string(),
      };
      let constitution = match &project.constitution_version {
        Some(version) => format!("v{}", escape(version)),
        None => "<span class=\"muted\">no constitution</span>".to_string(),
      };
      let active = match &project.active {
        Some(active) => active_summary(&project.slug, active),
        None => "<span class=\"muted\">no active spec</span>".to_string(),
      };
      rows.push_str(&format!(
        "<tr><td>{}</td><td>{}</td><td>{role}</td><td>{constitution}</td><td>{}</td><td>{active}</td></tr>",
        link(&format!("/ui/p/{}", project.slug), &project.name),
        escape(&project.slug),
        project.spec_count
      ));
    }
    format!(
      "<h1>Projects</h1><table><thead><tr><th>Project</th><th>Slug</th><th>Your role</th><th>Constitution</th><th>Specs</th><th>Active spec</th></tr></thead><tbody>{rows}</tbody></table>"
    )
  };
  Ok(Html(layout("Projects", Some(&viewer.caller), &body)).into_response())
}

fn active_summary(slug: &str, active: &ActiveStatus) -> String {
  format!(
    "{} {} <span class=\"progress\">{}/{} tasks</span>, {}",
    link(
      &format!("/ui/p/{slug}/specs/{}", active.number),
      &format!("{:03} {}", active.number, active.slug)
    ),
    badge(active.status.as_str()),
    active.tasks_done,
    active.tasks_total,
    markers_text(active.markers)
  )
}

fn markers_text(count: usize) -> String {
  match count {
    1 => "1 marker".to_string(),
    n => format!("{n} markers"),
  }
}

struct SpecRowData {
  summary: SpecSummary,
  tasks_done: usize,
  tasks_total: usize,
  markers: usize,
}

fn spec_rows(store: &Store, caller: &Caller, slug: &str) -> Result<Vec<SpecRowData>, StoreError> {
  let mut rows = Vec::new();
  for summary in store.list_specs(caller, slug)? {
    let (tasks_done, tasks_total) = match store.list_tasks(caller, slug, summary.number, false) {
      Ok(parsed) => (
        parsed.tasks.iter().filter(|t| t.done).count(),
        parsed.tasks.len(),
      ),
      Err(StoreError::NotFound(_)) => (0, 0),
      Err(error) => return Err(error),
    };
    let markers = store.list_markers(caller, slug, summary.number)?.len();
    rows.push(SpecRowData {
      summary,
      tasks_done,
      tasks_total,
      markers,
    });
  }
  Ok(rows)
}

pub async fn show(
  State(state): State<UiState>,
  Extension(viewer): Extension<Viewer>,
  Path(slug): Path<String>,
) -> UiResult {
  let caller = &viewer.caller;
  let project = state.store.project(caller, &slug)?;
  let access = state.store.access(caller, &slug)?;
  let status = state.store.project_status(caller, &slug)?;
  let members = state.store.list_members(caller, &slug)?;
  let specs = spec_rows(&state.store, caller, &slug)?;
  let active_number = status.active.as_ref().map(|a| a.number);

  let mut body = format!("<h1>{}</h1>", escape(&project.name));
  body.push_str(&format!(
    "<p class=\"meta\">Slug <code>{}</code></p>",
    escape(&project.slug)
  ));
  if access.role.is_none() {
    body.push_str(
      "<p class=\"muted\">You are viewing this project as an admin without membership (read-only).</p>",
    );
  }
  match &project.repository_url {
    Some(url) => body.push_str(&format!("<p>Repository: {}</p>", link(url, url))),
    None => body.push_str("<p class=\"muted\">No repository URL set.</p>"),
  }
  match &status.constitution_version {
    Some(version) => body.push_str(&format!(
      "<p>Constitution {}</p>",
      link(
        &format!("/ui/p/{slug}/constitution"),
        &format!("v{version}")
      )
    )),
    None => body.push_str("<p class=\"muted\">No constitution yet.</p>"),
  }
  if access.can_manage_members() {
    body.push_str(&format!(
      "<p>{}</p>",
      link(&format!("/ui/p/{slug}/audit"), "Audit trail")
    ));
  }

  body.push_str("<h2>Specs</h2>");
  if specs.is_empty() {
    body.push_str("<p class=\"empty\">No specs yet.</p>");
  } else {
    let mut rows = String::new();
    for row in &specs {
      let s = &row.summary;
      let is_active = active_number == Some(s.number);
      let kinds = s
        .kinds
        .iter()
        .map(|k| k.as_str())
        .collect::<Vec<_>>()
        .join(", ");
      let active_mark = if is_active {
        " <span class=\"badge\">active</span>"
      } else {
        ""
      };
      rows.push_str(&format!(
        "<tr{}><td>{}{active_mark}</td><td>{}</td><td><code>{}</code></td><td>{}</td><td class=\"progress\">{}/{}</td><td>{}</td></tr>",
        if is_active { " class=\"active\"" } else { "" },
        link(
          &format!("/ui/p/{slug}/specs/{}", s.number),
          &format!("{:03} {}", s.number, s.slug)
        ),
        badge(s.status.as_str()),
        escape(&s.branch),
        escape(&kinds),
        row.tasks_done,
        row.tasks_total,
        markers_text(row.markers)
      ));
    }
    body.push_str(&format!(
      "<table><thead><tr><th>Spec</th><th>Status</th><th>Branch</th><th>Documents</th><th>Tasks</th><th>Markers</th></tr></thead><tbody>{rows}</tbody></table>"
    ));
  }

  body.push_str("<h2>Members</h2>");
  if members.is_empty() {
    body.push_str("<p class=\"empty\">No members.</p>");
  } else {
    let mut rows = String::new();
    for member in &members {
      rows.push_str(&format!(
        "<tr><td>{}</td><td>{}</td></tr>",
        escape(&member.handle),
        badge(member.role.as_str())
      ));
    }
    body.push_str(&format!(
      "<table><thead><tr><th>Handle</th><th>Role</th></tr></thead><tbody>{rows}</tbody></table>"
    ));
  }

  Ok(Html(layout(&project.name, Some(caller), &body)).into_response())
}
