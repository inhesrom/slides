pub mod directives;
pub mod frontmatter;
pub mod markdown;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use self::directives::LayoutDirective;
use self::frontmatter::DeckConfig;

#[derive(Debug, Clone)]
pub struct Deck {
    pub config: DeckConfig,
    pub slides: Vec<Slide>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct SlideAttrs {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transition: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub class: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timing: Option<String>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Slide {
    pub html: String,
    pub attrs: SlideAttrs,
    pub speaker_notes: Vec<SpeakerNote>,
    pub layout: Option<LayoutDirective>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeakerNote {
    pub text: String,
}

/// Parse a full markdown presentation into a structured Deck.
pub fn parse(input: &str) -> Result<Deck> {
    let (config, body) = frontmatter::extract(input)?;
    let slides = split_slides(&body)
        .into_iter()
        .map(build_slide)
        .collect();
    Ok(Deck { config, slides })
}

/// Transform raw slide content and attributes into a fully processed Slide.
fn build_slide(raw: RawSlide) -> Slide {
    let (notes_blocks, content) = directives::extract_notes(&raw.content);
    let (inline_notes, content) = directives::extract_inline_notes(&content);
    let speaker_notes = notes_blocks.into_iter().chain(inline_notes).collect();
    let (layout, regions) = directives::extract_layout(&content);
    let html = match &layout {
        Some(dir) => render_layout(dir, &regions),
        None => markdown::render(&content),
    };
    Slide {
        html,
        attrs: raw.attrs,
        speaker_notes,
        layout,
    }
}

/// Convert a LayoutDirective into an inline CSS style string.
fn layout_css_style(layout: &LayoutDirective) -> String {
    match layout {
        LayoutDirective::Split { ratios } => {
            let cols: Vec<String> = ratios.iter().map(|r| format!("{}fr", r)).collect();
            format!("display:grid;grid-template-columns:{};gap:2rem;", cols.join(" "))
        }
        LayoutDirective::Grid { cols, rows } => format!(
            "display:grid;grid-template-columns:repeat({cols},1fr);grid-template-rows:repeat({rows},1fr);gap:2rem;"
        ),
        LayoutDirective::Stack => "display:flex;flex-direction:column;gap:2rem;".to_string(),
    }
}

/// Render layout regions as HTML wrapped in a styled container div.
fn render_layout(layout: &LayoutDirective, regions: &[String]) -> String {
    let style = layout_css_style(layout);
    let regions_html: String = regions
        .iter()
        .map(|r| format!("<div class=\"region\">{}</div>", markdown::render(r)))
        .collect();
    format!("<div class=\"layout\" style=\"{style}\">{regions_html}</div>")
}

struct RawSlide {
    content: String,
    attrs: SlideAttrs,
}

/// Split the post-frontmatter body on `---` separators into raw slides.
fn split_slides(body: &str) -> Vec<RawSlide> {
    let mut slides = Vec::new();
    let mut current = String::new();
    let mut current_attrs = SlideAttrs::default();
    let mut first = true;

    for line in body.lines() {
        if is_separator(line) {
            if first {
                if !current.trim().is_empty() {
                    slides.push(RawSlide {
                        content: current.clone(),
                        attrs: current_attrs,
                    });
                }
                first = false;
            } else {
                slides.push(RawSlide {
                    content: current.clone(),
                    attrs: current_attrs,
                });
            }
            current.clear();
            current_attrs = parse_separator_attrs(line);
        } else {
            current.push_str(line);
            current.push('\n');
        }
    }

    if !current.trim().is_empty() {
        slides.push(RawSlide {
            content: current,
            attrs: current_attrs,
        });
    }

    slides
}

/// Check whether a line is a slide separator (three or more hyphens).
fn is_separator(line: &str) -> bool {
    let trimmed = line.trim();
    if !trimmed.starts_with("---") {
        return false;
    }
    let after_hyphens = trimmed.trim_start_matches('-');
    after_hyphens.is_empty()
        || after_hyphens.starts_with(' ')
        || after_hyphens.starts_with('{')
}

/// Parse key-value attributes from a slide separator line (e.g. `--- {transition: fade}`).
fn parse_separator_attrs(line: &str) -> SlideAttrs {
    let trimmed = line.trim();
    let after_hyphens = trimmed.trim_start_matches('-').trim();

    if let Some(inner) = after_hyphens.strip_prefix('{').and_then(|s| s.strip_suffix('}')) {
        let mut attrs = SlideAttrs::default();
        for part in inner.split(',') {
            let part = part.trim();
            if let Some((key, value)) = part.split_once(':') {
                let key = key.trim();
                let value = value.trim().trim_matches('"').trim_matches('\'');
                match key {
                    "transition" => attrs.transition = Some(value.to_string()),
                    "class" => attrs.class = Some(value.to_string()),
                    "timing" => attrs.timing = Some(value.to_string()),
                    _ => {}
                }
            }
        }
        attrs
    } else {
        SlideAttrs::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- is_separator tests ---

    #[test]
    fn test_basic_separator() {
        assert!(is_separator("---"));
    }

    #[test]
    fn test_long_separator() {
        assert!(is_separator("-----"));
    }

    #[test]
    fn test_separator_with_attrs() {
        assert!(is_separator("--- {transition: fade}"));
    }

    #[test]
    fn test_not_separator_text() {
        assert!(!is_separator("some text"));
    }

    #[test]
    fn test_not_separator_two_hyphens() {
        assert!(!is_separator("--"));
    }

    #[test]
    fn test_separator_with_whitespace() {
        assert!(is_separator("  ---  "));
    }

    // --- parse_separator_attrs tests ---

    #[test]
    fn test_attrs_transition() {
        let attrs = parse_separator_attrs("--- {transition: fade}");
        assert_eq!(attrs.transition.as_deref(), Some("fade"));
    }

    #[test]
    fn test_attrs_class() {
        let attrs = parse_separator_attrs("--- {class: centered}");
        assert_eq!(attrs.class.as_deref(), Some("centered"));
    }

    #[test]
    fn test_attrs_timing() {
        let attrs = parse_separator_attrs("--- {timing: 45s}");
        assert_eq!(attrs.timing.as_deref(), Some("45s"));
    }

    #[test]
    fn test_attrs_multiple() {
        let attrs = parse_separator_attrs("--- {transition: fade, class: centered}");
        assert_eq!(attrs.transition.as_deref(), Some("fade"));
        assert_eq!(attrs.class.as_deref(), Some("centered"));
    }

    #[test]
    fn test_attrs_none() {
        let attrs = parse_separator_attrs("---");
        assert!(attrs.transition.is_none());
        assert!(attrs.class.is_none());
        assert!(attrs.timing.is_none());
    }

    #[test]
    fn test_attrs_quoted_values() {
        let attrs = parse_separator_attrs("--- {transition: \"fade\"}");
        assert_eq!(attrs.transition.as_deref(), Some("fade"));
    }

    // --- split_slides tests ---

    #[test]
    fn test_split_two_slides() {
        let slides = split_slides("# Slide 1\n\n---\n\n# Slide 2\n");
        assert_eq!(slides.len(), 2);
        assert!(slides[0].content.contains("Slide 1"));
        assert!(slides[1].content.contains("Slide 2"));
    }

    #[test]
    fn test_split_three_slides() {
        let slides = split_slides("A\n---\nB\n---\nC\n");
        assert_eq!(slides.len(), 3);
    }

    #[test]
    fn test_split_with_attrs() {
        let slides = split_slides("A\n--- {transition: fade}\nB\n");
        assert_eq!(slides.len(), 2);
        assert_eq!(slides[1].attrs.transition.as_deref(), Some("fade"));
    }

    #[test]
    fn test_split_single_slide() {
        let slides = split_slides("# Just one slide\n\nContent here\n");
        assert_eq!(slides.len(), 1);
    }

    #[test]
    fn test_split_empty_body() {
        let slides = split_slides("");
        assert_eq!(slides.len(), 0);
    }

    // --- full parse tests ---

    #[test]
    fn test_parse_basic_deck() {
        let input = "---\ntitle: Test\n---\n\n# Slide 1\n\n---\n\n# Slide 2\n";
        let deck = parse(input).unwrap();
        assert_eq!(deck.config.title.as_deref(), Some("Test"));
        assert_eq!(deck.slides.len(), 2);
    }

    #[test]
    fn test_parse_no_frontmatter() {
        let input = "# Slide 1\n\n---\n\n# Slide 2\n";
        let deck = parse(input).unwrap();
        assert!(deck.config.title.is_none());
        assert_eq!(deck.slides.len(), 2);
    }

    #[test]
    fn test_parse_with_notes() {
        let input = "# Title ^[Speaker note]\n\n---\n\n:::notes\nBlock note\n:::\n\n# Slide 2\n";
        let deck = parse(input).unwrap();
        assert_eq!(deck.slides[0].speaker_notes.len(), 1);
        assert_eq!(deck.slides[0].speaker_notes[0].text, "Speaker note");
        assert_eq!(deck.slides[1].speaker_notes.len(), 1);
        assert_eq!(deck.slides[1].speaker_notes[0].text, "Block note");
    }

    #[test]
    fn test_parse_with_layout() {
        let input = ":::split 60/40\nLeft\n+++\nRight\n:::\n";
        let deck = parse(input).unwrap();
        assert_eq!(deck.slides.len(), 1);
        assert!(deck.slides[0].layout.is_some());
        assert!(deck.slides[0].html.contains("class=\"layout\""));
    }

    #[test]
    fn test_parse_html_output() {
        let input = "# Hello\n\nA paragraph\n";
        let deck = parse(input).unwrap();
        assert!(deck.slides[0].html.contains("<h1>Hello</h1>"));
        assert!(deck.slides[0].html.contains("<p>A paragraph</p>"));
    }

    #[test]
    fn test_parse_notes_stripped_from_html() {
        let input = "Text ^[hidden note] visible\n";
        let deck = parse(input).unwrap();
        assert!(!deck.slides[0].html.contains("hidden note"));
        assert!(deck.slides[0].html.contains("visible"));
    }

    // --- render_layout tests ---

    #[test]
    fn test_render_split_layout() {
        let layout = LayoutDirective::Split {
            ratios: vec![60.0, 40.0],
        };
        let regions = vec!["# Left".to_string(), "# Right".to_string()];
        let html = render_layout(&layout, &regions);
        assert!(html.contains("grid-template-columns:60fr 40fr"));
        assert!(html.contains("class=\"region\""));
        assert!(html.contains("<h1>Left</h1>"));
        assert!(html.contains("<h1>Right</h1>"));
    }

    #[test]
    fn test_render_grid_layout() {
        let layout = LayoutDirective::Grid { cols: 2, rows: 2 };
        let regions = vec!["A".to_string(), "B".to_string(), "C".to_string(), "D".to_string()];
        let html = render_layout(&layout, &regions);
        assert!(html.contains("repeat(2,1fr)"));
        assert_eq!(html.matches("class=\"region\"").count(), 4);
    }

    #[test]
    fn test_render_stack_layout() {
        let layout = LayoutDirective::Stack;
        let regions = vec!["Top".to_string(), "Bottom".to_string()];
        let html = render_layout(&layout, &regions);
        assert!(html.contains("flex-direction:column"));
    }

    #[test]
    fn test_layout_css_split() {
        let style = layout_css_style(&LayoutDirective::Split {
            ratios: vec![70.0, 30.0],
        });
        assert!(style.contains("70fr 30fr"));
    }

    #[test]
    fn test_layout_css_grid() {
        let style = layout_css_style(&LayoutDirective::Grid { cols: 3, rows: 2 });
        assert!(style.contains("repeat(3,1fr)"));
        assert!(style.contains("repeat(2,1fr)"));
    }

    #[test]
    fn test_layout_css_stack() {
        let style = layout_css_style(&LayoutDirective::Stack);
        assert!(style.contains("flex-direction:column"));
    }
}
