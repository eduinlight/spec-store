#[cfg(test)]
mod tests {
  use super::*;

  const OLD: &str = "a\nb\nc\nd\ne\nf\ng\nh\ni\nj\n";

  #[test]
  fn identical_bodies_have_no_hunks() {
    assert!(diff_lines(OLD, OLD).is_empty());
    assert!(diff_lines("", "").is_empty());
  }

  #[test]
  fn insertions_and_deletions_carry_line_numbers() {
    let new = "a\nb\nc\nd\nX\nf\ng\nh\ni\nj\n";
    let hunks = diff_lines(OLD, new);
    assert_eq!(hunks.len(), 1);
    let lines = &hunks[0].lines;
    let deleted: Vec<_> = lines.iter().filter(|l| l.tag == DiffTag::Delete).collect();
    let inserted: Vec<_> = lines.iter().filter(|l| l.tag == DiffTag::Insert).collect();
    assert_eq!(deleted.len(), 1);
    assert_eq!(deleted[0].text, "e");
    assert_eq!(deleted[0].old_line, Some(5));
    assert_eq!(deleted[0].new_line, None);
    assert_eq!(inserted.len(), 1);
    assert_eq!(inserted[0].text, "X");
    assert_eq!(inserted[0].new_line, Some(5));
    assert_eq!(inserted[0].old_line, None);
    let context = lines.iter().filter(|l| l.tag == DiffTag::Equal).count();
    assert_eq!(context, 6, "three lines of context on each side");
  }

  #[test]
  fn distant_changes_make_separate_hunks() {
    let new = "A\nb\nc\nd\ne\nf\ng\nh\ni\nJ\n";
    let hunks = diff_lines(OLD, new);
    assert_eq!(hunks.len(), 2);
    assert_eq!(hunks[0].lines[0].text, "A");
    assert!(
      hunks[0]
        .lines
        .iter()
        .any(|l| l.tag == DiffTag::Delete && l.text == "a")
    );
    assert!(
      hunks[1]
        .lines
        .iter()
        .any(|l| l.tag == DiffTag::Insert && l.text == "J")
    );
  }

  #[test]
  fn appended_lines_without_trailing_newline_are_handled() {
    let hunks = diff_lines("a\nb", "a\nb\nc");
    let inserted: Vec<_> = hunks
      .iter()
      .flat_map(|h| h.lines.iter())
      .filter(|l| l.tag == DiffTag::Insert)
      .collect();
    assert!(inserted.iter().any(|l| l.text == "c"));
  }

  #[test]
  fn render_marks_rows_and_escapes_text() {
    let hunks = diff_lines("x\n<b>\n", "x\n&y\n");
    let html = render_diff(&hunks);
    assert!(html.contains("class=\"del\""));
    assert!(html.contains("class=\"ins\""));
    assert!(html.contains("&lt;b&gt;"));
    assert!(html.contains("&amp;y"));
    assert!(!html.contains("<b>"));
    assert_eq!(render_diff(&[]), "");
  }
}
