use std::collections::HashMap;

use crate::markdown::markers::{Marker, MarkerKind, scan_markers};
use crate::model::{Caller, DocumentKind, StoreResult};
use crate::store::Store;
use crate::store::tasks::SpecMarker;
use crate::store::views::{RevisionRecord, revision_records, spec_document_ids, spec_scope};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedMarker {
  pub document_kind: DocumentKind,
  pub kind: MarkerKind,
  pub heading: String,
  pub text: String,
  pub revision_id: i64,
  pub resolved_by: String,
  pub client: Option<String>,
  pub at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpecMarkers {
  pub open: Vec<SpecMarker>,
  pub resolved: Vec<ResolvedMarker>,
}

type MarkerKey = (&'static str, String);

fn counts(markers: &[Marker]) -> HashMap<MarkerKey, usize> {
  let mut counts = HashMap::new();
  for marker in markers {
    *counts
      .entry((marker.kind.as_str(), marker.text.clone()))
      .or_insert(0) += 1;
  }
  counts
}

/// Markers present in a revision and absent from the next one, attributed to the next revision, oldest first.
pub fn resolved_markers(
  document_kind: DocumentKind,
  revisions: &[RevisionRecord],
) -> Vec<ResolvedMarker> {
  let scanned: Vec<Vec<Marker>> = revisions.iter().map(|r| scan_markers(&r.body)).collect();
  let mut resolved = Vec::new();
  for (index, pair) in revisions.windows(2).enumerate() {
    let before = &scanned[index];
    if before.is_empty() {
      continue;
    }
    let mut remaining = counts(&scanned[index + 1]);
    let next = &pair[1];
    for marker in before {
      let key = (marker.kind.as_str(), marker.text.clone());
      match remaining.get_mut(&key) {
        Some(left) if *left > 0 => *left -= 1,
        _ => resolved.push(ResolvedMarker {
          document_kind,
          kind: marker.kind,
          heading: marker.heading.clone(),
          text: marker.text.clone(),
          revision_id: next.id,
          resolved_by: next.author.clone(),
          client: next.client.clone(),
          at: next.at.clone(),
        }),
      }
    }
  }
  resolved
}

impl Store {
  /// Open markers of the current revisions and resolved markers of the history; resolved are newest first.
  pub fn spec_markers(
    &self,
    caller: &Caller,
    project: &str,
    number: i64,
  ) -> StoreResult<SpecMarkers> {
    self.with_conn(|conn| {
      let scope = spec_scope(conn, caller, project, number)?;
      let mut open = Vec::new();
      let mut resolved = Vec::new();
      for (kind, document_id) in spec_document_ids(conn, scope.spec.id)? {
        let history = revision_records(conn, document_id)?;
        if let Some(latest) = history.last() {
          open.extend(scan_markers(&latest.body).into_iter().map(|m| SpecMarker {
            document_kind: kind,
            kind: m.kind,
            heading: m.heading,
            text: m.text,
            line: m.line,
          }));
        }
        resolved.extend(resolved_markers(kind, &history));
      }
      resolved.sort_by_key(|marker| std::cmp::Reverse(marker.revision_id));
      Ok(SpecMarkers { open, resolved })
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

  #[test]
  fn a_marker_absent_from_the_next_revision_is_resolved_by_its_author() {
    let history = [
      revision(
        1,
        "ana",
        None,
        "## Q\n[NEEDS CLARIFICATION: who?] and [NEEDS RESEARCH: lib]\n",
      ),
      revision(
        2,
        "bob",
        Some("Cursor"),
        "## Q\n[NEEDS RESEARCH: lib]\nWho: ana\n",
      ),
      revision(3, "cy", None, "## Q\nAll answered\n"),
    ];
    let found = resolved_markers(DocumentKind::Spec, &history);
    assert_eq!(found.len(), 2);
    assert_eq!(found[0].text, "who?");
    assert_eq!(found[0].kind, MarkerKind::Clarification);
    assert_eq!(found[0].heading, "Q");
    assert_eq!(found[0].resolved_by, "bob");
    assert_eq!(found[0].client.as_deref(), Some("Cursor"));
    assert_eq!(found[0].revision_id, 2);
    assert_eq!(found[1].text, "lib");
    assert_eq!(found[1].resolved_by, "cy");
    assert_eq!(found[1].client, None);
  }

  #[test]
  fn moving_a_marker_between_lines_does_not_resolve_it() {
    let history = [
      revision(1, "ana", None, "[NEEDS CLARIFICATION: who?]\n"),
      revision(
        2,
        "bob",
        None,
        "intro\n\n## Later\n[NEEDS CLARIFICATION: who?]\n",
      ),
    ];
    assert!(resolved_markers(DocumentKind::Plan, &history).is_empty());
  }

  #[test]
  fn duplicates_resolve_one_at_a_time() {
    let history = [
      revision(
        1,
        "ana",
        None,
        "[NEEDS CLARIFICATION: x]\n[NEEDS CLARIFICATION: x]\n",
      ),
      revision(2, "bob", None, "[NEEDS CLARIFICATION: x]\n"),
    ];
    assert_eq!(resolved_markers(DocumentKind::Spec, &history).len(), 1);
  }

  #[test]
  fn single_revisions_and_empty_histories_resolve_nothing() {
    assert!(resolved_markers(DocumentKind::Spec, &[]).is_empty());
    let one = [revision(1, "ana", None, "[NEEDS CLARIFICATION: x]\n")];
    assert!(resolved_markers(DocumentKind::Spec, &one).is_empty());
  }
}
