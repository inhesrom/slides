use pulldown_cmark::{html, Options, Parser};

/// Marker inserted for `+` list items so we can find them in the HTML output.
const FRAGMENT_MARKER: &str = "\u{FEFF}";

/// Render markdown to HTML without any slide-specific transformations.
///
/// Enables the same pulldown-cmark extensions as the slide renderer (tables,
/// footnotes, strikethrough, task lists) but does NOT convert `+` list items
/// into fragment reveals or strip `{.class}` trailing annotations. Use this
/// for rendering prose documents like SYNTAX.md where those transformations
/// would mis-render example snippets.
pub fn render_plain(markdown: &str) -> String {
    let options = Options::ENABLE_TABLES
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS;

    let parser = Parser::new_ext(markdown, options);
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);
    html_output
}

/// Render markdown to HTML, applying semantic style annotations.
pub fn render(markdown: &str) -> String {
    let (cleaned, annotations) = extract_annotations(markdown);

    // Note: we don't enable ENABLE_HEADING_ATTRIBUTES because we handle
    // {.class} annotations ourselves in the pre-processing pass.
    let options = Options::ENABLE_TABLES
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS;

    let parser = Parser::new_ext(&cleaned, options);
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);

    let html_output = apply_annotations(&html_output, &annotations);
    apply_fragment_markers(&html_output)
}

/// Replace fragment markers with actual class attributes on `<li>` tags.
/// Handles both bare `<li>` and `<li class="existing">` cases.
fn apply_fragment_markers(html: &str) -> String {
    let marker = FRAGMENT_MARKER;
    let mut result = String::with_capacity(html.len());
    let mut remaining = html;

    while let Some(pos) = remaining.find(marker) {
        // Look backwards for the `<li` that owns this marker
        let before = &remaining[..pos];
        if let Some(li_start) = before.rfind("<li") {
            let tag_region = &remaining[li_start..pos];
            if let Some(class_pos) = tag_region.find("class=\"") {
                // Already has a class — append fragment to it
                let abs_class = li_start + class_pos + 7; // after `class="`
                result.push_str(&remaining[..abs_class]);
                result.push_str("fragment ");
                result.push_str(&remaining[abs_class..pos]);
            } else {
                // No class yet — add one to the <li> tag
                let tag_close = before[li_start..].find('>').unwrap() + li_start;
                result.push_str(&remaining[..tag_close]);
                result.push_str(" class=\"fragment\"");
                result.push_str(&remaining[tag_close..pos]);
            }
        } else {
            result.push_str(before);
        }
        // Skip the marker character
        remaining = &remaining[pos + marker.len()..];
    }
    result.push_str(remaining);
    result
}

/// An annotation extracted from a markdown line.
struct Annotation {
    classes: Vec<String>,
}

/// Pre-process markdown to extract `{.class1 .class2}` from line endings
/// and convert `+ item` list markers into `- ` items with a fragment marker.
/// Returns cleaned markdown and a list of annotations with their line indices.
fn extract_annotations(markdown: &str) -> (String, Vec<Annotation>) {
    let mut cleaned = String::new();
    let mut annotations = Vec::new();

    for line in markdown.lines() {
        let trimmed = line.trim_end();

        // Detect `+ ` list markers — these become fragment items.
        // Insert a marker character that we find in the HTML output later.
        let working_line = if let Some(rest) = strip_plus_marker(trimmed) {
            let indent = &trimmed[..trimmed.len() - trimmed.trim_start().len()];
            format!("{}- {}{}", indent, FRAGMENT_MARKER, rest)
        } else {
            trimmed.to_string()
        };

        if let Some((before, classes)) = parse_trailing_annotation(&working_line) {
            if !classes.is_empty() {
                annotations.push(Annotation { classes });
                cleaned.push_str(before);
                cleaned.push('\n');
                continue;
            }
        }

        cleaned.push_str(&working_line);
        cleaned.push('\n');
    }

    (cleaned, annotations)
}

