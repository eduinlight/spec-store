use crate::address::ProjectRef;
use crate::markdown::tasks::{Task, parse_tasks};
use crate::model::{Caller, DocumentKind, SpecStatus, StoreResult};
use crate::store::Store;
use crate::store::projects::access_in;
use crate::store::tasks::SpecMarker;
use crate::store::views::spec_list::{spec_summary, spec_title};
use crate::store::views::{LiveSpec, live_specs};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NextTask {
  pub id: String,
  pub parallel: bool,
  pub frs: Vec<String>,
  pub description: String,
  pub done_when: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveSpec {
  pub number: i64,
  pub slug: String,
  pub title: String,
  pub status: SpecStatus,
  pub branch: String,
  pub tasks_done: usize,
  pub tasks_total: usize,
  pub current_phase: Option<String>,
  pub open_markers: usize,
  pub next_task: Option<NextTask>,
  pub agent_prompt: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DraftSpec {
  pub number: i64,
  pub slug: String,
  pub title: String,
  pub summary: String,
  pub markers: Vec<SpecMarker>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ActiveSpecs {
  pub active: Option<ActiveSpec>,
  pub drafts: Vec<DraftSpec>,
}

fn title_of(spec: &LiveSpec) -> String {
  spec
    .body(DocumentKind::Spec)
    .and_then(spec_title)
    .unwrap_or_else(|| spec.row.slug.clone())
}

/// Instruction an agent can follow to work on a task of a spec.
fn agent_prompt(project: &str, spec: &LiveSpec, task: &Task) -> String {
  format!(
    "Work on SDD Vault project {project}, spec {:03}-{}, task {}: {}. Done when: {}. Read the spec, plan and task through the sdd-vault MCP server first, follow the constitution, write tests first, and mark the task done with its commit.",
    spec.row.number, spec.row.slug, task.id, task.description, task.done_when
  )
}

fn active_of(project: &str, spec: &LiveSpec) -> ActiveSpec {
  let tasks = spec
    .body(DocumentKind::Tasks)
    .map(|body| parse_tasks(body).tasks)
    .unwrap_or_default();
  let next = tasks.iter().find(|t| !t.done);
  ActiveSpec {
    number: spec.row.number,
    slug: spec.row.slug.clone(),
    title: title_of(spec),
    status: spec.row.status,
    branch: spec.row.branch.clone(),
    tasks_done: tasks.iter().filter(|t| t.done).count(),
    tasks_total: tasks.len(),
    current_phase: next.map(|t| t.phase.clone()).filter(|p| !p.is_empty()),
    open_markers: spec.markers().len(),
    next_task: next.map(|t| NextTask {
      id: t.id.clone(),
      parallel: t.parallel,
      frs: t.frs.clone(),
      description: t.description.clone(),
      done_when: t.done_when.clone(),
    }),
    agent_prompt: next.map(|t| agent_prompt(project, spec, t)),
  }
}

impl Store {
  /// The active spec with its next task and agent prompt, and the other drafts with their open markers.
  pub fn active_specs(&self, caller: &Caller, project: &str) -> StoreResult<ActiveSpecs> {
    self.with_conn(|conn| {
      let access = access_in(conn, caller, project)?;
      let address = ProjectRef::parse(project)?.to_string();
      let specs = live_specs(conn, access.project_id)?;
      let active = specs.last();
      let drafts = specs
        .iter()
        .filter(|spec| spec.row.status == SpecStatus::Draft)
        .filter(|spec| active.is_none_or(|a| a.row.number != spec.row.number))
        .map(|spec| DraftSpec {
          number: spec.row.number,
          slug: spec.row.slug.clone(),
          title: title_of(spec),
          summary: spec
            .body(DocumentKind::Spec)
            .map(spec_summary)
            .unwrap_or_default(),
          markers: spec.markers(),
        })
        .collect();
      Ok(ActiveSpecs {
        active: active.map(|spec| active_of(&address, spec)),
        drafts,
      })
    })
  }
}
