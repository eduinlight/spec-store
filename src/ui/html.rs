use crate::model::Caller;

pub fn escape(input: &str) -> String {
  let mut out = String::with_capacity(input.len());
  for c in input.chars() {
    match c {
      '&' => out.push_str("&amp;"),
      '<' => out.push_str("&lt;"),
      '>' => out.push_str("&gt;"),
      '"' => out.push_str("&quot;"),
      '\'' => out.push_str("&#39;"),
      _ => out.push(c),
    }
  }
  out
}

pub fn link(href: &str, text: &str) -> String {
  format!("<a href=\"{}\">{}</a>", escape(href), escape(text))
}

pub fn badge(value: &str) -> String {
  let safe = escape(value);
  format!("<span class=\"badge badge-{safe}\">{safe}</span>")
}

pub fn layout(title: &str, viewer: Option<&Caller>, body: &str) -> String {
  let mut nav = String::new();
  if let Some(user) = viewer {
    nav.push_str(&link("/ui/", "Projects"));
    if user.is_admin {
      nav.push_str(&link("/ui/audit", "Audit"));
    }
    nav.push_str("<span class=\"viewer\">");
    nav.push_str(&link("/ui/profile", &user.handle));
    if user.is_admin {
      nav.push_str(" <span class=\"badge badge-admin\">admin</span>");
    }
    nav.push_str("</span>");
    nav.push_str(
      "<form method=\"post\" action=\"/ui/logout\" class=\"logout\"><button type=\"submit\">Log out</button></form>",
    );
  }
  format!(
    "<!doctype html>\n<html lang=\"en\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width, initial-scale=1\"><title>{} · spec-store</title><link rel=\"stylesheet\" href=\"/ui/style.css\"></head><body><header><a class=\"brand\" href=\"/ui/\">spec-store</a><nav>{nav}</nav></header><main>{body}</main></body></html>\n",
    escape(title)
  )
}

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
