use std::collections::{HashMap, HashSet};

use crate::markdown::tasks::{ParsedTasks, parse_tasks};
use crate::model::{Caller, DocumentKind, StoreError, StoreResult};
use crate::store::Store;
use crate::store::documents::{current_revision, document_id};
use crate::store::views::{RevisionRecord, revision_records, spec_scope};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskCompletion {
  pub revision_id: i64,
  pub author: String,
  pub client: Option<String>,
  pub at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpecTasks {
  pub parsed: ParsedTasks,
  pub completions: HashMap<String, TaskCompletion>,
}

/// For each task done in the last revision, the revision where its current done streak began (`[ ]` to `[x]`, or first appearance as done).
pub fn completions(revisions: &[RevisionRecord]) -> HashMap<String, TaskCompletion> {
  let mut current: HashMap<String, TaskCompletion> = HashMap::new();
  let mut previously_done: HashSet<String> = HashSet::new();
  for revision in revisions {
    let done_now: HashSet<String> = parse_tasks(&revision.body)
      .tasks
      .into_iter()
      .filter(|task| task.done)
      .map(|task| task.id)
      .collect();
    current.retain(|id, _| done_now.contains(id));
    for id in &done_now {
      if !previously_done.contains(id) {
        current.insert(
          id.clone(),
          TaskCompletion {
            revision_id: revision.id,
            author: revision.author.clone(),
            client: revision.client.clone(),
            at: revision.at.clone(),
          },
        );
      }
    }
    previously_done = done_now;
  }
  current
}

impl Store {
  /// Tasks of a spec with the attribution of the revision that completed each done task.
  pub fn spec_tasks(&self, caller: &Caller, project: &str, number: i64) -> StoreResult<SpecTasks> {
    self.with_conn(|conn| {
      let scope = spec_scope(conn, caller, project, number)?;
      let missing = || StoreError::NotFound(format!("tasks document of spec {number:03}"));
      let doc_id = document_id(
        conn,
        scope.project_id,
        Some(scope.spec.id),
        DocumentKind::Tasks,
      )?
      .ok_or_else(missing)?;
      let (body, _) = current_revision(conn, doc_id)?.ok_or_else(missing)?;
      let parsed = parse_tasks(&body);
      let completions = if parsed.tasks.iter().any(|task| task.done) {
        completions(&revision_records(conn, doc_id)?)
      } else {
        HashMap::new()
      };
      Ok(SpecTasks {
        parsed,
        completions,
      })
    })
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn revision(id: i64, author: &str, client: Option<&str>, body: &str) -> RevisionRecord {
    RevisionRecord {
      id,
      body: body.to_string(),
      author: author.to_string(),
      client: client.map(str::to_string),
      at: format!("2026-10-24T10:00:0{id}Z"),
    }
  }

  const T1: &str = "- [ ] T01 (FR-1) a — Done when: d\n- [ ] T02 (FR-1) b — Done when: d\n";
  const T1_DONE: &str =
    "- [x] T01 (FR-1) a — Done when: d · commit: abc1234\n- [ ] T02 (FR-1) b — Done when: d\n";
  const BOTH_DONE: &str = "- [x] T01 (FR-1) a — Done when: d · commit: abc1234\n- [x] T02 (FR-1) b — Done when: d · commit: def5678\n";

  #[test]
  fn attributes_each_task_to_the_revision_that_checked_it() {
    let history = [
      revision(1, "ana", None, T1),
      revision(2, "bob", Some("Cursor"), T1_DONE),
      revision(3, "ana", Some("Claude Code"), BOTH_DONE),
      revision(4, "cy", None, &format!("{BOTH_DONE}\nextra line\n")),
    ];
    let found = completions(&history);
    assert_eq!(found["T01"].author, "bob");
    assert_eq!(found["T01"].client.as_deref(), Some("Cursor"));
    assert_eq!(found["T01"].revision_id, 2);
    assert_eq!(found["T02"].author, "ana");
    assert_eq!(found["T02"].client.as_deref(), Some("Claude Code"));
    assert_eq!(found["T02"].at, "2026-10-24T10:00:03Z");
  }

  #[test]
  fn undone_tasks_have_no_completion_and_redone_ones_use_the_latest_streak() {
    let history = [
      revision(1, "ana", None, T1),
      revision(2, "bob", None, T1_DONE),
      revision(3, "cy", None, T1),
      revision(4, "dee", None, T1_DONE),
    ];
    let found = completions(&history);
    assert_eq!(found.len(), 1);
    assert_eq!(found["T01"].author, "dee");
    assert!(completions(&history[..1]).is_empty());
    assert!(completions(&history[..3]).is_empty());
  }

  #[test]
  fn a_task_that_first_appears_done_belongs_to_that_revision() {
    let history = [revision(1, "ana", Some("CLI"), T1_DONE)];
    assert_eq!(completions(&history)["T01"].author, "ana");
    assert!(completions(&[]).is_empty());
  }

  #[test]
  fn two_hundred_revisions_are_walked_in_one_pass() {
    let mut history = Vec::new();
    for n in 1..=200 {
      let mut body = String::new();
      for task in 1..=40 {
        let done = task <= n / 5;
        body.push_str(&format!(
          "- [{}] T{task:02} (FR-1) task {task} — Done when: d{}\n",
          if done { "x" } else { " " },
          if done { " · commit: abc1234" } else { "" }
        ));
      }
      history.push(revision(n, "ana", None, &body));
    }
    let started = std::time::Instant::now();
    let found = completions(&history);
    assert_eq!(found.len(), 40);
    assert_eq!(found["T01"].revision_id, 5);
    assert_eq!(found["T40"].revision_id, 200);
    assert!(started.elapsed() < std::time::Duration::from_secs(2));
  }
}
