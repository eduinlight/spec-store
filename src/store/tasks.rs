use crate::markdown::tasks::{mark_done, parse_tasks};
use crate::model::{Caller, DocumentKind, SpecStatus, StoreError, StoreResult, validate_commit};
use crate::store::documents::{append_revision, current_revision, document_id};
use crate::store::projects::access_in;
use crate::store::specs::spec_by_number;
use crate::store::{Store, audit};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkedTask {
  pub revision_id: i64,
  pub line: String,
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
    self.with_tx(|tx| {
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
      let revision_id = append_revision(tx, doc_id, &new_body, None, caller.user_id, &message)?;
      audit(
        tx,
        caller.user_id,
        Some(access.project_id),
        "task.done",
        &format!("{number:03}/{task_id}@{commit}"),
      )?;
      Ok(MarkedTask { revision_id, line })
    })
  }
}
