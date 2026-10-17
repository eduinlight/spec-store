use std::io;
use std::path::Path;

use crate::model::{DocumentKind, validate_slug};
use crate::store::specs::branch_name;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeFile {
  pub relative_path: String,
  pub body: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeSpec {
  pub folder: String,
  pub number: i64,
  pub slug: String,
  pub documents: Vec<(DocumentKind, TreeFile)>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ProjectTree {
  pub constitution: Option<TreeFile>,
  pub specs: Vec<TreeSpec>,
  pub skipped: Vec<String>,
  pub problems: Vec<String>,
}

pub fn scan(_dir: &Path) -> io::Result<ProjectTree> {
  Ok(ProjectTree::default())
}

pub fn parse_spec_folder(name: &str) -> Option<(i64, String)> {
  let (digits, slug) = name.split_once('-')?;
  if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
    return None;
  }
  let number: i64 = digits.parse().ok()?;
  if number < 1 || validate_slug(slug).is_err() || branch_name(number, slug) != name {
    return None;
  }
  Some((number, slug.to_string()))
}

fn first_line_after<'a>(body: &'a str, prefix: &str) -> Option<&'a str> {
  body
    .lines()
    .find_map(|line| line.strip_prefix(prefix))
    .and_then(|rest| rest.split_whitespace().next())
}

pub fn status_word(body: &str) -> Option<&str> {
  first_line_after(body, "Status:")
}

pub fn version_line(body: &str) -> Option<&str> {
  first_line_after(body, "Version ")
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn accepts_canonical_spec_folders() {
    assert_eq!(
      parse_spec_folder("001-store-core"),
      Some((1, "store-core".to_string()))
    );
    assert_eq!(parse_spec_folder("042-a"), Some((42, "a".to_string())));
    assert_eq!(parse_spec_folder("1000-x"), Some((1000, "x".to_string())));
  }

  #[test]
  fn rejects_folders_that_cannot_round_trip() {
    for name in [
      "0001-x",
      "01-x",
      "000-x",
      "001-Name",
      "001-a-b-c-d-e",
      "001",
      "001-",
      "abc-x",
      "001-a/b",
      "001-..",
      "",
    ] {
      assert_eq!(parse_spec_folder(name), None, "{name:?} should be rejected");
    }
  }

  #[test]
  fn reads_the_status_word() {
    assert_eq!(
      status_word("# Spec 001\n\nStatus: approved · Created: 2026-10-17\n"),
      Some("approved")
    );
    assert_eq!(
      status_word("Status: draft | clarified | approved · Created: <YYYY-MM-DD>\n"),
      Some("draft")
    );
    assert_eq!(status_word("Status:clarified\r\n"), Some("clarified"));
    assert_eq!(status_word("# Spec\n\nStatus: \nStatus: approved\n"), None);
    assert_eq!(status_word("# Spec\n\nno status here\n"), None);
    assert_eq!(status_word("  Status: approved\n"), None);
  }

  #[test]
  fn reads_the_version_token() {
    assert_eq!(
      version_line("# C\n\nVersion 1.0 · Ratified 2026-10-17\n"),
      Some("1.0")
    );
    assert_eq!(version_line("Version 2.1\r\n"), Some("2.1"));
    assert_eq!(version_line("# C\n\nRatified only\n"), None);
    assert_eq!(version_line("Version \n"), None);
    assert_eq!(version_line("Versions 1.0\n"), None);
  }
}
