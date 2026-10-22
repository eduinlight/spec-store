use pulldown_cmark::{CowStr, Event, Options, Parser, Tag, html};

pub fn render_markdown(body: &str) -> String {
  let options = Options::ENABLE_TABLES | Options::ENABLE_TASKLISTS | Options::ENABLE_STRIKETHROUGH;
  let events = Parser::new_ext(body, options).map(sanitize);
  let mut out = String::with_capacity(body.len() * 2);
  html::push_html(&mut out, events);
  out
}

fn sanitize(event: Event<'_>) -> Event<'_> {
  match event {
    Event::Html(raw) | Event::InlineHtml(raw) => Event::Text(raw),
    Event::Start(Tag::Link {
      link_type,
      dest_url,
      title,
      id,
    }) => Event::Start(Tag::Link {
      link_type,
      dest_url: safe_url(dest_url),
      title,
      id,
    }),
    Event::Start(Tag::Image {
      link_type,
      dest_url,
      title,
      id,
    }) => Event::Start(Tag::Image {
      link_type,
      dest_url: safe_url(dest_url),
      title,
      id,
    }),
    other => other,
  }
}

fn safe_url(url: CowStr<'_>) -> CowStr<'_> {
  if is_safe(&url) {
    url
  } else {
    CowStr::Borrowed("")
  }
}

fn is_safe(url: &str) -> bool {
  let lower = url.trim().to_ascii_lowercase();
  lower.starts_with("http://")
    || lower.starts_with("https://")
    || lower.starts_with("mailto:")
    || lower.starts_with('#')
    || lower.starts_with('/')
    || !lower.contains(':')
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn renders_headings_lists_and_tables() {
    let html = render_markdown("# Title\n\n- one\n- two\n\n| a | b |\n|---|---|\n| 1 | 2 |\n");
    assert!(html.contains("<h1>Title</h1>"));
    assert!(html.contains("<li>one</li>"));
    assert!(html.contains("<table>"));
    assert!(html.contains("<td>2</td>"));
  }

  #[test]
  fn renders_task_boxes_with_their_state() {
    let html = render_markdown("- [x] T01 done\n- [ ] T02 todo\n");
    assert_eq!(html.matches("type=\"checkbox\"").count(), 2);
    assert_eq!(html.matches("checked").count(), 1);
    assert!(html.contains("disabled"));
  }

  #[test]
  fn raw_html_is_shown_as_text() {
    let html = render_markdown("<script>alert(1)</script>\n\nword <b>bold</b> end\n");
    assert!(!html.contains("<script>"));
    assert!(html.contains("&lt;script&gt;alert(1)&lt;/script&gt;"));
    assert!(!html.contains("<b>"));
    assert!(html.contains("&lt;b&gt;bold&lt;/b&gt;"));
  }

  #[test]
  fn unsafe_link_schemes_are_dropped_and_safe_ones_kept() {
    let html = render_markdown(
      "[bad](javascript:alert(1)) [good](https://example.com/x) [rel](../other.md) [mail](mailto:a@b.c) ![img](data:image/png;base64,AAAA)\n",
    );
    assert!(!html.contains("javascript:"));
    assert!(!html.contains("data:"));
    assert!(html.contains("href=\"https://example.com/x\""));
    assert!(html.contains("href=\"../other.md\""));
    assert!(html.contains("href=\"mailto:a@b.c\""));
  }

  #[test]
  fn handles_crlf_and_code_blocks() {
    let html = render_markdown("# T\r\n\r\npara\r\n\r\n```sh\r\nls -la\r\n```\r\n");
    assert!(html.contains("<h1>T</h1>"));
    assert!(html.contains("<p>para</p>"));
    assert!(html.contains("<code class=\"language-sh\">ls -la"));
  }

  #[test]
  fn empty_body_renders_nothing() {
    assert_eq!(render_markdown(""), "");
  }
}
