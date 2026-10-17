use axum::Extension;
use axum::extract::{Path, Query, State};
use axum::http::header::CONTENT_TYPE;
use axum::response::{Html, IntoResponse, Response};
use serde::Deserialize;

use crate::markdown::tasks::ParsedTasks;
use crate::model::{Caller, DocumentKind, StoreError};
use crate::store::Store;
use crate::store::documents::RevisionMeta;
use crate::store::specs::SpecSummary;
use crate::store::tasks::SpecMarker;
use crate::ui::html::{badge, escape, layout, link};
use crate::ui::render::render_markdown;
use crate::ui::{UiError, UiResult, UiState, Viewer};

#[derive(Debug, Deserialize)]
pub struct ViewQuery {
  pub view: Option<String>,
}

impl ViewQuery {
  fn raw(&self) -> bool {
    self.view.as_deref() == Some("raw")
  }
}

pub fn raw_response(body: &str) -> Response {
  (
    [(CONTENT_TYPE, "text/plain; charset=utf-8")],
    body.to_string(),
  )
    .into_response()
}

pub fn parse_number(raw: &str) -> Result<i64, UiError> {
  raw
    .parse::<i64>()
    .ok()
    .filter(|n| *n > 0)
    .ok_or(UiError::NotFound)
}

pub fn spec_kind(raw: &str) -> Result<DocumentKind, UiError> {
  let kind: DocumentKind = raw.parse().map_err(|_| UiError::NotFound)?;
  if DocumentKind::SPEC_KINDS.contains(&kind) {
    Ok(kind)
  } else {
    Err(UiError::NotFound)
  }
}

pub fn find_spec(
  store: &Store,
  caller: &Caller,
  slug: &str,
  number: i64,
) -> Result<SpecSummary, UiError> {
  store
    .list_specs(caller, slug)?
    .into_iter()
    .find(|s| s.number == number)
    .ok_or(UiError::NotFound)
}

pub fn revision_meta(meta: &RevisionMeta) -> String {
  let version = meta
    .version
    .as_deref()
    .map(|v| format!(" · version {}", escape(v)))
    .unwrap_or_default();
  format!(
    "<p class=\"meta\">Revision {} by {} at {} — {}{version}</p>",
    meta.id,
    escape(&meta.author),
    escape(&meta.created_at),
    escape(&meta.message)
  )
}

fn views(base: &str, include_open: bool) -> String {
  let mut items = Vec::new();
  if include_open {
    items.push(link(base, "open"));
  }
  items.push(link(&format!("{base}?view=raw"), "raw"));
  items.push(link(&format!("{base}/history"), "history"));
  format!("<p class=\"views\">{}</p>", items.join(" · "))
}

fn document_article(body: &str) -> String {
  format!(
    "<article class=\"document\">{}</article>",
    render_markdown(body)
  )
}

pub fn commit_link(repository_url: Option<&str>, commit: &str) -> String {
  match repository_url {
    Some(repo) => link(
      &format!("{}/commit/{commit}", repo.trim_end_matches('/')),
      commit,
    ),
    None => format!("<code>{}</code>", escape(commit)),
  }
}

fn tasks_section(parsed: &ParsedTasks, repository_url: Option<&str>) -> String {
  let mut out = String::new();
  let mut phases: Vec<&str> = Vec::new();
  for task in &parsed.tasks {
    if !phases.contains(&task.phase.as_str()) {
      phases.push(task.phase.as_str());
    }
  }
  for phase in phases {
    let in_phase: Vec<_> = parsed.tasks.iter().filter(|t| t.phase == phase).collect();
    let done = in_phase.iter().filter(|t| t.done).count();
    let heading = if phase.is_empty() {
      "Without phase".to_string()
    } else {
      escape(phase)
    };
    out.push_str(&format!(
      "<h3>{heading} <span class=\"progress\">{done}/{}</span></h3>",
      in_phase.len()
    ));
    let mut rows = String::new();
    for task in in_phase {
      let frs = if task.frs.is_empty() {
        "—".to_string()
      } else {
        escape(&task.frs.join(", "))
      };
      let commit = task
        .commit
        .as_deref()
        .map(|c| commit_link(repository_url, c))
        .unwrap_or_default();
      rows.push_str(&format!(
        "<tr{}><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{commit}</td></tr>",
        if task.done { " class=\"done\"" } else { "" },
        escape(&task.id),
        if task.parallel { "[P]" } else { "" },
        frs,
        escape(&task.description),
        escape(&task.done_when),
        if task.done { "done" } else { "pending" }
      ));
    }
    out.push_str(&format!(
      "<table class=\"tasks\"><thead><tr><th>Task</th><th></th><th>FRs</th><th>Description</th><th>Done when</th><th>State</th><th>Commit</th></tr></thead><tbody>{rows}</tbody></table>"
    ));
  }
  if !parsed.malformed.is_empty() {
    out.push_str("<h3>Malformed lines</h3><ul class=\"malformed\">");
    for line in &parsed.malformed {
      out.push_str(&format!(
        "<li>line {}: <code>{}</code></li>",
        line.line,
        escape(&line.text)
      ));
    }
    out.push_str("</ul>");
  }
  out
}

