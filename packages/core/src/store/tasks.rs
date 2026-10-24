use diesel::pg::PgConnection;
use diesel::prelude::*;

use crate::markdown::markers::{MarkerKind, scan_markers};
use crate::markdown::tasks::{ParsedTasks, Task, mark_done, parse_tasks};
use crate::model::{
  Caller, DocumentKind, Role, SpecStatus, StoreError, StoreResult, validate_commit,
};
use crate::schema::specs;
use crate::store::documents::{append_revision, current_revision, document_id};
use crate::store::projects::access_in;
use crate::store::projects::visible_projects;
use crate::store::refusals::Attempt;
use crate::store::specs::{SpecRow, SpecSummary, active_spec_in, list_specs_in, spec_by_number};
use crate::store::{Store, audit};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkedTask {
  pub revision_id: i64,
  pub line: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpecMarker {
  pub document_kind: DocumentKind,
  pub kind: MarkerKind,
  pub heading: String,
  pub text: String,
  pub line: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveStatus {
  pub number: i64,
  pub slug: String,
  pub status: SpecStatus,
  pub branch: String,
  pub tasks_done: usize,
  pub tasks_total: usize,
  pub current_phase: Option<String>,
  pub next_task: Option<Task>,
  pub markers: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectStatus {
  pub constitution_version: Option<String>,
  pub specs: Vec<SpecSummary>,
  pub active: Option<ActiveStatus>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectOverview {
  pub slug: String,
  pub name: String,
  pub role: Option<Role>,
  pub constitution_version: Option<String>,
  pub spec_count: usize,
  pub active: Option<ActiveStatus>,
}

fn constitution_version(conn: &mut PgConnection, project_id: i64) -> StoreResult<Option<String>> {
  match document_id(conn, project_id, None, DocumentKind::Constitution)? {
    Some(doc_id) => Ok(current_revision(conn, doc_id)?.and_then(|(_, meta)| meta.version)),
    None => Ok(None),
  }
}

fn document_body(
  conn: &mut PgConnection,
  project_id: i64,
  spec_id: i64,
  kind: DocumentKind,
) -> StoreResult<Option<String>> {
  match document_id(conn, project_id, Some(spec_id), kind)? {
    Some(doc_id) => Ok(current_revision(conn, doc_id)?.map(|(body, _)| body)),
    None => Ok(None),
  }
}

fn markers_of(
  conn: &mut PgConnection,
  project_id: i64,
  spec_id: i64,
) -> StoreResult<Vec<SpecMarker>> {
  let mut markers = Vec::new();
  for kind in DocumentKind::SPEC_KINDS {
    if let Some(body) = document_body(conn, project_id, spec_id, kind)? {
      markers.extend(scan_markers(&body).into_iter().map(|m| SpecMarker {
        document_kind: kind,
        kind: m.kind,
        heading: m.heading,
        text: m.text,
        line: m.line,
      }));
    }
  }
  Ok(markers)
}

fn active_status(
  conn: &mut PgConnection,
  project_id: i64,
  spec: SpecRow,
) -> StoreResult<ActiveStatus> {
  let parsed = document_body(conn, project_id, spec.id, DocumentKind::Tasks)?
    .map(|body| parse_tasks(&body))
    .unwrap_or_default();
  let tasks_done = parsed.tasks.iter().filter(|t| t.done).count();
  let next_task = parsed.tasks.iter().find(|t| !t.done).cloned();
  let current_phase = next_task
    .as_ref()
    .map(|t| t.phase.clone())
    .filter(|p| !p.is_empty());
  Ok(ActiveStatus {
    number: spec.number,
    slug: spec.slug,
    status: spec.status,
    branch: spec.branch,
    tasks_done,
    tasks_total: parsed.tasks.len(),
    current_phase,
    next_task,
    markers: markers_of(conn, project_id, spec.id)?.len(),
  })
}

impl Store {
  pub fn list_tasks(
    &self,
    caller: &Caller,
    project: &str,
    number: i64,
    pending_only: bool,
  ) -> StoreResult<ParsedTasks> {
    self.with_conn(|conn| {
      let access = access_in(conn, caller, project)?;
      let spec = spec_by_number(conn, access.project_id, project, number)?;
      let body = document_body(conn, access.project_id, spec.id, DocumentKind::Tasks)?
        .ok_or_else(|| StoreError::NotFound(format!("tasks document of spec {number:03}")))?;
      let mut parsed = parse_tasks(&body);
      if pending_only {
        parsed.tasks.retain(|t| !t.done);
      }
      Ok(parsed)
    })
  }

  pub fn list_markers(
    &self,
    caller: &Caller,
    project: &str,
    number: i64,
  ) -> StoreResult<Vec<SpecMarker>> {
    self.with_conn(|conn| {
      let access = access_in(conn, caller, project)?;
      let spec = spec_by_number(conn, access.project_id, project, number)?;
      markers_of(conn, access.project_id, spec.id)
    })
  }

  pub fn project_status(&self, caller: &Caller, project: &str) -> StoreResult<ProjectStatus> {
    self.with_conn(|conn| {
      let access = access_in(conn, caller, project)?;
      let constitution_version = constitution_version(conn, access.project_id)?;
      let specs = list_specs_in(conn, access.project_id)?;
      let active = active_spec_in(conn, access.project_id)?
        .map(|spec| active_status(conn, access.project_id, spec))
        .transpose()?;
      Ok(ProjectStatus {
        constitution_version,
        specs,
        active,
      })
    })
  }

  pub fn project_overview(&self, caller: &Caller) -> StoreResult<Vec<ProjectOverview>> {
    self.with_conn(|conn| {
      let mut overview = Vec::new();
      for project in visible_projects(conn, caller)? {
        let spec_count: i64 = specs::table
          .filter(specs::project_id.eq(project.id))
          .count()
          .get_result(conn)?;
        let active = active_spec_in(conn, project.id)?
          .map(|spec| active_status(conn, project.id, spec))
          .transpose()?;
        overview.push(ProjectOverview {
          slug: project.address.to_string(),
          name: project.name,
          role: project.role,
          constitution_version: constitution_version(conn, project.id)?,
          spec_count: spec_count as usize,
          active,
        });
      }
      Ok(overview)
    })
  }
}

impl Store {
  pub fn mark_task_done(
    &self,
    caller: &Caller,
    project: &str,
    number: i64,
    task_id: &str,
    commit: &str,
  ) -> StoreResult<MarkedTask> {
    validate_commit(commit)?;
    let attempt = Attempt::project("task.done", project, format!("{number:03}/{task_id}"));
    self.audited(caller, attempt, |tx| {
      let access = access_in(tx, caller, project)?;
      if !access.can_write() {
        return Err(StoreError::Forbidden);
      }
      let spec = spec_by_number(tx, access.project_id, project, number)?;
      if spec.status == SpecStatus::Archived {
        return Err(StoreError::Forbidden);
      }
      let missing = || StoreError::NotFound(format!("tasks document of spec {number:03}"));
      let doc_id = document_id(tx, access.project_id, Some(spec.id), DocumentKind::Tasks)?
        .ok_or_else(missing)?;
      let (body, _) = current_revision(tx, doc_id)?.ok_or_else(missing)?;
      let line_number = parse_tasks(&body)
        .tasks
        .iter()
        .find(|t| t.id == task_id)
        .map(|t| t.line);
      let new_body = mark_done(&body, task_id, commit)?;
      let line = line_number
        .and_then(|n| new_body.split('\n').nth(n - 1))
        .map(|l| l.strip_suffix('\r').unwrap_or(l).to_string())
        .unwrap_or_default();
      let message = format!("{task_id}: done, commit {commit}");
      let revision_id = append_revision(tx, doc_id, &new_body, None, caller, &message)?;
      audit(
        tx,
        caller,
        Some(access.project_id),
        "task.done",
        &format!("{number:03}/{task_id}@{commit}"),
      )?;
      Ok(MarkedTask { revision_id, line })
    })
  }
}
