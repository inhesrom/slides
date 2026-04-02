use crate::parser::frontmatter::AspectRatio;

/// Overflow detection result for a single slide.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct OverflowResult {
    pub slide_index: usize,
    pub estimated_height: f32,
    pub available_height: f32,
    pub overflow_pct: f32,
}

/// Estimate the content height of a rendered HTML slide.
/// Uses heuristics based on element counts and typical sizes.
pub fn estimate_content_height(html: &str, aspect: &AspectRatio) -> f32 {
    let base_font = 28.0; // px, matches core.css
    let line_height = 1.5;
    let line_px = base_font * line_height;

    let mut height: f32 = 0.0;

    // Count headings
    height += count_tag(html, "<h1") as f32 * base_font * 2.4 * line_height;
    height += count_tag(html, "<h2") as f32 * base_font * 1.8 * line_height;
    height += count_tag(html, "<h3") as f32 * base_font * 1.3 * line_height;

    // Count paragraphs — estimate lines by character count
    let para_count = count_tag(html, "<p");
    let avg_chars_per_line = estimate_chars_per_line(aspect);
    let total_text_len = estimate_text_length(html);
    let estimated_text_lines = if para_count > 0 {
        (total_text_len as f32 / avg_chars_per_line).ceil()
    } else {
        0.0
    };
    height += estimated_text_lines * line_px;
    height += para_count as f32 * base_font * 0.8; // paragraph margins

    // Count list items
    let li_count = count_tag(html, "<li");
    height += li_count as f32 * line_px;

    // Count code blocks
    for block in extract_tag_contents(html, "<pre", "</pre>") {
        let lines = block.lines().count();
        height += lines as f32 * base_font * 0.7 * 1.6; // code font size * line height
        height += base_font * 2.4; // padding
    }

    // Count images — assume default height
    let img_count = count_tag(html, "<img");
    height += img_count as f32 * 300.0;

    // Count tables
    let table_rows = count_tag(html, "<tr");
    height += table_rows as f32 * line_px * 0.85;

    // Count blockquotes
    let bq_count = count_tag(html, "<blockquote");
    height += bq_count as f32 * base_font * 1.5;

    // Add slide padding (4rem top + 4rem bottom)
    height += base_font * 8.0;

    height
}

/// Get the available slide height based on aspect ratio.
pub fn available_height(aspect: &AspectRatio) -> f32 {
    // Assuming 1920x1080 for 16:9 and 1024x768 for 4:3
    match aspect {
        AspectRatio::Wide => 1080.0,
        AspectRatio::Standard => 768.0,
    }
}

/// Check all slides for overflow and return warnings.
pub fn check_overflow(
    slides_html: &[String],
    aspect: &AspectRatio,
) -> Vec<OverflowResult> {
    let avail = available_height(aspect);

    slides_html
        .iter()
        .enumerate()
        .filter_map(|(i, html)| {
            let estimated = estimate_content_height(html, aspect);
            if estimated > avail {
                let overflow_pct = ((estimated - avail) / avail * 100.0).round();
                Some(OverflowResult {
                    slide_index: i,
                    estimated_height: estimated,
                    available_height: avail,
                    overflow_pct,
                })
            } else {
                None
            }
        })
        .collect()
}

/// Count occurrences of an HTML tag opening.
fn count_tag(html: &str, tag: &str) -> usize {
    html.matches(tag).count()
}

/// Estimate the number of characters per line based on aspect ratio.
fn estimate_chars_per_line(aspect: &AspectRatio) -> f32 {
    match aspect {
        AspectRatio::Wide => 60.0,
        AspectRatio::Standard => 45.0,
    }
}

/// Rough estimate of visible text length (strip tags).
fn estimate_text_length(html: &str) -> usize {
    let mut in_tag = false;
    let mut len = 0;
    for c in html.chars() {
        if c == '<' {
            in_tag = true;
        } else if c == '>' {
            in_tag = false;
        } else if !in_tag {
            len += 1;
        }
    }
    len
}

/// Extract the content between opening and closing tags.
fn extract_tag_contents<'a>(html: &'a str, open: &str, close: &str) -> Vec<&'a str> {
    let mut results = Vec::new();
    let mut remaining = html;
    while let Some(start) = remaining.find(open) {
        let after_open = &remaining[start..];
        if let Some(gt) = after_open.find('>') {
            let content_start = start + gt + 1;
            if let Some(end) = remaining[content_start..].find(close) {
                results.push(&remaining[content_start..content_start + end]);
                remaining = &remaining[content_start + end + close.len()..];
            } else {
                break;
            }
        } else {
            break;
        }
    }
    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_count_tag() {
        assert_eq!(count_tag("<p>Hello</p><p>World</p>", "<p"), 2);
        assert_eq!(count_tag("<h1>Title</h1>", "<h1"), 1);
        assert_eq!(count_tag("no tags here", "<p"), 0);
    }

    #[test]
    fn test_estimate_text_length() {
        assert_eq!(estimate_text_length("<p>Hello</p>"), 5);
        assert_eq!(estimate_text_length("<h1>Title</h1><p>Body</p>"), 9);
        assert_eq!(estimate_text_length("plain text"), 10);
    }

    #[test]
    fn test_extract_tag_contents() {
        let html = "<pre><code>line1\nline2</code></pre>";
        let contents = extract_tag_contents(html, "<pre", "</pre>");
        assert_eq!(contents.len(), 1);
        assert!(contents[0].contains("line1"));
    }

    #[test]
    fn test_no_overflow_simple_slide() {
        let html = "<h1>Title</h1><p>Short content</p>";
        let results = check_overflow(&[html.to_string()], &AspectRatio::Wide);
        assert!(results.is_empty());
    }

    #[test]
    fn test_overflow_with_lots_of_content() {
        // Create a slide with excessive content
        let mut html = String::from("<h1>Title</h1>");
        for i in 0..50 {
            html.push_str(&format!("<p>Paragraph {} with some reasonably long text content that should contribute to the estimated height of this slide.</p>", i));
        }
        let results = check_overflow(&[html], &AspectRatio::Wide);
        assert!(!results.is_empty());
        assert!(results[0].overflow_pct > 0.0);
    }

    #[test]
    fn test_available_height_wide() {
        assert_eq!(available_height(&AspectRatio::Wide), 1080.0);
    }

    #[test]
    fn test_available_height_standard() {
        assert_eq!(available_height(&AspectRatio::Standard), 768.0);
    }

    #[test]
    fn test_estimate_height_increases_with_content() {
        let short = "<p>Short</p>";
        let long = "<p>Short</p><p>More</p><p>Even more</p><h1>Title</h1><ul><li>A</li><li>B</li><li>C</li></ul>";

        let short_h = estimate_content_height(short, &AspectRatio::Wide);
        let long_h = estimate_content_height(long, &AspectRatio::Wide);
        assert!(long_h > short_h, "Longer content should have greater estimated height");
    }

    #[test]
    fn test_multiple_slides_overflow() {
        let ok = "<h1>Title</h1>".to_string();
        let mut big = String::from("<h1>Title</h1>");
        for _ in 0..60 {
            big.push_str("<p>Long paragraph with lots of text that takes up space on the slide.</p>");
        }

        let results = check_overflow(&[ok, big], &AspectRatio::Wide);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].slide_index, 1);
    }
}
