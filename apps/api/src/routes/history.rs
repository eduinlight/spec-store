use axum::Json;
use axum::extract::{Path, Query, State};
use sdd_vault_core::diff::{DiffTag, diff_lines};
use sdd_vault_core::model::DocumentKind;
use serde::{Deserialize, Serialize};

use crate::auth::Principal;
use crate::error::{ApiError, ErrorBody};
use crate::routes::documents::RevisionOutput;
use crate::routes::projects::address;
use crate::state::AppState;

#[derive(Debug, Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub struct HistoryQuery {
  pub spec: Option<i64>,
  pub kind: String,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct RevisionsOutput {
  pub revisions: Vec<RevisionOutput>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct RevisionBodyOutput {
  pub body: String,
  pub kind: String,
  pub number: Option<i64>,
  pub revision: RevisionOutput,
  pub current: bool,
  pub current_revision: i64,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct DiffLineOutput {
  pub tag: String,
  pub old_line: Option<usize>,
  pub new_line: Option<usize>,
  pub text: String,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct HunkOutput {
  pub lines: Vec<DiffLineOutput>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct DiffOutput {
  pub revision: i64,
  pub previous_revision: Option<i64>,
  pub hunks: Vec<HunkOutput>,
}

#[utoipa::path(
  get,
  operation_id = "history_list",
  path = "/api/v1/projects/{org}/{project}/revisions",
  tag = "history",
  params(("org" = String, Path), ("project" = String, Path), HistoryQuery),
  responses((status = 200, body = RevisionsOutput), (status = 401, body = ErrorBody), (status = 404, body = ErrorBody), (status = 422, body = ErrorBody))
)]
pub async fn list(
  State(state): State<AppState>,
  principal: Principal,
  Path((org, project)): Path<(String, String)>,
  Query(query): Query<HistoryQuery>,
) -> Result<Json<RevisionsOutput>, ApiError> {
  let caller = principal.caller;
  let target = address(&org, &project);
  let revisions = state
    .blocking(move |store| {
      let kind: DocumentKind = query.kind.parse()?;
      store.list_revisions(&caller, &target, query.spec, kind)
    })
    .await?;
  Ok(Json(RevisionsOutput {
    revisions: revisions.into_iter().map(RevisionOutput::from).collect(),
  }))
}

#[utoipa::path(
  get,
  operation_id = "history_read",
  path = "/api/v1/projects/{org}/{project}/revisions/{id}",
  tag = "history",
  params(("org" = String, Path), ("project" = String, Path), ("id" = i64, Path)),
  responses((status = 200, body = RevisionBodyOutput), (status = 401, body = ErrorBody), (status = 404, body = ErrorBody))
)]
pub async fn read(
  State(state): State<AppState>,
  principal: Principal,
  Path((org, project, id)): Path<(String, String, i64)>,
) -> Result<Json<RevisionBodyOutput>, ApiError> {
  let caller = principal.caller;
  let target = address(&org, &project);
  let (revision, current_revision) = state
    .blocking(move |store| {
      let revision = store.get_revision(&caller, &target, id)?;
      let history = store.list_revisions(&caller, &target, revision.number, revision.kind)?;
      let current = history.first().map(|r| r.id).unwrap_or(revision.meta.id);
      Ok((revision, current))
    })
    .await?;
  Ok(Json(RevisionBodyOutput {
    current: revision.meta.id == current_revision,
    current_revision,
    body: revision.body,
    kind: revision.kind.as_str().to_string(),
    number: revision.number,
    revision: revision.meta.into(),
  }))
}

fn tag(tag: DiffTag) -> &'static str {
  match tag {
    DiffTag::Equal => "equal",
    DiffTag::Insert => "insert",
    DiffTag::Delete => "delete",
  }
}

#[utoipa::path(
  get,
  operation_id = "history_diff",
  path = "/api/v1/projects/{org}/{project}/revisions/{id}/diff",
  tag = "history",
  params(("org" = String, Path), ("project" = String, Path), ("id" = i64, Path)),
  responses((status = 200, body = DiffOutput), (status = 401, body = ErrorBody), (status = 404, body = ErrorBody))
)]
pub async fn diff(
  State(state): State<AppState>,
  principal: Principal,
  Path((org, project, id)): Path<(String, String, i64)>,
) -> Result<Json<DiffOutput>, ApiError> {
  let caller = principal.caller;
  let target = address(&org, &project);
  let (new_body, previous) = state
    .blocking(move |store| {
      let revision = store.get_revision(&caller, &target, id)?;
      let history = store.list_revisions(&caller, &target, revision.number, revision.kind)?;
      let previous_id = history
        .iter()
        .position(|r| r.id == id)
        .and_then(|index| history.get(index + 1))
        .map(|r| r.id);
      let previous = match previous_id {
        Some(previous_id) => Some((
          previous_id,
          store.get_revision(&caller, &target, previous_id)?.body,
        )),
        None => None,
      };
      Ok((revision.body, previous))
    })
    .await?;
  let old_body = previous
    .as_ref()
    .map(|(_, body)| body.as_str())
    .unwrap_or("");
  let hunks = diff_lines(old_body, &new_body)
    .into_iter()
    .map(|hunk| HunkOutput {
      lines: hunk
        .lines
        .into_iter()
        .map(|line| DiffLineOutput {
          tag: tag(line.tag).to_string(),
          old_line: line.old_line,
          new_line: line.new_line,
          text: line.text,
        })
        .collect(),
    })
    .collect();
  Ok(Json(DiffOutput {
    revision: id,
    previous_revision: previous.map(|(id, _)| id),
    hunks,
  }))
}
