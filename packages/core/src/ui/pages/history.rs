use axum::Extension;
use axum::extract::{Path, Query, State};
use axum::response::{Html, IntoResponse};

use crate::model::{Caller, DocumentKind};
use crate::store::Store;
use crate::store::documents::{RevisionBody, RevisionMeta};
use crate::ui::diff::{diff_lines, render_diff};
use crate::ui::html::{escape, layout, link};
use crate::ui::pages::specs::{
  ViewQuery, find_spec, parse_number, raw_response, revision_meta, spec_kind,
};
use crate::ui::render::render_markdown;
use crate::ui::{UiError, UiResult, UiState, Viewer};

struct DocumentRef {
  slug: String,
  kind: DocumentKind,
  number: Option<i64>,
}

impl DocumentRef {
  fn document_url(&self) -> String {
    match self.number {
      None => format!("/ui/p/{}/constitution", self.slug),
      Some(number) => format!("/ui/p/{}/specs/{number}/{}", self.slug, self.kind.as_str()),
    }
  }

  fn history_url(&self) -> String {
    format!("{}/history", self.document_url())
  }

  fn label(&self) -> String {
    match self.number {
      None => "Constitution".to_string(),
      Some(number) => format!("{number:03} {}", self.kind.as_str()),
    }
  }

  fn current_revision_id(&self, store: &Store, caller: &Caller) -> Result<i64, UiError> {
    Ok(match self.number {
      None => store.get_constitution(caller, &self.slug)?.revision.id,
      Some(number) => {
        store
          .get_document(caller, &self.slug, number, self.kind)?
          .revision
          .id
      }
    })
  }

  fn revisions(&self, store: &Store, caller: &Caller) -> Result<Vec<RevisionMeta>, UiError> {
    Ok(store.list_revisions(caller, &self.slug, self.number, self.kind)?)
  }
}

fn history_table(slug: &str, revisions: &[RevisionMeta], current_id: i64) -> String {
  let show_version = revisions.iter().any(|r| r.version.is_some());
  let version_header = if show_version { "<th>Version</th>" } else { "" };
  let mut rows = String::new();
  for revision in revisions {
    let version = if show_version {
      format!(
        "<td>{}</td>",
        escape(revision.version.as_deref().unwrap_or(""))
      )
    } else {
      String::new()
    };
    let current = if revision.id == current_id {
      " <span class=\"badge\">current</span>"
    } else {
      ""
    };
    rows.push_str(&format!(
      "<tr><td>{}{current}</td>{version}<td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
      link(
        &format!("/ui/p/{slug}/r/{}", revision.id),
        &revision.id.to_string()
      ),
      escape(&revision.author),
      escape(&revision.created_at),
      escape(&revision.message),
      link(&format!("/ui/p/{slug}/r/{}/diff", revision.id), "diff")
    ));
  }
  format!(
    "<table class=\"history\"><thead><tr><th>Revision</th>{version_header}<th>Author</th><th>Time</th><th>Message</th><th></th></tr></thead><tbody>{rows}</tbody></table>"
  )
}

fn history_page(state: &UiState, caller: &Caller, doc: DocumentRef) -> UiResult {
  let current_id = doc.current_revision_id(&state.store, caller)?;
  let revisions = doc.revisions(&state.store, caller)?;
  let project = state.store.project(caller, &doc.slug)?;
  let title = format!("History of {}", doc.label());
  let mut crumbs = format!("{} › ", link(&format!("/ui/p/{}", doc.slug), &project.name));
  if let Some(number) = doc.number {
    let spec = find_spec(&state.store, caller, &doc.slug, number)?;
    crumbs.push_str(&format!(
      "{} › ",
      link(
        &format!("/ui/p/{}/specs/{number}", doc.slug),
        &format!("{number:03} {}", spec.slug)
      )
    ));
  }
  crumbs.push_str(&link(&doc.document_url(), &doc.label()));
  let body = format!(
    "<p class=\"meta\">{crumbs} › history</p><h1>{}</h1>{}",
    escape(&title),
    history_table(&doc.slug, &revisions, current_id)
  );
  Ok(Html(layout(&title, Some(caller), &body)).into_response())
}

pub async fn document_history(
  State(state): State<UiState>,
  Extension(viewer): Extension<Viewer>,
  Path((slug, number, kind)): Path<(String, String, String)>,
) -> UiResult {
  let number = parse_number(&number)?;
  let kind = spec_kind(&kind)?;
  history_page(
    &state,
    &viewer.caller,
    DocumentRef {
      slug,
      kind,
      number: Some(number),
    },
  )
}

