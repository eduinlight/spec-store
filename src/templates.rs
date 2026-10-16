use crate::model::{DocumentKind, StoreError, StoreResult};

pub const TEMPLATE_KINDS: [DocumentKind; 5] = [
  DocumentKind::Constitution,
  DocumentKind::Spec,
  DocumentKind::Plan,
  DocumentKind::Tasks,
  DocumentKind::Validation,
];

pub fn get(kind: DocumentKind) -> StoreResult<&'static str> {
  match kind {
    DocumentKind::Constitution => Ok(include_str!("../.sdd/templates/constitution.md")),
    DocumentKind::Spec => Ok(include_str!("../.sdd/templates/spec.md")),
    DocumentKind::Plan => Ok(include_str!("../.sdd/templates/plan.md")),
    DocumentKind::Tasks => Ok(include_str!("../.sdd/templates/tasks.md")),
    DocumentKind::Validation => Ok(include_str!("../.sdd/templates/validation.md")),
    DocumentKind::Research => Err(StoreError::Validation(format!(
      "no template for kind {kind}; template kinds: constitution, spec, plan, tasks, validation"
    ))),
  }
}

pub fn get_by_name(name: &str) -> StoreResult<(DocumentKind, &'static str)> {
  let kind: DocumentKind = name.parse()?;
  Ok((kind, get(kind)?))
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::model::{DocumentKind, StoreError};

  #[test]
  fn returns_the_five_template_kinds() {
    for kind in TEMPLATE_KINDS {
      let body = get(kind).unwrap();
      assert!(
        body.starts_with("# "),
        "{kind} template should start with a heading"
      );
      assert!(!body.trim().is_empty());
    }
    assert!(
      get(DocumentKind::Spec)
        .unwrap()
        .contains("Functional requirements")
    );
    assert!(get(DocumentKind::Tasks).unwrap().contains("Done when"));
    assert!(
      get(DocumentKind::Constitution)
        .unwrap()
        .contains("Non-negotiable")
    );
  }

  #[test]
  fn rejects_kinds_without_template_listing_valid_ones() {
    let err = get(DocumentKind::Research).unwrap_err();
    match err {
      StoreError::Validation(msg) => {
        for k in ["constitution", "spec", "plan", "tasks", "validation"] {
          assert!(msg.contains(k), "{msg} should list {k}");
        }
        assert!(msg.contains("research"));
      }
      other => panic!("expected validation error, got {other:?}"),
    }
  }

  #[test]
  fn get_by_name_parses_and_validates() {
    assert!(get_by_name("plan").is_ok());
    assert!(matches!(
      get_by_name("notes"),
      Err(StoreError::Validation(_))
    ));
    assert!(matches!(
      get_by_name("research"),
      Err(StoreError::Validation(_))
    ));
  }
}
