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

/// Parse key-value attributes from a slide separator line.
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