/// If the line is a `+ ` list item, return the content after the marker.
fn strip_plus_marker(line: &str) -> Option<&str> {
    let trimmed = line.trim_start();
    trimmed.strip_prefix("+ ")
}

/// Parse a trailing `{.class1 .class2}` from a line.
/// Returns (line_without_annotation, classes) or None if no annotation found.
fn parse_trailing_annotation(line: &str) -> Option<(&str, Vec<String>)> {
    let close = line.rfind('}')?;
    let open = line[..close].rfind('{')?;
    let inner = &line[open + 1..close];

    // Must start with a dot-class
    if !inner.trim_start().starts_with('.') {
        return None;
    }

    let classes: Vec<String> = inner
        .split_whitespace()
        .filter_map(|s| s.strip_prefix('.').map(|c| c.to_string()))
        .collect();

    if classes.is_empty() {
        return None;
    }

    let before = line[..open].trim_end();
    Some((before, classes))
}

/// Apply extracted annotations to rendered HTML by wrapping annotated blocks
/// in `<div class="...">` wrappers.
fn apply_annotations(html: &str, annotations: &[Annotation]) -> String {
    if annotations.is_empty() {
        return html.to_string();
    }

    let mut result = html.to_string();

    // For each annotation, find the HTML block element that corresponds to
    // the annotated line and add classes to it.
    // Strategy: work on the HTML line by line, matching annotations by content proximity.
    for annotation in annotations {
        let class_attr = annotation.classes.join(" ");
        result = add_class_to_block(&result, &class_attr);
    }

    result
}

