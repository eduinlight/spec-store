#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkerKind {
  Clarification,
  Research,
}

impl MarkerKind {
  pub fn as_str(self) -> &'static str {
    match self {
      MarkerKind::Clarification => "clarification",
      MarkerKind::Research => "research",
    }
  }

  fn keyword(self) -> &'static str {
    match self {
      MarkerKind::Clarification => "NEEDS CLARIFICATION",
      MarkerKind::Research => "NEEDS RESEARCH",
    }
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Marker {
  pub kind: MarkerKind,
  pub heading: String,
  pub text: String,
  pub line: usize,
}

fn heading_of(line: &str) -> Option<&str> {
  let hashes = line.bytes().take_while(|b| *b == b'#').count();
  if hashes == 0 {
    return None;
  }
  let rest = &line[hashes..];
  (rest.is_empty() || rest.starts_with(' ')).then(|| rest.trim())
}

fn classify(inner: &str) -> Option<(MarkerKind, String)> {
  for kind in [MarkerKind::Clarification, MarkerKind::Research] {
    if let Some(rest) = inner.strip_prefix(kind.keyword()) {
      let text = rest.trim_start().strip_prefix(':').unwrap_or(rest).trim();
      return Some((kind, text.to_string()));
    }
  }
  None
}

pub fn scan_markers(body: &str) -> Vec<Marker> {
  let mut markers = Vec::new();
  let mut heading = String::new();
  for (index, raw) in body.split('\n').enumerate() {
    let line = raw.strip_suffix('\r').unwrap_or(raw);
    if let Some(h) = heading_of(line) {
      heading = h.to_string();
      continue;
    }
    let mut rest = line;
    while let Some(open) = rest.find("[NEEDS ") {
      let after_open = &rest[open + 1..];
      let Some(close) = after_open.find(']') else {
        break;
      };
      if let Some((kind, text)) = classify(&after_open[..close]) {
        markers.push(Marker {
          kind,
          heading: heading.clone(),
          text,
          line: index + 1,
        });
      }
      rest = &after_open[close + 1..];
    }
  }
  markers
}

#[cfg(test)]
mod tests {
  use super::*;

  const BODY: &str = "# Spec 001 — First

## 5. Functional requirements
- **FR-1** (US-1): WHEN ... [NEEDS CLARIFICATION: which actor?]

## 12. Open questions
- [NEEDS CLARIFICATION: Who writes the constitution?]

### Research notes
Pending: [NEEDS RESEARCH: rmcp version] and [NEEDS RESEARCH: postgres locks]
";

  #[test]
  fn finds_both_kinds_with_nearest_heading() {
    let markers = scan_markers(BODY);
    assert_eq!(markers.len(), 4);

    assert_eq!(markers[0].kind, MarkerKind::Clarification);
    assert_eq!(markers[0].heading, "5. Functional requirements");
    assert_eq!(markers[0].text, "which actor?");
    assert_eq!(markers[0].line, 4);

    assert_eq!(markers[1].kind, MarkerKind::Clarification);
    assert_eq!(markers[1].heading, "12. Open questions");
    assert_eq!(markers[1].text, "Who writes the constitution?");
    assert_eq!(markers[1].line, 7);

    assert_eq!(markers[2].kind, MarkerKind::Research);
    assert_eq!(markers[2].heading, "Research notes");
    assert_eq!(markers[2].text, "rmcp version");
    assert_eq!(markers[3].text, "postgres locks");
    assert_eq!(markers[3].line, 10);
  }

  #[test]
  fn kind_names_are_stable() {
    assert_eq!(MarkerKind::Clarification.as_str(), "clarification");
    assert_eq!(MarkerKind::Research.as_str(), "research");
  }

  #[test]
  fn ignores_text_without_brackets_and_handles_missing_text() {
    let markers =
      scan_markers("NEEDS CLARIFICATION here\n[NEEDS CLARIFICATION]\n[NEEDS RESEARCH:   ]\n");
    assert_eq!(markers.len(), 2);
    assert_eq!(markers[0].text, "");
    assert_eq!(markers[0].line, 2);
    assert_eq!(markers[1].kind, MarkerKind::Research);
    assert_eq!(markers[1].text, "");
  }

  #[test]
  fn no_markers_and_no_heading() {
    assert!(scan_markers("# Title\n\nAll clear.\n").is_empty());
    let markers = scan_markers("[NEEDS CLARIFICATION: first line]\n");
    assert_eq!(markers[0].heading, "");
  }

  #[test]
  fn handles_crlf() {
    let markers = scan_markers(&BODY.replace('\n', "\r\n"));
    assert_eq!(markers.len(), 4);
    assert_eq!(markers[1].heading, "12. Open questions");
    assert_eq!(markers[3].text, "postgres locks");
  }
}
