use std::fmt;
use std::str::FromStr;

pub const MAX_BODY_BYTES: usize = 1_048_576;

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
  #[error("authentication required")]
  Unauthenticated,
  #[error("not allowed")]
  Forbidden,
  #[error("{0} not found")]
  NotFound(String),
  #[error("revision conflict: current revision is {current_revision}")]
  Conflict { current_revision: i64 },
  #[error("{0}")]
  Validation(String),
  #[error("{0}")]
  Malformed(String),
  #[error("database error: {0}")]
  Database(#[from] diesel::result::Error),
  #[error("database pool error: {0}")]
  Pool(#[from] diesel::r2d2::PoolError),
  #[error("migration step {step} failed: {detail}")]
  Migration { step: String, detail: String },
  #[error("{0} is unavailable")]
  Unavailable(String),
}

pub type StoreResult<T> = Result<T, StoreError>;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Via {
  pub client: String,
  pub origin: Option<String>,
}

impl Via {
  pub fn new(client: impl Into<String>, origin: Option<String>) -> Self {
    Via {
      client: client.into(),
      origin,
    }
  }

  pub fn client_name(&self) -> Option<&str> {
    Some(self.client.as_str()).filter(|c| !c.is_empty())
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Caller {
  pub user_id: i64,
  pub handle: String,
  pub is_operator: bool,
  pub via: Via,
}

impl Caller {
  pub fn with_via(mut self, via: Via) -> Self {
    self.via = via;
    self
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AuditResult {
  Ok,
  Denied,
  Conflict,
}

impl AuditResult {
  pub const ALL: [AuditResult; 3] = [AuditResult::Ok, AuditResult::Denied, AuditResult::Conflict];

  pub fn as_str(self) -> &'static str {
    match self {
      AuditResult::Ok => "ok",
      AuditResult::Denied => "denied",
      AuditResult::Conflict => "conflict",
    }
  }
}

impl FromStr for AuditResult {
  type Err = StoreError;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    AuditResult::ALL
      .into_iter()
      .find(|r| r.as_str() == s)
      .ok_or_else(|| {
        StoreError::Validation(format!(
          "unknown audit result {s:?}; valid results: ok, denied, conflict"
        ))
      })
  }
}

impl fmt::Display for AuditResult {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str(self.as_str())
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OrgRole {
  Owner,
  Member,
}

impl OrgRole {
  pub const ALL: [OrgRole; 2] = [OrgRole::Owner, OrgRole::Member];

  pub fn as_str(self) -> &'static str {
    match self {
      OrgRole::Owner => "owner",
      OrgRole::Member => "member",
    }
  }
}

impl FromStr for OrgRole {
  type Err = StoreError;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    OrgRole::ALL
      .into_iter()
      .find(|r| r.as_str() == s)
      .ok_or_else(|| {
        StoreError::Validation(format!(
          "unknown organization role {s:?}; valid roles: owner, member"
        ))
      })
  }
}

impl fmt::Display for OrgRole {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str(self.as_str())
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Role {
  Owner,
  Editor,
  Reader,
}

impl Role {
  pub const ALL: [Role; 3] = [Role::Owner, Role::Editor, Role::Reader];

  pub fn as_str(self) -> &'static str {
    match self {
      Role::Owner => "owner",
      Role::Editor => "editor",
      Role::Reader => "reader",
    }
  }

  pub fn can_write(self) -> bool {
    matches!(self, Role::Owner | Role::Editor)
  }
}

impl FromStr for Role {
  type Err = StoreError;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    Role::ALL
      .into_iter()
      .find(|r| r.as_str() == s)
      .ok_or_else(|| {
        StoreError::Validation(format!(
          "unknown role {s:?}; valid roles: owner, editor, reader"
        ))
      })
  }
}

impl fmt::Display for Role {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str(self.as_str())
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SpecStatus {
  Draft,
  Clarified,
  Approved,
  Archived,
}

impl SpecStatus {
  pub const ALL: [SpecStatus; 4] = [
    SpecStatus::Draft,
    SpecStatus::Clarified,
    SpecStatus::Approved,
    SpecStatus::Archived,
  ];

  pub fn as_str(self) -> &'static str {
    match self {
      SpecStatus::Draft => "draft",
      SpecStatus::Clarified => "clarified",
      SpecStatus::Approved => "approved",
      SpecStatus::Archived => "archived",
    }
  }
}

impl FromStr for SpecStatus {
  type Err = StoreError;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    SpecStatus::ALL
      .into_iter()
      .find(|v| v.as_str() == s)
      .ok_or_else(|| {
        StoreError::Validation(format!(
          "unknown status {s:?}; valid statuses: draft, clarified, approved, archived"
        ))
      })
  }
}