fn markers_section(markers: &[SpecMarker]) -> String {
  if markers.is_empty() {
    return "<p class=\"empty\">No markers.</p>".to_string();
  }
  let mut rows = String::new();
  for marker in markers {
    rows.push_str(&format!(
      "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
      badge(marker.kind.as_str()),
      escape(marker.document_kind.as_str()),
      escape(&marker.heading),
      escape(&marker.text),
      marker.line
    ));
  }
  format!(
    "<table><thead><tr><th>Kind</th><th>Document</th><th>Heading</th><th>Text</th><th>Line</th></tr></thead><tbody>{rows}</tbody></table>"
  )
}

pub async fn spec(
  State(state): State<UiState>,
  Extension(viewer): Extension<Viewer>,
  Path((slug, number)): Path<(String, String)>,
) -> UiResult {
  let caller = &viewer.caller;
  let number = parse_number(&number)?;
  let project = state.store.project(caller, &slug)?;
  let spec = find_spec(&state.store, caller, &slug, number)?;
  let title = format!("Spec {:03} — {}", spec.number, spec.slug);
  let mut body = format!(
    "<p class=\"meta\">{} › {}</p><h1>{} {}</h1><p class=\"meta\">Branch <code>{}</code></p>",
    link(&format!("/ui/p/{slug}"), &project.name),
    escape(&title),
    escape(&title),
    badge(spec.status.as_str()),
    escape(&spec.branch)
  );

  let mut tasks_html = None;
  for kind in DocumentKind::SPEC_KINDS {
    let name = kind.as_str();
    body.push_str(&format!("<section id=\"{name}\"><h2>{name}</h2>"));
    match state.store.get_document(caller, &slug, number, kind) {
      Ok(doc) => {
        let base = format!("/ui/p/{slug}/specs/{number}/{name}");
        body.push_str(&revision_meta(&doc.revision));
        body.push_str(&views(&base, true));
        if kind == DocumentKind::Tasks {
          let parsed = crate::markdown::tasks::parse_tasks(&doc.body);
          tasks_html = Some(tasks_section(&parsed, project.repository_url.as_deref()));
        }
        body.push_str(&document_article(&doc.body));
      }
      Err(StoreError::NotFound(_)) => {
        body.push_str("<p class=\"empty\">Not written yet.</p>");
      }
      Err(error) => return Err(error.into()),
    }
    body.push_str("</section>");
  }

  body.push_str("<section id=\"task-table\"><h2>Tasks</h2>");
  match tasks_html {
    Some(html) => body.push_str(&html),
    None => body.push_str("<p class=\"empty\">No tasks document yet.</p>"),
  }
  body.push_str("</section>");

  let markers = state.store.list_markers(caller, &slug, number)?;
  body.push_str("<section id=\"markers\"><h2>Markers</h2>");
  body.push_str(&markers_section(&markers));
  body.push_str("</section>");
  body.push_str(&format!(
    "<p>{} · {}</p>",
    link(&format!("/ui/p/{slug}/constitution"), "Constitution"),
    link(&format!("/ui/p/{slug}"), "Back to project")
  ));

  Ok(Html(layout(&title, Some(caller), &body)).into_response())
}

pub async fn document(
  State(state): State<UiState>,
  Extension(viewer): Extension<Viewer>,
  Path((slug, number, kind)): Path<(String, String, String)>,
  Query(query): Query<ViewQuery>,
) -> UiResult {
  let caller = &viewer.caller;
  let number = parse_number(&number)?;
  let kind = spec_kind(&kind)?;
  let doc = state.store.get_document(caller, &slug, number, kind)?;
  if query.raw() {
    return Ok(raw_response(&doc.body));
  }
  let project = state.store.project(caller, &slug)?;
  let spec = find_spec(&state.store, caller, &slug, number)?;
  let name = kind.as_str();
  let title = format!("{:03} {} — {name}", spec.number, spec.slug);
  let base = format!("/ui/p/{slug}/specs/{number}/{name}");
  let body = format!(
    "<p class=\"meta\">{} › {} › {name}</p><h1>{}</h1>{}{}{}",
    link(&format!("/ui/p/{slug}"), &project.name),
    link(
      &format!("/ui/p/{slug}/specs/{number}"),
      &format!("{:03} {}", spec.number, spec.slug)
    ),
    escape(&title),
    revision_meta(&doc.revision),
    views(&base, false),
    document_article(&doc.body)
  );
  Ok(Html(layout(&title, Some(caller), &body)).into_response())
}

pub async fn constitution(
  State(state): State<UiState>,
  Extension(viewer): Extension<Viewer>,
  Path(slug): Path<String>,
  Query(query): Query<ViewQuery>,
) -> UiResult {
  let caller = &viewer.caller;
  let constitution = state.store.get_constitution(caller, &slug)?;
  if query.raw() {
    return Ok(raw_response(&constitution.body));
  }
  let project = state.store.project(caller, &slug)?;
  let base = format!("/ui/p/{slug}/constitution");
  let body = format!(
    "<p class=\"meta\">{} › Constitution</p><h1>Constitution</h1><p class=\"meta\">Version {}</p>{}{}{}",
    link(&format!("/ui/p/{slug}"), &project.name),
    escape(&constitution.version),
    revision_meta(&constitution.revision),
    views(&base, false),
    document_article(&constitution.body)
  );
  Ok(Html(layout("Constitution", Some(caller), &body)).into_response())
}
