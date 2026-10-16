#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn accepts_valid_slugs_and_handles() {
    for s in ["a", "store-core", "001-store", "one-two-three-four", "x9"] {
      assert!(validate_slug(s).is_ok(), "{s} should be valid");
    }
  }

  #[test]
  fn rejects_invalid_slugs() {
    for s in [
      "",
      "Store",
      "store_core",
      "-store",
      "store-",
      "store--core",
      "one-two-three-four-five",
      "with space",
      "ünicode",
    ] {
      assert!(
        matches!(validate_slug(s), Err(StoreError::Validation(_))),
        "{s:?} should be rejected"
      );
    }
  }

  #[test]
  fn validates_commit_identifiers() {
    assert!(validate_commit("abc1234").is_ok());
    assert!(validate_commit(&"a".repeat(40)).is_ok());
    for c in ["abc123", &"a".repeat(41), "ABC1234", "ghijklm", ""] {
      assert!(
        matches!(validate_commit(c), Err(StoreError::Validation(_))),
        "{c:?} should be rejected"
      );
    }
  }

  #[test]
  fn parses_roles_statuses_and_kinds() {
    assert_eq!("owner".parse::<Role>().unwrap(), Role::Owner);
    assert_eq!("editor".parse::<Role>().unwrap(), Role::Editor);
    assert_eq!("reader".parse::<Role>().unwrap(), Role::Reader);
    assert!("admin".parse::<Role>().is_err());
    assert_eq!(Role::Editor.as_str(), "editor");

    assert_eq!("draft".parse::<SpecStatus>().unwrap(), SpecStatus::Draft);
    assert_eq!("archived".parse::<SpecStatus>().unwrap(), SpecStatus::Archived);
    assert!("done".parse::<SpecStatus>().is_err());
    assert_eq!(SpecStatus::Clarified.as_str(), "clarified");

    assert_eq!("tasks".parse::<DocumentKind>().unwrap(), DocumentKind::Tasks);
    assert_eq!(
      "constitution".parse::<DocumentKind>().unwrap(),
      DocumentKind::Constitution
    );
    assert!("notes".parse::<DocumentKind>().is_err());
    assert_eq!(DocumentKind::SPEC_KINDS.len(), 5);
    assert!(!DocumentKind::SPEC_KINDS.contains(&DocumentKind::Constitution));
  }

  #[test]
  fn kind_parse_error_lists_valid_kinds() {
    let err = "notes".parse::<DocumentKind>().unwrap_err();
    let msg = err.to_string();
    for k in ["spec", "plan", "tasks", "research", "validation", "constitution"] {
      assert!(msg.contains(k), "{msg} should list {k}");
    }
  }

  #[test]
  fn status_transition_table() {
    use SpecStatus::*;
    let editor = Role::Editor;
    let owner = Role::Owner;
    assert!(transition_allowed(Draft, Clarified, editor));
    assert!(transition_allowed(Clarified, Approved, editor));
    assert!(transition_allowed(Clarified, Draft, editor));
    assert!(transition_allowed(Approved, Draft, editor));
    assert!(!transition_allowed(Draft, Approved, editor));
    assert!(!transition_allowed(Approved, Clarified, editor));
    assert!(!transition_allowed(Draft, Draft, editor));
    assert!(transition_allowed(Draft, Archived, owner));
    assert!(transition_allowed(Approved, Archived, owner));
    assert!(!transition_allowed(Draft, Archived, editor));
    assert!(!transition_allowed(Archived, Draft, owner));
    assert!(!transition_allowed(Archived, Archived, owner));
    assert!(!transition_allowed(Draft, Clarified, Role::Reader));
  }

  #[test]
  fn allowed_targets_are_listed_for_errors() {
    let targets = allowed_targets(SpecStatus::Clarified, Role::Owner);
    assert_eq!(
      targets,
      vec![SpecStatus::Draft, SpecStatus::Approved, SpecStatus::Archived]
    );
    assert!(allowed_targets(SpecStatus::Archived, Role::Owner).is_empty());
  }

  #[test]
  fn body_limit_is_one_mebibyte() {
    assert_eq!(MAX_BODY_BYTES, 1_048_576);
    assert!(validate_body_size("x".repeat(MAX_BODY_BYTES).as_str()).is_ok());
    assert!(matches!(
      validate_body_size("x".repeat(MAX_BODY_BYTES + 1).as_str()),
      Err(StoreError::Validation(_))
    ));
  }
}
