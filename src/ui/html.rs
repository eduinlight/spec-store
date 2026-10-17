#[cfg(test)]
mod tests {
  use super::*;
  use crate::model::Caller;

  fn viewer(is_admin: bool) -> Caller {
    Caller {
      user_id: 1,
      handle: "bob".to_string(),
      is_admin,
    }
  }

  #[test]
  fn escape_neutralises_html_special_characters() {
    assert_eq!(
      escape("<a href=\"x\">&'"),
      "&lt;a href=&quot;x&quot;&gt;&amp;&#39;"
    );
    assert_eq!(escape("plain text"), "plain text");
    assert_eq!(escape(""), "");
  }

  #[test]
  fn layout_shows_title_viewer_and_controls() {
    let page = layout("Demo project", Some(&viewer(false)), "<p>body</p>");
    assert!(page.starts_with("<!doctype html>"));
    assert!(page.contains("<title>Demo project · spec-store</title>"));
    assert!(page.contains("/ui/style.css"));
    assert!(page.contains("<p>body</p>"));
    assert!(page.contains("bob"));
    assert!(page.contains("href=\"/ui/profile\""));
    assert!(page.contains("action=\"/ui/logout\""));
    assert!(page.contains("href=\"/ui/\""));
    assert!(!page.contains("admin"));
    assert!(!page.contains("href=\"/ui/audit\""));
  }

  #[test]
  fn layout_marks_admins_and_links_the_global_audit() {
    let page = layout("Projects", Some(&viewer(true)), "");
    assert!(page.contains("admin"));
    assert!(page.contains("href=\"/ui/audit\""));
  }

  #[test]
  fn layout_without_viewer_has_no_user_controls_and_escapes_the_title() {
    let page = layout("<Login>", None, "");
    assert!(page.contains("<title>&lt;Login&gt; · spec-store</title>"));
    assert!(!page.contains("/ui/logout"));
    assert!(!page.contains("/ui/profile"));
  }

  #[test]
  fn helpers_escape_their_input() {
    assert_eq!(
      link("/ui/p/demo", "a<b"),
      "<a href=\"/ui/p/demo\">a&lt;b</a>"
    );
    assert_eq!(link("/ui/p/d\"q", "x"), "<a href=\"/ui/p/d&quot;q\">x</a>");
    assert_eq!(
      badge("draft"),
      "<span class=\"badge badge-draft\">draft</span>"
    );
  }
}
