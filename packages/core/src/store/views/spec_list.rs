use std::collections::HashMap;

use diesel::pg::PgConnection;
use diesel::prelude::*;

use crate::markdown::markers::scan_markers;
use crate::markdown::requirements::{RequirementKind, parse_requirements};
use crate::markdown::tasks::parse_tasks;
use crate::model::{Caller, DocumentKind, SpecStatus, StoreResult};
use crate::schema::{documents, iso_utc, revisions, specs, users};
use crate::store::Store;
use crate::store::projects::access_in;
use crate::store::views::current_bodies;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpecCard {
  pub number: i64,
  pub slug: String,
  pub title: String,
  pub summary: String,
  pub status: SpecStatus,
  pub branch: String,
  pub created_at: String,
  pub updated_at: String,
  pub contributors: Vec<String>,
  pub documents: Vec<DocumentKind>,
  pub requirement_count: usize,
  pub tasks_done: usize,
  pub tasks_total: usize,
  pub open_markers: usize,
}

fn heading_text(line: &str) -> Option<(usize, &str)> {
  let hashes = line.bytes().take_while(|b| *b == b'#').count();
  if hashes == 0 || hashes > 6 {
    return None;
  }
  let rest = &line[hashes..];
  rest.starts_with(' ').then(|| (hashes, rest.trim()))
}

fn strip_spec_prefix(title: &str) -> &str {
  let Some(rest) = title.strip_prefix("Spec ") else {
    return title;
  };
  let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
  if digits == 0 {
    return title;
  }
  let after = rest[digits..].trim_start();
  let stripped = after
    .strip_prefix('—')
    .or_else(|| after.strip_prefix('–'))
    .or_else(|| after.strip_prefix('-'))
    .or_else(|| after.strip_prefix(':'));
  match stripped {
    Some(title) if !title.trim().is_empty() => title.trim(),
    _ => title,
  }
}

/// Title of a spec document: its first `#` heading without the `Spec NNN —` prefix.
pub fn spec_title(body: &str) -> Option<String> {
  let mut in_fence = false;
  for raw in body.split('\n') {
    let line = raw.strip_suffix('\r').unwrap_or(raw);
    if line.trim_start().starts_with("```") {
      in_fence = !in_fence;
      continue;
    }
    if in_fence {
      continue;
    }
    if let Some((1, text)) = heading_text(line) {
      let title = strip_spec_prefix(text);
      return (!title.is_empty()).then(|| title.to_string());
    }
  }
  None
}

fn paragraphs(lines: &[&str]) -> Vec<String> {
  let mut found = Vec::new();
  let mut current: Vec<&str> = Vec::new();
  let mut in_fence = false;
  for line in lines {
    if line.trim_start().starts_with("```") {
      in_fence = !in_fence;
      current.clear();
      continue;
    }
    if in_fence {
      continue;
    }
    if line.trim().is_empty() {
      if !current.is_empty() {
        found.push(current.join(" "));
        current.clear();
      }
      continue;
    }
    current.push(line.trim());
  }
  if !current.is_empty() {
    found.push(current.join(" "));
  }
  found
}

fn is_prose(paragraph: &str) -> bool {
  let first = paragraph.chars().next().unwrap_or(' ');
  !matches!(first, '#' | '-' | '*' | '+' | '|' | '>' | '[' | '<' | '!')
    && !paragraph.starts_with("Status:")
    && !paragraph.starts_with("Format:")
    && !is_numbered_item(paragraph)
}

fn is_numbered_item(paragraph: &str) -> bool {
  let digits = paragraph.bytes().take_while(u8::is_ascii_digit).count();
  digits > 0 && paragraph[digits..].starts_with(". ")
}

fn section_lines<'a>(lines: &[&'a str], section: impl Fn(&str) -> bool) -> Vec<&'a str> {
  let mut collected = Vec::new();
  let mut inside = false;
  for line in lines {
    if let Some((level, text)) = heading_text(line) {
      if inside && level <= 2 {
        break;
      }
      if level == 2 {
        inside = section(text);
        continue;
      }
    }
    if inside {
      collected.push(*line);
    }
  }
  collected
}

/// One-line summary: first paragraph of the Context section, else the first prose paragraph.
pub fn spec_summary(body: &str) -> String {
  let lines: Vec<&str> = body
    .split('\n')
    .map(|l| l.strip_suffix('\r').unwrap_or(l))
    .collect();
  let context = section_lines(&lines, |text| text.to_lowercase().contains("context"));
  let from_context = paragraphs(&context).into_iter().find(|p| is_prose(p));
  let found = from_context.or_else(|| {
    let without_headings: Vec<&str> = lines
      .iter()
      .copied()
      .filter(|l| heading_text(l).is_none())
      .collect();
    paragraphs(&without_headings)
      .into_iter()
      .find(|p| is_prose(p))
  });
  found.unwrap_or_default()
}

struct Activity {
  updated_at: String,
  contributors: Vec<(i64, String)>,
}

