pub const UI_ROOT: &str = "/ui/";

pub fn safe_next(next: Option<&str>) -> String {
  match next {
    Some(path)
      if path.starts_with(UI_ROOT)
        && !path.contains("//")
        && !path.contains("..")
        && !path.contains('\\')
        && !path.chars().any(|c| c.is_control() || c.is_whitespace()) =>
    {
      path.to_string()
    }
    _ => UI_ROOT.to_string(),
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn safe_next_keeps_ui_paths_only() {
    assert_eq!(safe_next(Some("/ui/p/demo")), "/ui/p/demo");
    assert_eq!(safe_next(Some("/ui/")), "/ui/");
    assert_eq!(
      safe_next(Some("/ui/p/demo/specs/1?view=raw")),
      "/ui/p/demo/specs/1?view=raw"
    );
    for bad in [
      "//evil.example",
      "https://evil.example/ui/",
      "/mcp",
      "/ui",
      "",
      "/ui/../mcp",
      "/ui//evil",
      "/ui/x\r\nSet-Cookie: a=b",
      "javascript:alert(1)",
    ] {
      assert_eq!(safe_next(Some(bad)), "/ui/", "{bad:?}");
    }
    assert_eq!(safe_next(None), "/ui/");
  }
}
