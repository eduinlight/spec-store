use std::io;
use std::path::Path;

use crate::model::{DocumentKind, MAX_BODY_BYTES, validate_slug};
use crate::store::specs::branch_name;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TreeFile {
  pub relative_path: String,
  pub body: String,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TreeSpec {
  pub folder: String,
  pub number: i64,
  pub slug: String,
  pub documents: Vec<(DocumentKind, TreeFile)>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub struct ProjectTree {
  pub constitution: Option<TreeFile>,
  pub specs: Vec<TreeSpec>,
  pub skipped: Vec<String>,
  pub problems: Vec<String>,
}

pub const CONSTITUTION_PATH: &str = "docs/constitution.md";

fn sorted_entries(dir: &Path) -> io::Result<Vec<(String, bool)>> {
  let mut entries = Vec::new();
  for entry in std::fs::read_dir(dir)? {
    let entry = entry?;
    let is_dir = entry.path().is_dir();
    entries.push((entry.file_name().to_string_lossy().into_owned(), is_dir));
  }
  entries.sort();
  Ok(entries)
}

fn read_document(
  root: &Path,
  relative_path: &str,
  problems: &mut Vec<String>,
) -> io::Result<Option<TreeFile>> {
  let bytes = std::fs::read(root.join(relative_path))?;
  if bytes.len() > MAX_BODY_BYTES {
    problems.push(format!(
      "{relative_path}: {} bytes exceeds the limit of {MAX_BODY_BYTES} bytes",
      bytes.len()
    ));
    return Ok(None);
  }
  match String::from_utf8(bytes) {
    Ok(body) => Ok(Some(TreeFile {
      relative_path: relative_path.to_string(),
      body,
    })),
    Err(_) => {
      problems.push(format!("{relative_path}: not valid UTF-8"));
      Ok(None)
    }
  }
}

fn document_kind_of(file_name: &str) -> Option<DocumentKind> {
  let stem = file_name.strip_suffix(".md")?;
  DocumentKind::SPEC_KINDS
    .into_iter()
    .find(|kind| kind.as_str() == stem)
}

fn scan_spec(
  root: &Path,
  folder: &str,
  number: i64,
  slug: String,
  tree: &mut ProjectTree,
) -> io::Result<TreeSpec> {
  let mut documents = Vec::new();
  for (name, is_dir) in sorted_entries(&root.join("specs").join(folder))? {
    let relative_path = format!("specs/{folder}/{name}");
    match document_kind_of(&name).filter(|_| !is_dir) {
      Some(kind) => {
        if let Some(file) = read_document(root, &relative_path, &mut tree.problems)? {
          documents.push((kind, file));
        }
      }
      None if is_dir => tree.skipped.push(format!("{relative_path}/")),
      None => tree.skipped.push(relative_path),
    }
  }
  documents.sort_by_key(|(kind, _)| DocumentKind::SPEC_KINDS.iter().position(|k| k == kind));
  Ok(TreeSpec {
    folder: folder.to_string(),
    number,
    slug,
    documents,
  })
}

pub fn scan(dir: &Path) -> io::Result<ProjectTree> {
  if !dir.is_dir() {
    return Err(io::Error::new(
      io::ErrorKind::NotFound,
      format!("{} is not a directory", dir.display()),
    ));
  }
  let mut tree = ProjectTree::default();
  if dir.join(CONSTITUTION_PATH).is_file() {
    tree.constitution = read_document(dir, CONSTITUTION_PATH, &mut tree.problems)?;
  }
  let specs_dir = dir.join("specs");
  let entries = if specs_dir.is_dir() {
    sorted_entries(&specs_dir)?
  } else {
    Vec::new()
  };
  let mut spec_folders = 0;
  for (name, is_dir) in entries {
    if !is_dir {
      tree.skipped.push(format!("specs/{name}"));
      continue;
    }
    spec_folders += 1;
    let Some((number, slug)) = parse_spec_folder(&name) else {
      tree.problems.push(format!(
        "specs/{name}: folder name is not a spec branch name such as 001-name"
      ));
      continue;
    };
    if let Some(other) = tree.specs.iter().find(|s| s.number == number) {
      tree.problems.push(format!(
        "specs/{name}: number {number:03} is also used by specs/{}",
        other.folder
      ));
      continue;
    }
    if let Some(other) = tree.specs.iter().find(|s| s.slug == slug) {
      tree.problems.push(format!(
        "specs/{name}: slug {slug:?} is also used by specs/{}",
        other.folder
      ));
      continue;
    }
    let spec = scan_spec(dir, &name, number, slug, &mut tree)?;
    tree.specs.push(spec);
  }
  if spec_folders == 0 && !dir.join(CONSTITUTION_PATH).is_file() {
    tree.problems.push(format!(
      "{}: no {CONSTITUTION_PATH} and no spec folder in specs/",
      dir.display()
    ));
  }
  tree.skipped.sort();
  Ok(tree)
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

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ExportFile {
  pub relative_path: String,
  pub body: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetState {
  New,
  Unchanged,
  Different,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportPlan {
  pub root: std::path::PathBuf,
  pub targets: Vec<(ExportFile, TargetState)>,
}

impl ExportPlan {
  pub fn different(&self) -> Vec<&str> {
    self
      .targets
      .iter()
      .filter(|(_, state)| *state == TargetState::Different)
      .map(|(file, _)| file.relative_path.as_str())
      .collect()
  }
}

fn target_state(path: &Path, body: &str) -> io::Result<TargetState> {
  if !path.exists() {
    return Ok(TargetState::New);
  }
  if std::fs::read(path)? == body.as_bytes() {
    Ok(TargetState::Unchanged)
  } else {
    Ok(TargetState::Different)
  }
}

pub fn plan_export(dir: &Path, files: Vec<ExportFile>) -> io::Result<ExportPlan> {
  let mut targets = Vec::with_capacity(files.len());
  for file in files {
    let state = target_state(&dir.join(&file.relative_path), &file.body)?;
    targets.push((file, state));
  }
  Ok(ExportPlan {
    root: dir.to_path_buf(),
    targets,
  })
}

pub fn write_export(plan: &ExportPlan, force: bool) -> io::Result<()> {
  std::fs::create_dir_all(&plan.root)?;
  for (file, state) in &plan.targets {
    let write = match state {
      TargetState::New => true,
      TargetState::Different => force,
      TargetState::Unchanged => false,
    };
    if !write {
      continue;
    }
    let path = plan.root.join(&file.relative_path);
    if let Some(parent) = path.parent() {
      std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, &file.body)?;
  }
  Ok(())
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
