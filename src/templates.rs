#[cfg(test)]
mod tests {
  use super::*;
  use crate::model::{DocumentKind, StoreError};

  #[test]
  fn returns_the_five_template_kinds() {
    for kind in TEMPLATE_KINDS {
      let body = get(kind).unwrap();
      assert!(body.starts_with("# "), "{kind} template should start with a heading");
      assert!(!body.trim().is_empty());
    }
    assert!(get(DocumentKind::Spec).unwrap().contains("Functional requirements"));
    assert!(get(DocumentKind::Tasks).unwrap().contains("Done when"));
    assert!(get(DocumentKind::Constitution).unwrap().contains("Non-negotiable"));
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
    assert!(matches!(get_by_name("notes"), Err(StoreError::Validation(_))));
    assert!(matches!(get_by_name("research"), Err(StoreError::Validation(_))));
  }
}