/// Find the last block-level opening tag without a class and add the given classes.
fn add_class_to_block(html: &str, classes: &str) -> String {
    // Find block tags that don't have a class attribute yet
    let block_tags = [
        "<p>", "<blockquote>\n<p>", "<li>",
        "<h1>", "<h2>", "<h3>", "<h4>", "<h5>", "<h6>",
    ];

    // Try to find the last unclassed block tag
    for tag in &block_tags {
        if let Some(pos) = html.rfind(tag) {
            let target = if *tag == "<blockquote>\n<p>" {
                "<blockquote>"
            } else {
                *tag
            };

            let actual_pos = html.rfind(target).unwrap_or(pos);

            let mut result = String::with_capacity(html.len() + classes.len() + 10);
            let tag_no_close = target.trim_end_matches('>');
            result.push_str(&html[..actual_pos]);
            result.push_str(tag_no_close);
            result.push_str(&format!(" class=\"{}\">", classes));
            result.push_str(&html[actual_pos + target.len()..]);
            return result;
        }
    }

    html.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_markdown() {
        let result = render("# Hello\n\nWorld");
        assert!(result.contains("<h1>Hello</h1>"));
        assert!(result.contains("<p>World</p>"));
    }

    #[test]
    fn test_code_block() {
        let result = render("```rust\nfn main() {}\n```");
        assert!(result.contains("<code class=\"language-rust\">"));
    }

    #[test]
    fn test_semantic_emphasis_class() {
        let result = render("## Important Point {.emphasis}");
        assert!(result.contains("emphasis"), "Got: {}", result);
        assert!(!result.contains("{.emphasis}"), "Annotation not stripped: {}", result);
    }

    #[test]
    fn test_semantic_paragraph_class() {
        let result = render("Some text {.aside}");
        assert!(result.contains("class=\"aside\""), "Got: {}", result);
        assert!(!result.contains("{.aside}"), "Annotation not stripped: {}", result);
    }

    #[test]
    fn test_semantic_multiple_classes() {
        let result = render("> Quote text {.callout .warning}");
        assert!(result.contains("callout"), "Got: {}", result);
        assert!(result.contains("warning"), "Got: {}", result);
    }

    #[test]
    fn test_no_annotation() {
        let result = render("Just a normal paragraph");
        assert!(result.contains("<p>Just a normal paragraph</p>"));
    }

    #[test]
    fn test_tables() {
        let result = render("| A | B |\n|---|---|\n| 1 | 2 |");
        assert!(result.contains("<table>"));
    }

    #[test]
    fn test_lists() {
        let result = render("- item 1\n- item 2");
        assert!(result.contains("<li>item 1</li>"));
    }

    #[test]
    fn test_fragment_class_legacy() {
        let result = render("- item {.fragment}");
        assert!(result.contains("fragment"), "Got: {}", result);
    }

    #[test]
    fn test_plus_marker_fragment() {
        let result = render("+ revealed item");
        assert!(result.contains("fragment"), "Got: {}", result);
        assert!(result.contains("revealed item"), "Got: {}", result);
    }

    #[test]
    fn test_plus_marker_mixed_list() {
        let result = render("- normal\n+ revealed\n- also normal");
        // Only the middle item should be a fragment
        assert_eq!(result.matches("class=\"fragment\"").count(), 1, "Got: {}", result);
        assert!(result.contains("<li class=\"fragment\">revealed</li>"), "Got: {}", result);
        assert!(result.contains("<li>normal</li>"), "Got: {}", result);
        assert!(result.contains("<li>also normal</li>"), "Got: {}", result);
    }

    #[test]
    fn test_plus_marker_with_annotation() {
        let result = render("+ item {.highlight}");
        assert!(result.contains("fragment"), "Got: {}", result);
        assert!(result.contains("highlight"), "Got: {}", result);
    }

    #[test]
    fn test_strip_plus_marker() {
        assert_eq!(strip_plus_marker("+ hello"), Some("hello"));
        assert_eq!(strip_plus_marker("  + nested"), Some("nested"));
        assert_eq!(strip_plus_marker("- normal"), None);
        assert_eq!(strip_plus_marker("+no space"), None);
    }

    #[test]
    fn test_parse_trailing_annotation() {
        let (before, classes) = parse_trailing_annotation("Some text {.aside}").unwrap();
        assert_eq!(before, "Some text");
        assert_eq!(classes, vec!["aside"]);
    }

    #[test]
    fn test_parse_multiple_trailing_classes() {
        let (before, classes) = parse_trailing_annotation("> text {.callout .warning}").unwrap();
        assert_eq!(before, "> text");
        assert_eq!(classes, vec!["callout", "warning"]);
    }

    #[test]
    fn test_no_trailing_annotation() {
        let result = parse_trailing_annotation("Just normal text");
        assert!(result.is_none());
    }

    #[test]
    fn test_render_plain_basic_headings() {
        let result = render_plain("# Hello\n\nWorld");
        assert!(result.contains("<h1>Hello</h1>"));
        assert!(result.contains("<p>World</p>"));
    }

    #[test]
    fn test_render_plain_does_not_make_fragments() {
        // `+` list items must remain normal list items in plain render,
        // not be converted into fragment-classed <li>s.
        let result = render_plain("+ item one\n+ item two");
        assert!(!result.contains("fragment"), "Got: {}", result);
        assert!(result.contains("<li>item one</li>"), "Got: {}", result);
    }

    #[test]
    fn test_render_plain_does_not_strip_annotations() {
        // `{.class}` trailing annotations are slide-specific; plain render
        // leaves them as literal text.
        let result = render_plain("Some text {.aside}");
        assert!(result.contains("{.aside}"), "Got: {}", result);
        assert!(!result.contains("class=\"aside\""), "Got: {}", result);
    }

    #[test]
    fn test_render_plain_tables_and_code() {
        let table = render_plain("| A | B |\n|---|---|\n| 1 | 2 |");
        assert!(table.contains("<table>"), "Got: {}", table);

        let code = render_plain("```rust\nfn main() {}\n```");
        assert!(code.contains("<code class=\"language-rust\">"), "Got: {}", code);
    }

    #[test]
    fn test_extract_annotations() {
        let input = "# Title\n\nSome text {.aside}\n\nMore text";
        let (cleaned, annotations) = extract_annotations(input);
        assert!(!cleaned.contains("{.aside}"));
        assert_eq!(annotations.len(), 1);
        assert_eq!(annotations[0].classes, vec!["aside"]);
    }
}
