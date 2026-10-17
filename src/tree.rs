pub fn parse_spec_folder(_name: &str) -> Option<(i64, String)> {
  None
}

pub fn status_word(_body: &str) -> Option<&str> {
  None
}

pub fn version_line(_body: &str) -> Option<&str> {
  None
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