fn activities(conn: &mut PgConnection, spec_ids: &[i64]) -> StoreResult<HashMap<i64, Activity>> {
  let rows: Vec<(Option<i64>, i64, String, String)> = revisions::table
    .inner_join(documents::table)
    .inner_join(users::table)
    .filter(documents::spec_id.eq_any(spec_ids))
    .select((
      documents::spec_id,
      revisions::id,
      users::handle,
      iso_utc(revisions::created_at),
    ))
    .order(revisions::id)
    .load(conn)?;
  let mut by_spec: HashMap<i64, Activity> = HashMap::new();
  for (spec_id, revision_id, handle, created_at) in rows {
    let Some(spec_id) = spec_id else { continue };
    let activity = by_spec.entry(spec_id).or_insert_with(|| Activity {
      updated_at: created_at.clone(),
      contributors: Vec::new(),
    });
    if created_at > activity.updated_at {
      activity.updated_at = created_at;
    }
    if !activity.contributors.iter().any(|(_, h)| *h == handle) {
      activity.contributors.push((revision_id, handle));
    }
  }
  Ok(by_spec)
}

type SpecRowWithDate = (i64, i64, String, String, String, String);

pub(crate) fn cards_in(conn: &mut PgConnection, project_id: i64) -> StoreResult<Vec<SpecCard>> {
  let rows: Vec<SpecRowWithDate> = specs::table
    .filter(specs::project_id.eq(project_id))
    .select((
      specs::id,
      specs::number,
      specs::slug,
      specs::status,
      specs::branch,
      iso_utc(specs::created_at),
    ))
    .order(specs::number.desc())
    .load(conn)?;
  let spec_ids: Vec<i64> = rows.iter().map(|row| row.0).collect();
  let mut bodies = current_bodies(conn, &spec_ids)?;
  let mut activities = activities(conn, &spec_ids)?;
  rows
    .into_iter()
    .map(|(id, number, slug, status, branch, created_at)| {
      let documents_of = bodies.remove(&id).unwrap_or_default();
      let spec_body = documents_of.get(&DocumentKind::Spec);
      let tasks = documents_of
        .get(&DocumentKind::Tasks)
        .map(|body| parse_tasks(body).tasks)
        .unwrap_or_default();
      let activity = activities.remove(&id);
      let (updated_at, contributors) = match activity {
        Some(activity) => (
          activity.updated_at,
          activity.contributors.into_iter().map(|(_, h)| h).collect(),
        ),
        None => (created_at.clone(), Vec::new()),
      };
      Ok(SpecCard {
        title: spec_body
          .and_then(|body| spec_title(body))
          .unwrap_or_else(|| slug.clone()),
        summary: spec_body.map(|body| spec_summary(body)).unwrap_or_default(),
        requirement_count: spec_body
          .map(|body| {
            parse_requirements(body)
              .iter()
              .filter(|r| r.kind == RequirementKind::Functional)
              .count()
          })
          .unwrap_or(0),
        open_markers: documents_of
          .values()
          .map(|body| scan_markers(body).len())
          .sum(),
        documents: DocumentKind::SPEC_KINDS
          .into_iter()
          .filter(|kind| documents_of.contains_key(kind))
          .collect(),
        tasks_done: tasks.iter().filter(|t| t.done).count(),
        tasks_total: tasks.len(),
        status: status.parse()?,
        number,
        slug,
        branch,
        created_at,
        updated_at,
        contributors,
      })
    })
    .collect()
}

impl Store {
  /// Cards of every spec of a project, newest first.
  pub fn spec_cards(&self, caller: &Caller, project: &str) -> StoreResult<Vec<SpecCard>> {
    self.with_conn(|conn| {
      let access = access_in(conn, caller, project)?;
      cards_in(conn, access.project_id)
    })
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn title_drops_the_spec_prefix() {
    assert_eq!(spec_title("# Spec 013 — Views\n").as_deref(), Some("Views"));
    assert_eq!(spec_title("# Spec 7 - Dash\n").as_deref(), Some("Dash"));
    assert_eq!(spec_title("# Spec 7: Colon\n").as_deref(), Some("Colon"));
    assert_eq!(
      spec_title("# Plain title\n").as_deref(),
      Some("Plain title")
    );
    assert_eq!(spec_title("# Spec 013\n").as_deref(), Some("Spec 013"));
    assert_eq!(
      spec_title("## Not first level\n\n# Later\n").as_deref(),
      Some("Later")
    );
    assert_eq!(spec_title("no heading\n"), None);
    assert_eq!(spec_title("```\n# in fence\n```\n"), None);
  }

  #[test]
  fn summary_prefers_the_context_paragraph() {
    let body = "# Spec 1 — T\n\nStatus: approved · Created: x\n\n## 1. Context and goal\n\nThe app shows\nspecs as cards.\n\nSecond paragraph.\n\n## 2. Users\n\nOther.\n";
    assert_eq!(spec_summary(body), "The app shows specs as cards.");
  }

  #[test]
  fn summary_falls_back_to_the_first_prose_paragraph() {
    let body = "# T\n\nStatus: draft\n\n- list\n\nFirst real paragraph.\n\nSecond.\n";
    assert_eq!(spec_summary(body), "First real paragraph.");
    assert_eq!(spec_summary("# T\n"), "");
  }

  #[test]
  fn summary_skips_non_prose_in_the_context_section() {
    let body = "# T\n\n## Context\n\n| a | b |\n|---|---|\n\nReal text.\n";
    assert_eq!(spec_summary(body), "Real text.");
  }
}
