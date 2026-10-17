use crate::model::{StoreError, StoreResult, validate_commit};

const TODO_PREFIX: &str = "- [ ] ";
const DONE_PREFIX: &str = "- [x] ";
const DONE_WHEN: &str = " — Done when: ";
const COMMIT_SUFFIX: &str = " · commit: ";
const HEADING_PREFIX: &str = "## ";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Task {
  pub id: String,
  pub phase: String,
  pub parallel: bool,
  pub frs: Vec<String>,
  pub description: String,
  pub done_when: String,
  pub done: bool,
  pub commit: Option<String>,
  pub line: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Malformed {
  pub line: usize,
  pub text: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ParsedTasks {
  pub tasks: Vec<Task>,
  pub malformed: Vec<Malformed>,
}

fn strip_cr(line: &str) -> &str {
  line.strip_suffix('\r').unwrap_or(line)
}

fn looks_like_task(line: &str) -> bool {
  line.starts_with(TODO_PREFIX) || line.starts_with(DONE_PREFIX)
}

fn valid_task_id(id: &str) -> bool {
  let digits = id.strip_prefix('T').unwrap_or("");
  digits.len() >= 2 && digits.bytes().all(|b| b.is_ascii_digit())
}

fn parse_frs(group: &str) -> Option<Vec<String>> {
  if let Some(none) = group.strip_prefix("FR:") {
    return matches!(none.trim(), "—" | "-" | "–").then(Vec::new);
  }
  let frs: Vec<String> = group.split(',').map(|fr| fr.trim().to_string()).collect();
  let valid = frs.iter().all(|fr| {
    fr.strip_prefix("FR-")
      .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
  });
  valid.then_some(frs)
}

fn parse_task_line(line: &str, number: usize, phase: &str) -> Option<Task> {
  let done = line.starts_with(DONE_PREFIX);
  let rest = &line[TODO_PREFIX.len()..];
  let (id, rest) = rest.split_once(' ')?;
  if !valid_task_id(id) {
    return None;
  }
  let (parallel, rest) = match rest.strip_prefix("[P] ") {
    Some(rest) => (true, rest),
    None => (false, rest),
  };
  let rest = rest.strip_prefix('(')?;
  let close = rest.find(')')?;
  let frs = parse_frs(&rest[..close])?;
  let rest = rest[close + 1..].strip_prefix(' ')?;
  let (description, tail) = rest.split_once(DONE_WHEN)?;
  let (done_when, commit) = match tail.rsplit_once(COMMIT_SUFFIX) {
    Some((done_when, commit)) => {
      let commit = commit.trim();
      validate_commit(commit).ok()?;
      (done_when, Some(commit.to_string()))
    }
    None => (tail, None),
  };
  let description = description.trim();
  let done_when = done_when.trim();
  if description.is_empty() || done_when.is_empty() {
    return None;
  }
  Some(Task {
    id: id.to_string(),
    phase: phase.to_string(),
    parallel,
    frs,
    description: description.to_string(),
    done_when: done_when.to_string(),
    done,
    commit,
    line: number,
  })
}

pub fn parse_tasks(body: &str) -> ParsedTasks {
  let mut parsed = ParsedTasks::default();
  let mut phase = String::new();
  for (index, raw) in body.split('\n').enumerate() {
    let line = strip_cr(raw);
    if let Some(heading) = line.strip_prefix(HEADING_PREFIX) {
      phase = heading.trim().to_string();
      continue;
    }
    if !looks_like_task(line) {
      continue;
    }
    match parse_task_line(line, index + 1, &phase) {
      Some(task) => parsed.tasks.push(task),
      None => parsed.malformed.push(Malformed {
        line: index + 1,
        text: line.to_string(),
      }),
    }
  }
  parsed
}

pub fn mark_done(body: &str, task_id: &str, commit: &str) -> StoreResult<String> {
  let parsed = parse_tasks(body);
  if !parsed.malformed.is_empty() {
    let lines: Vec<String> = parsed
      .malformed
      .iter()
      .map(|m| m.line.to_string())
      .collect();
    return Err(StoreError::Malformed(format!(
      "tasks document has malformed task lines at line(s) {}",
      lines.join(", ")
    )));
  }
  let matching: Vec<&Task> = parsed.tasks.iter().filter(|t| t.id == task_id).collect();
  let task = match matching.as_slice() {
    [] => return Err(StoreError::NotFound(format!("task {task_id}"))),
    [task] => *task,
    many => {
      return Err(StoreError::Malformed(format!(
        "task id {task_id} appears {} times in the tasks document",
        many.len()
      )));
    }
  };
  if task.done {
    return Err(StoreError::Validation(format!(
      "task {task_id} is already done"
    )));
  }
  let lines: Vec<String> = body
    .split('\n')
    .enumerate()
    .map(|(index, raw)| {
      if index + 1 != task.line {
        return raw.to_string();
      }
      let had_cr = raw.ends_with('\r');
      let line = strip_cr(raw);
      let rewritten = format!(
        "{DONE_PREFIX}{}{COMMIT_SUFFIX}{commit}",
        &line[TODO_PREFIX.len()..]
      );
      if had_cr {
        format!("{rewritten}\r")
      } else {
        rewritten
      }
    })
    .collect();
  Ok(lines.join("\n"))
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::model::StoreError;

  const BODY: &str = "# Tasks — Spec 001

Format: `- [ ] Tnn [P] (FR-x, FR-y) Description with path — Done when: verifiable criterion`

## Phase 1 — Setup
- [ ] T01 (FR: —) Create `Cargo.toml` — Done when: `cargo build` passes

## Phase 2 — Foundation
- [x] T02 [P] (FR-16, FR-21, FR-23) Write unit tests in `src/model.rs` — Done when: tests written and red · commit: 4e18ae6
- [ ] T03 (FR-16) Implement `src/model.rs` — Done when: T02 tests green

## Dependencies
- Phase 1 blocks everything.
";

  #[test]
  fn parses_all_fields() {
    let parsed = parse_tasks(BODY);
    assert!(parsed.malformed.is_empty(), "{:?}", parsed.malformed);
    assert_eq!(parsed.tasks.len(), 3);

    let t1 = &parsed.tasks[0];
    assert_eq!(t1.id, "T01");
    assert_eq!(t1.phase, "Phase 1 — Setup");
    assert!(!t1.parallel);
    assert!(t1.frs.is_empty(), "(FR: —) means no FR");
    assert_eq!(t1.description, "Create `Cargo.toml`");
    assert_eq!(t1.done_when, "`cargo build` passes");
    assert!(!t1.done);
    assert_eq!(t1.commit, None);
    assert_eq!(t1.line, 6);

    let t2 = &parsed.tasks[1];
    assert_eq!(t2.id, "T02");
    assert_eq!(t2.phase, "Phase 2 — Foundation");
    assert!(t2.parallel);
    assert_eq!(t2.frs, vec!["FR-16", "FR-21", "FR-23"]);
    assert_eq!(t2.description, "Write unit tests in `src/model.rs`");
    assert_eq!(t2.done_when, "tests written and red");
    assert!(t2.done);
    assert_eq!(t2.commit.as_deref(), Some("4e18ae6"));
    assert_eq!(t2.line, 9);

    let t3 = &parsed.tasks[2];
    assert_eq!(t3.id, "T03");
    assert_eq!(t3.frs, vec!["FR-16"]);
    assert!(!t3.parallel);
    assert_eq!(t3.line, 10);
  }

  #[test]
  fn handles_crlf_bodies() {
    let body = BODY.replace('\n', "\r\n");
    let parsed = parse_tasks(&body);
    assert_eq!(parsed.tasks.len(), 3);
    assert!(parsed.malformed.is_empty());
    assert_eq!(parsed.tasks[2].done_when, "T02 tests green");
    let marked = mark_done(&body, "T03", "abcdef1").unwrap();
    assert!(marked.contains("\r\n"));
    assert!(!marked.contains("\n\n\n"));
    assert!(marked.contains("- [x] T03 (FR-16) Implement `src/model.rs` — Done when: T02 tests green · commit: abcdef1\r\n"));
  }

  #[test]
  fn reports_malformed_lines_with_numbers() {
    let body = "## Phase 1\n- [ ] T01 (FR-1) ok — Done when: fine\n- [ ] T02 missing done when\n- [ ] no id (FR-1) x — Done when: y\n- [x] T04 (FR-1) done without commit is fine — Done when: z\n- [ ] T05 [P] [P] (FR-1) twice — Done when: z\n- [ ] T06 (FR-1) bad commit — Done when: z · commit: xyz\n";
    let parsed = parse_tasks(body);
    let ids: Vec<&str> = parsed.tasks.iter().map(|t| t.id.as_str()).collect();
    assert_eq!(ids, vec!["T01", "T04"]);
    let lines: Vec<usize> = parsed.malformed.iter().map(|m| m.line).collect();
    assert_eq!(lines, vec![3, 4, 6, 7]);
    assert_eq!(parsed.malformed[0].text, "- [ ] T02 missing done when");
  }

  #[test]
  fn mark_done_rewrites_exactly_one_line() {
    let marked = mark_done(BODY, "T03", "0123abc").unwrap();
    let before: Vec<&str> = BODY.lines().collect();
    let after: Vec<&str> = marked.lines().collect();
    assert_eq!(before.len(), after.len());
    let changed: Vec<usize> = (0..before.len())
      .filter(|i| before[*i] != after[*i])
      .collect();
    assert_eq!(changed, vec![9]);
    assert_eq!(
      after[9],
      "- [x] T03 (FR-16) Implement `src/model.rs` — Done when: T02 tests green · commit: 0123abc"
    );
    let reparsed = parse_tasks(&marked);
    assert!(reparsed.tasks[2].done);
    assert_eq!(reparsed.tasks[2].commit.as_deref(), Some("0123abc"));
  }

  #[test]
  fn mark_done_rejects_unknown_done_and_duplicates() {
    assert!(matches!(
      mark_done(BODY, "T09", "0123abc"),
      Err(StoreError::NotFound(_))
    ));
    match mark_done(BODY, "T02", "0123abc") {
      Err(StoreError::Validation(msg)) => {
        assert!(msg.contains("T02") && msg.contains("already"), "{msg}")
      }
      other => panic!("expected validation, got {other:?}"),
    }
    let duplicated = format!("{BODY}- [ ] T03 (FR-2) again — Done when: dup\n");
    assert!(matches!(
      mark_done(&duplicated, "T03", "0123abc"),
      Err(StoreError::Malformed(_))
    ));
    let with_malformed = format!("{BODY}- [ ] T07 broken line\n");
    assert!(matches!(
      mark_done(&with_malformed, "T03", "0123abc"),
      Err(StoreError::Malformed(_))
    ));
  }

  #[test]
  fn tasks_before_any_heading_have_an_empty_phase() {
    let parsed = parse_tasks("- [ ] T01 (FR-1) x — Done when: y\n");
    assert_eq!(parsed.tasks[0].phase, "");
  }
}