pub async fn constitution_history(
  State(state): State<UiState>,
  Extension(viewer): Extension<Viewer>,
  Path(slug): Path<String>,
) -> UiResult {
  history_page(
    &state,
    &viewer.caller,
    DocumentRef {
      slug,
      kind: DocumentKind::Constitution,
      number: None,
    },
  )
}

fn load_revision(
  state: &UiState,
  caller: &Caller,
  slug: &str,
  raw_id: &str,
) -> Result<(RevisionBody, DocumentRef), UiError> {
  let id = parse_number(raw_id)?;
  let revision = state.store.get_revision(caller, slug, id)?;
  let doc = DocumentRef {
    slug: slug.to_string(),
    kind: revision.kind,
    number: revision.number,
  };
  Ok((revision, doc))
}

pub async fn revision(
  State(state): State<UiState>,
  Extension(viewer): Extension<Viewer>,
  Path((slug, id)): Path<(String, String)>,
  Query(query): Query<ViewQuery>,
) -> UiResult {
  let caller = &viewer.caller;
  let (revision, doc) = load_revision(&state, caller, &slug, &id)?;
  if query.view.as_deref() == Some("raw") {
    return Ok(raw_response(&revision.body));
  }
  let project = state.store.project(caller, &slug)?;
  let current_id = doc.current_revision_id(&state.store, caller)?;
  let notice = if revision.meta.id == current_id {
    String::new()
  } else {
    format!(
      "<p class=\"notice\">This is not the current revision. {}</p>",
      link(&doc.document_url(), "Open the current revision")
    )
  };
  let title = format!("Revision {} of {}", revision.meta.id, doc.label());
  let body = format!(
    "<p class=\"meta\">{} › {} › {}</p><h1>{}</h1>{}{notice}<p class=\"views\">{} · {} · {}</p><article class=\"document\">{}</article>",
    link(&format!("/ui/p/{slug}"), &project.name),
    link(&doc.document_url(), &doc.label()),
    link(&doc.history_url(), "history"),
    escape(&title),
    revision_meta(&revision.meta),
    link(
      &format!("/ui/p/{slug}/r/{}?view=raw", revision.meta.id),
      "raw"
    ),
    link(&format!("/ui/p/{slug}/r/{}/diff", revision.meta.id), "diff"),
    link(&doc.history_url(), "history"),
    render_markdown(&revision.body)
  );
  Ok(Html(layout(&title, Some(caller), &body)).into_response())
}

pub async fn diff(
  State(state): State<UiState>,
  Extension(viewer): Extension<Viewer>,
  Path((slug, id)): Path<(String, String)>,
) -> UiResult {
  let caller = &viewer.caller;
  let (revision, doc) = load_revision(&state, caller, &slug, &id)?;
  let project = state.store.project(caller, &slug)?;
  let revisions = doc.revisions(&state.store, caller)?;
  let predecessor = revisions
    .iter()
    .skip_while(|r| r.id != revision.meta.id)
    .nth(1)
    .cloned();
  let title = format!(
    "Changes in revision {} of {}",
    revision.meta.id,
    doc.label()
  );
  let comparison = match predecessor {
    None => {
      "<p class=\"empty\">This is the first revision; it has no predecessor to compare with.</p>"
        .to_string()
    }
    Some(previous) => {
      let previous_body = state.store.get_revision(caller, &slug, previous.id)?;
      let hunks = diff_lines(&previous_body.body, &revision.body);
      let table = if hunks.is_empty() {
        "<p class=\"empty\">The body is identical to the previous revision.</p>".to_string()
      } else {
        format!(
          "<table class=\"diff\"><tbody>{}</tbody></table>",
          render_diff(&hunks)
        )
      };
      format!(
        "<h2>Compared with revision {}</h2>{}{table}",
        link(
          &format!("/ui/p/{slug}/r/{}", previous.id),
          &previous.id.to_string()
        ),
        revision_meta(&previous)
      )
    }
  };
  let body = format!(
    "<p class=\"meta\">{} › {} › {}</p><h1>{}</h1>{}<p class=\"views\">{} · {}</p>{comparison}",
    link(&format!("/ui/p/{slug}"), &project.name),
    link(&doc.document_url(), &doc.label()),
    link(&doc.history_url(), "history"),
    escape(&title),
    revision_meta(&revision.meta),
    link(
      &format!("/ui/p/{slug}/r/{}", revision.meta.id),
      "open this revision"
    ),
    link(&doc.history_url(), "history")
  );
  Ok(Html(layout(&title, Some(caller), &body)).into_response())
}
