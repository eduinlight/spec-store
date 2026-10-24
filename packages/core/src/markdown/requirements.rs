#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequirementKind {
  Functional,
  NonFunctional,
}

impl RequirementKind {
  pub fn as_str(self) -> &'static str {
    match self {
      RequirementKind::Functional => "functional",
      RequirementKind::NonFunctional => "non_functional",
    }
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Requirement {
  pub id: String,
  pub kind: RequirementKind,
  pub text: String,
  pub line: usize,
}

fn strip_list_marker(line: &str) -> Option<&str> {
  let trimmed = line.trim_start();
  trimmed
    .strip_prefix("- ")
    .or_else(|| trimmed.strip_prefix("* "))
    .or_else(|| trimmed.strip_prefix("+ "))
    .map(str::trim_start)
}

fn split_id(rest: &str) -> Option<(String, RequirementKind, &str)> {
  let rest = rest.strip_prefix("**").unwrap_or(rest);
  let (prefix, kind, after) = match rest.strip_prefix("NFR-") {
    Some(after) => ("NFR-", RequirementKind::NonFunctional, after),
    None => (
      "FR-",
      RequirementKind::Functional,
      rest.strip_prefix("FR-")?,
    ),
  };
  let digits = after.bytes().take_while(u8::is_ascii_digit).count();
  if digits == 0 {
    return None;
  }
  let tail = &after[digits..];
  let tail = tail.strip_prefix("**").unwrap_or(tail);
  Some((format!("{prefix}{}", &after[..digits]), kind, tail))
}

fn strip_story_reference(tail: &str) -> &str {
  let tail = tail.trim_start();
  if let Some(inner) = tail.strip_prefix('(')
    && let Some(close) = inner.find(')')
  {
    let group = &inner[..close];
    let is_story_group = !group.is_empty()
      && group
        .split(',')
        .all(|part| part.trim().starts_with("US-") || part.trim().starts_with("FR-"));
    if is_story_group {
      return &inner[close + 1..];
    }
  }
  tail
}

fn clean_text(tail: &str) -> &str {
  let tail = tail.trim_start();
  let tail = tail.strip_prefix("**").unwrap_or(tail);
  tail
    .trim_start_matches(|c: char| c == ':' || c == '—' || c == '–' || c == '-' || c.is_whitespace())
    .trim_end()
}

fn is_continuation(raw: &str) -> bool {
  let line = raw.strip_suffix('\r').unwrap_or(raw);
  let indented = line.starts_with("  ") || line.starts_with('\t');
  indented && !line.trim().is_empty() && strip_list_marker(line).is_none()
}

/// Requirement lines (`- **FR-n** (US-x): text` and its variants) of a spec body, first occurrence per id.
pub fn parse_requirements(body: &str) -> Vec<Requirement> {
  let lines: Vec<&str> = body.split('\n').collect();
  let mut requirements: Vec<Requirement> = Vec::new();
  let mut in_fence = false;
  let mut index = 0;
  while index < lines.len() {
    let line = lines[index].strip_suffix('\r').unwrap_or(lines[index]);
    index += 1;
    if line.trim_start().starts_with("```") {
      in_fence = !in_fence;
      continue;
    }
    if in_fence {
      continue;
    }
    let Some(rest) = strip_list_marker(line) else {
      continue;
    };
    let Some((id, kind, tail)) = split_id(rest) else {
      continue;
    };
    let mut text = clean_text(strip_story_reference(tail)).to_string();
    while index < lines.len() && is_continuation(lines[index]) {
      let continuation = lines[index].trim();
      if !text.is_empty() {
        text.push(' ');
      }
      text.push_str(continuation);
      index += 1;
    }
    if requirements.iter().any(|r| r.id == id) {
      continue;
    }
    requirements.push(Requirement {
      id,
      kind,
      text,
      line: index,
    });
  }
  requirements
}

#[cfg(test)]
mod tests {
  use super::*;

  const BODY: &str = "# Spec 013 — Views

## 5. Functional requirements (EARS)

- **FR-1** (US-1): THE SYSTEM SHALL list specs.
- **FR-2** (US-1, US-2): WHEN a member opens a spec,
  THE SYSTEM SHALL show tabs.
- **FR-9**: THE SYSTEM SHALL show archived specs.
- FR-10: plain format.
* **FR-11** — dash separated

## 6. Non-functional requirements

- **NFR-1**: Resolving adds no call.

Prose mentioning FR-1 and (FR-2) inline is not a requirement.
- **FR-1** (US-3): duplicate id is ignored

```
- **FR-77**: inside a fence
```
";

  #[test]
  fn parses_ids_kinds_and_texts() {
    let found = parse_requirements(BODY);
    let ids: Vec<&str> = found.iter().map(|r| r.id.as_str()).collect();
    assert_eq!(ids, ["FR-1", "FR-2", "FR-9", "FR-10", "FR-11", "NFR-1"]);
    assert_eq!(found[0].text, "THE SYSTEM SHALL list specs.");
    assert_eq!(found[0].kind, RequirementKind::Functional);
    assert_eq!(
      found[1].text,
      "WHEN a member opens a spec, THE SYSTEM SHALL show tabs."
    );
    assert_eq!(found[2].text, "THE SYSTEM SHALL show archived specs.");
    assert_eq!(found[3].text, "plain format.");
    assert_eq!(found[4].text, "dash separated");
    assert_eq!(found[5].kind, RequirementKind::NonFunctional);
    assert_eq!(found[5].text, "Resolving adds no call.");
    assert_eq!(found[0].line, 5);
  }

  #[test]
  fn handles_crlf_and_empty_bodies() {
    assert!(parse_requirements("").is_empty());
    assert!(parse_requirements("# Only a title\n").is_empty());
    let found = parse_requirements(&BODY.replace('\n', "\r\n"));
    assert_eq!(found.len(), 6);
    assert_eq!(
      found[1].text,
      "WHEN a member opens a spec, THE SYSTEM SHALL show tabs."
    );
  }

  #[test]
  fn keeps_the_text_when_the_parenthesis_is_not_a_story_group() {
    let found = parse_requirements("- **FR-3**: (optional) text\n");
    assert_eq!(found[0].text, "(optional) text");
  }

  #[test]
  fn kind_names_are_stable() {
    assert_eq!(RequirementKind::Functional.as_str(), "functional");
    assert_eq!(RequirementKind::NonFunctional.as_str(), "non_functional");
  }
}
