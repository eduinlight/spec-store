use std::fmt;

use crate::model::{StoreError, StoreResult, validate_slug};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ProjectRef {
  pub organization: String,
  pub project: String,
}

impl ProjectRef {
  pub fn new(organization: &str, project: &str) -> ProjectRef {
    ProjectRef {
      organization: organization.to_string(),
      project: project.to_string(),
    }
  }

  pub fn parse(address: &str) -> StoreResult<ProjectRef> {
    let invalid = || {
      StoreError::Validation(format!(
        "invalid project address {address:?}: write it as <organization>/<project>"
      ))
    };
    let (organization, project) = address.split_once('/').ok_or_else(invalid)?;
    if validate_slug(organization).is_err() || validate_slug(project).is_err() {
      return Err(invalid());
    }
    Ok(ProjectRef::new(organization, project))
  }
}

impl fmt::Display for ProjectRef {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}/{}", self.organization, self.project)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn parses_and_formats_addresses() {
    let address = ProjectRef::parse("acme/sdd-vault").unwrap();
    assert_eq!(address.organization, "acme");
    assert_eq!(address.project, "sdd-vault");
    assert_eq!(address.to_string(), "acme/sdd-vault");
  }

  #[test]
  fn rejects_malformed_addresses() {
    for address in [
      "demo",
      "acme/",
      "/demo",
      "acme/demo/x",
      "Acme/demo",
      "acme/de mo",
      "",
    ] {
      assert!(
        matches!(ProjectRef::parse(address), Err(StoreError::Validation(_))),
        "{address:?} should be rejected"
      );
    }
  }
}
