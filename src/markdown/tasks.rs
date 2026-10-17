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