impl fmt::Display for SpecStatus {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str(self.as_str())
  }
}

pub fn transition_allowed(from: SpecStatus, to: SpecStatus, role: Role) -> bool {
  use SpecStatus::*;
  if !role.can_write() || from == Archived {
    return false;
  }
  match (from, to) {
    (_, Archived) => role == Role::Owner,
    (Draft, Clarified) | (Clarified, Approved) | (Clarified, Draft) | (Approved, Draft) => true,
    _ => false,
  }
}

pub fn allowed_targets(from: SpecStatus, role: Role) -> Vec<SpecStatus> {
  SpecStatus::ALL
    .into_iter()
    .filter(|to| transition_allowed(from, *to, role))
    .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DocumentKind {
  Constitution,
  Spec,
  Plan,
  Tasks,
  Research,
  Validation,
}

impl DocumentKind {
  pub const ALL: [DocumentKind; 6] = [
    DocumentKind::Constitution,
    DocumentKind::Spec,
    DocumentKind::Plan,
    DocumentKind::Tasks,
    DocumentKind::Research,
    DocumentKind::Validation,
  ];

  pub const SPEC_KINDS: [DocumentKind; 5] = [
    DocumentKind::Spec,
    DocumentKind::Plan,
    DocumentKind::Tasks,
    DocumentKind::Research,
    DocumentKind::Validation,
  ];

  pub fn as_str(self) -> &'static str {
    match self {
      DocumentKind::Constitution => "constitution",
      DocumentKind::Spec => "spec",
      DocumentKind::Plan => "plan",
      DocumentKind::Tasks => "tasks",
      DocumentKind::Research => "research",
      DocumentKind::Validation => "validation",
    }
  }
}

impl FromStr for DocumentKind {
  type Err = StoreError;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    DocumentKind::ALL
      .into_iter()
      .find(|k| k.as_str() == s)
      .ok_or_else(|| {
        StoreError::Validation(format!(
          "unknown document kind {s:?}; valid kinds: constitution, spec, plan, tasks, research, validation"
        ))
      })
  }
}

impl fmt::Display for DocumentKind {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str(self.as_str())
  }
}

pub fn validate_slug(slug: &str) -> StoreResult<()> {
  let words: Vec<&str> = slug.split('-').collect();
  let word_ok = |w: &&str| {
    !w.is_empty()
      && w
        .bytes()
        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
  };
  if slug.is_empty() || words.len() > 4 || !words.iter().all(word_ok) {
    return Err(StoreError::Validation(format!(
      "invalid slug {slug:?}: one to four lowercase words of letters and digits joined by single hyphens"
    )));
  }
  Ok(())
}

pub fn validate_commit(commit: &str) -> StoreResult<()> {
  let hex = commit
    .bytes()
    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b));
  if !(7..=40).contains(&commit.len()) || !hex {
    return Err(StoreError::Validation(format!(
      "invalid commit identifier {commit:?}: 7 to 40 lowercase hexadecimal characters"
    )));
  }
  Ok(())
}

pub fn validate_body(body: &str) -> StoreResult<()> {
  if body.len() > MAX_BODY_BYTES {
    return Err(StoreError::Validation(format!(
      "document body of {} bytes exceeds the limit of {MAX_BODY_BYTES} bytes",
      body.len()
    )));
  }
  if let Some(offset) = body.find('\0') {
    return Err(StoreError::Validation(format!(
      "document body contains a NUL character at byte {offset}; markdown documents cannot contain NUL"
    )));
  }
  Ok(())
}

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
    assert_eq!(
      "archived".parse::<SpecStatus>().unwrap(),
      SpecStatus::Archived
    );
    assert!("done".parse::<SpecStatus>().is_err());
    assert_eq!(SpecStatus::Clarified.as_str(), "clarified");

    assert_eq!(
      "tasks".parse::<DocumentKind>().unwrap(),
      DocumentKind::Tasks
    );
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
    for k in [
      "spec",
      "plan",
      "tasks",
      "research",
      "validation",
      "constitution",
    ] {
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
      vec![
        SpecStatus::Draft,
        SpecStatus::Approved,
        SpecStatus::Archived
      ]
    );
    assert!(allowed_targets(SpecStatus::Archived, Role::Owner).is_empty());
  }

  #[test]
  fn body_limit_is_one_mebibyte() {
    assert_eq!(MAX_BODY_BYTES, 1_048_576);
    assert!(validate_body("x".repeat(MAX_BODY_BYTES).as_str()).is_ok());
    assert!(matches!(
      validate_body("x".repeat(MAX_BODY_BYTES + 1).as_str()),
      Err(StoreError::Validation(_))
    ));
  }
}
