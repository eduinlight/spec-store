use std::collections::HashMap;

use crate::markdown::requirements::{RequirementKind, parse_requirements};
use crate::markdown::tasks::{Task, parse_tasks};
use crate::model::{Caller, DocumentKind, StoreResult};
use crate::store::Store;
use crate::store::views::{current_bodies, spec_scope};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Coverage {
  Covered,
  InProgress,
  NotStarted,
  NoTasks,
}

impl Coverage {
  pub fn as_str(self) -> &'static str {
    match self {
      Coverage::Covered => "covered",
      Coverage::InProgress => "in_progress",
      Coverage::NotStarted => "not_started",
      Coverage::NoTasks => "no_tasks",
    }
  }

  fn of(done: usize, total: usize) -> Coverage {
    match (done, total) {
      (_, 0) => Coverage::NoTasks,
      (0, _) => Coverage::NotStarted,
      (done, total) if done == total => Coverage::Covered,
      _ => Coverage::InProgress,
    }
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraceTask {
  pub id: String,
  pub description: String,
  pub done: bool,
  pub commit: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraceRequirement {
  pub id: String,
  pub text: String,
  pub status: Coverage,
  pub tasks: Vec<TraceTask>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Traceability {
  pub requirements: Vec<TraceRequirement>,
  pub tasks_without_requirements: Vec<TraceTask>,
}

fn trace_task(task: &Task) -> TraceTask {
  TraceTask {
    id: task.id.clone(),
    description: task.description.clone(),
    done: task.done,
    commit: task.commit.clone(),
  }
}

/// Requirement rows (functional requirements only, since tasks cite `FR-n`) joined with the tasks that cite them.
pub fn build_traceability(spec_body: Option<&str>, tasks_body: Option<&str>) -> Traceability {
  let tasks = tasks_body
    .map(|body| parse_tasks(body).tasks)
    .unwrap_or_default();
  let mut by_requirement: HashMap<&str, Vec<&Task>> = HashMap::new();
  for task in &tasks {
    for fr in &task.frs {
      by_requirement.entry(fr.as_str()).or_default().push(task);
    }
  }
  let requirements = spec_body
    .map(parse_requirements)
    .unwrap_or_default()
    .into_iter()
    .filter(|r| r.kind == RequirementKind::Functional)
    .map(|requirement| {
      let cited = by_requirement
        .get(requirement.id.as_str())
        .cloned()
        .unwrap_or_default();
      let done = cited.iter().filter(|t| t.done).count();
      TraceRequirement {
        status: Coverage::of(done, cited.len()),
        tasks: cited.into_iter().map(trace_task).collect(),
        id: requirement.id,
        text: requirement.text,
      }
    })
    .collect();
  Traceability {
    requirements,
    tasks_without_requirements: tasks
      .iter()
      .filter(|t| t.frs.is_empty())
      .map(trace_task)
      .collect(),
  }
}

impl Store {
  pub fn spec_traceability(
    &self,
    caller: &Caller,
    project: &str,
    number: i64,
  ) -> StoreResult<Traceability> {
    self.with_conn(|conn| {
      let scope = spec_scope(conn, caller, project, number)?;
      let mut bodies = current_bodies(conn, &[scope.spec.id])?
        .remove(&scope.spec.id)
        .unwrap_or_default();
      Ok(build_traceability(
        bodies.remove(&DocumentKind::Spec).as_deref(),
        bodies.remove(&DocumentKind::Tasks).as_deref(),
      ))
    })
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  const SPEC: &str = "- **FR-1**: one\n- **FR-2**: two\n- **FR-3**: three\n- **FR-4**: four\n- **NFR-1**: nonfunctional\n";
  const TASKS: &str = "## P\n- [x] T01 (FR-1, FR-2) a — Done when: d · commit: abc1234\n- [x] T02 (FR-2) b — Done when: d · commit: def5678\n- [ ] T03 (FR-2, FR-3) c — Done when: d\n- [ ] T04 (FR: —) d — Done when: d\n";

  #[test]
  fn statuses_follow_the_done_flags_of_the_citing_tasks() {
    let trace = build_traceability(Some(SPEC), Some(TASKS));
    let statuses: Vec<(&str, Coverage)> = trace
      .requirements
      .iter()
      .map(|r| (r.id.as_str(), r.status))
      .collect();
    assert_eq!(
      statuses,
      [
        ("FR-1", Coverage::Covered),
        ("FR-2", Coverage::InProgress),
        ("FR-3", Coverage::NotStarted),
        ("FR-4", Coverage::NoTasks),
      ]
    );
    let fr2 = &trace.requirements[1];
    let ids: Vec<&str> = fr2.tasks.iter().map(|t| t.id.as_str()).collect();
    assert_eq!(ids, ["T01", "T02", "T03"]);
    assert_eq!(fr2.tasks[1].commit.as_deref(), Some("def5678"));
    assert!(!fr2.tasks[2].done);
    assert_eq!(trace.tasks_without_requirements.len(), 1);
    assert_eq!(trace.tasks_without_requirements[0].id, "T04");
  }

  #[test]
  fn missing_documents_yield_empty_or_untasked_rows() {
    assert_eq!(build_traceability(None, None), Traceability::default());
    let only_spec = build_traceability(Some(SPEC), None);
    assert!(
      only_spec
        .requirements
        .iter()
        .all(|r| r.status == Coverage::NoTasks)
    );
    let only_tasks = build_traceability(None, Some(TASKS));
    assert!(only_tasks.requirements.is_empty());
    assert_eq!(only_tasks.tasks_without_requirements.len(), 1);
  }

  #[test]
  fn coverage_names_are_stable() {
    assert_eq!(Coverage::Covered.as_str(), "covered");
    assert_eq!(Coverage::InProgress.as_str(), "in_progress");
    assert_eq!(Coverage::NotStarted.as_str(), "not_started");
    assert_eq!(Coverage::NoTasks.as_str(), "no_tasks");
  }
}
