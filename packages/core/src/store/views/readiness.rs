use diesel::pg::PgConnection;

use crate::markdown::markers::scan_markers;
use crate::markdown::tasks::parse_tasks;
use crate::model::{Caller, DocumentKind, StoreResult};
use crate::store::Store;
use crate::store::documents::document_id;
use crate::store::views::traceability::{Coverage, build_traceability};
use crate::store::views::{current_bodies, spec_scope};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadinessKey {
  OpenMarkersResolved,
  RequirementsCovered,
  PlanWritten,
  TasksWritten,
  ConstitutionExists,
}

impl ReadinessKey {
  pub fn as_str(self) -> &'static str {
    match self {
      ReadinessKey::OpenMarkersResolved => "open_markers_resolved",
      ReadinessKey::RequirementsCovered => "requirements_covered",
      ReadinessKey::PlanWritten => "plan_written",
      ReadinessKey::TasksWritten => "tasks_written",
      ReadinessKey::ConstitutionExists => "constitution_exists",
    }
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadinessItem {
  pub key: ReadinessKey,
  pub ok: bool,
  pub count: Option<usize>,
  pub total: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Readiness {
  pub ready: bool,
  pub items: Vec<ReadinessItem>,
}

fn item(key: ReadinessKey, ok: bool, count: Option<usize>, total: Option<usize>) -> ReadinessItem {
  ReadinessItem {
    key,
    ok,
    count,
    total,
  }
}

pub struct ReadinessInput<'a> {
  pub spec: Option<&'a str>,
  pub plan: Option<&'a str>,
  pub tasks: Option<&'a str>,
  pub others: Vec<&'a str>,
  pub constitution_exists: bool,
}

/// Checklist items, in display order: markers, requirement coverage, plan, tasks, constitution.
pub fn build_readiness(input: &ReadinessInput<'_>) -> Readiness {
  let open_markers: usize = [input.spec, input.plan, input.tasks]
    .into_iter()
    .flatten()
    .chain(input.others.iter().copied())
    .map(|body| scan_markers(body).len())
    .sum();
  let trace = build_traceability(input.spec, input.tasks);
  let total = trace.requirements.len();
  let covered = trace
    .requirements
    .iter()
    .filter(|r| r.status != Coverage::NoTasks)
    .count();
  let task_count = input
    .tasks
    .map(|body| parse_tasks(body).tasks.len())
    .unwrap_or(0);
  let plan_written = input.plan.is_some_and(|body| !body.trim().is_empty());
  let items = vec![
    item(
      ReadinessKey::OpenMarkersResolved,
      open_markers == 0,
      Some(open_markers),
      None,
    ),
    item(
      ReadinessKey::RequirementsCovered,
      total > 0 && covered == total,
      Some(covered),
      Some(total),
    ),
    item(ReadinessKey::PlanWritten, plan_written, None, None),
    item(
      ReadinessKey::TasksWritten,
      task_count > 0,
      Some(task_count),
      None,
    ),
    item(
      ReadinessKey::ConstitutionExists,
      input.constitution_exists,
      None,
      None,
    ),
  ];
  Readiness {
    ready: items.iter().all(|i| i.ok),
    items,
  }
}

fn constitution_exists(conn: &mut PgConnection, project_id: i64) -> StoreResult<bool> {
  Ok(document_id(conn, project_id, None, DocumentKind::Constitution)?.is_some())
}

impl Store {
  pub fn spec_readiness(
    &self,
    caller: &Caller,
    project: &str,
    number: i64,
  ) -> StoreResult<Readiness> {
    self.with_conn(|conn| {
      let scope = spec_scope(conn, caller, project, number)?;
      let bodies = current_bodies(conn, &[scope.spec.id])?
        .remove(&scope.spec.id)
        .unwrap_or_default();
      let body = |kind: DocumentKind| bodies.get(&kind).map(String::as_str);
      Ok(build_readiness(&ReadinessInput {
        spec: body(DocumentKind::Spec),
        plan: body(DocumentKind::Plan),
        tasks: body(DocumentKind::Tasks),
        others: [DocumentKind::Research, DocumentKind::Validation]
          .into_iter()
          .filter_map(body)
          .collect(),
        constitution_exists: constitution_exists(conn, scope.project_id)?,
      }))
    })
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn input<'a>(
    spec: Option<&'a str>,
    plan: Option<&'a str>,
    tasks: Option<&'a str>,
  ) -> ReadinessInput<'a> {
    ReadinessInput {
      spec,
      plan,
      tasks,
      others: Vec::new(),
      constitution_exists: true,
    }
  }

  #[test]
  fn everything_in_place_is_ready() {
    let readiness = build_readiness(&input(
      Some("- **FR-1**: one\n"),
      Some("# Plan\n"),
      Some("- [ ] T01 (FR-1) a — Done when: d\n"),
    ));
    assert!(readiness.ready);
    assert_eq!(readiness.items.len(), 5);
    assert_eq!(readiness.items[1].count, Some(1));
    assert_eq!(readiness.items[1].total, Some(1));
  }

  #[test]
  fn each_missing_piece_flips_its_own_item() {
    let readiness = build_readiness(&ReadinessInput {
      constitution_exists: false,
      ..input(
        Some("- **FR-1**: one [NEEDS CLARIFICATION: q]\n- **FR-2**: two\n"),
        None,
        Some("- [ ] T01 (FR-1) a — Done when: d\n"),
      )
    });
    assert!(!readiness.ready);
    let flags: Vec<(&str, bool)> = readiness
      .items
      .iter()
      .map(|i| (i.key.as_str(), i.ok))
      .collect();
    assert_eq!(
      flags,
      [
        ("open_markers_resolved", false),
        ("requirements_covered", false),
        ("plan_written", false),
        ("tasks_written", true),
        ("constitution_exists", false),
      ]
    );
    assert_eq!(readiness.items[0].count, Some(1));
    assert_eq!(readiness.items[1].count, Some(1));
    assert_eq!(readiness.items[1].total, Some(2));
  }

  #[test]
  fn markers_in_any_document_count() {
    let mut with_research = input(Some("# s\n"), Some("# p\n"), None);
    with_research.others = vec!["[NEEDS RESEARCH: x]"];
    let readiness = build_readiness(&with_research);
    assert_eq!(readiness.items[0].count, Some(1));
    assert!(!readiness.items[0].ok);
    assert!(!readiness.items[1].ok);
    assert!(!readiness.items[3].ok);
  }
}
