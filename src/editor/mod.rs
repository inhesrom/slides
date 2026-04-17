pub mod page;
pub mod serialize;
pub mod types;

use anyhow::Result;

use crate::parser::frontmatter;

use self::types::{EditorConfig, EditorDeck, EditorLayout, EditorSlide};

/// Decompose a markdown presentation into structured editor state.
/// Unlike `parser::parse()`, this preserves raw markdown content (lossless).
pub fn deck_to_editor(input: &str) -> Result<EditorDeck> {
    let (deck_config, body) = frontmatter::extract(input)?;
    let config = EditorConfig::from(&deck_config);
    let slides = split_editor_slides(&body);

    Ok(EditorDeck { config, slides })
}

/// Separator detection — mirrors `parser::is_separator`.
fn is_separator(line: &str) -> bool {
    let trimmed = line.trim();
    if !trimmed.starts_with("---") {
        return false;
    }
    let after_hyphens = trimmed.trim_start_matches('-');
    after_hyphens.is_empty() || after_hyphens.starts_with(' ') || after_hyphens.starts_with('{')
}

/// Attributes carried from a slide separator line into the next slide.
#[derive(Default)]
struct SeparatorAttrs {
    transition: Option<String>,
    class: Option<String>,
    title_size: Option<String>,
    body_size: Option<String>,
}

/// Parse separator attributes — mirrors `parser::parse_separator_attrs`.
fn parse_separator_attrs(line: &str) -> SeparatorAttrs {
    let trimmed = line.trim();
    let after_hyphens = trimmed.trim_start_matches('-').trim();

    let mut out = SeparatorAttrs::default();
    if let Some(inner) = after_hyphens
        .strip_prefix('{')
        .and_then(|s| s.strip_suffix('}'))
    {
        for part in inner.split(',') {
            let part = part.trim();
            if let Some((key, value)) = part.split_once(':') {
                let key = key.trim();
                let value = value.trim().trim_matches('"').trim_matches('\'');
                match key {
                    "transition" => out.transition = Some(value.to_string()),
                    "class" => out.class = Some(value.to_string()),
                    "title_size" => out.title_size = Some(value.to_string()),
                    "body_size" => out.body_size = Some(value.to_string()),
                    _ => {}
                }
            }
        }
    }
    out
}

/// Split the body into editor slides, preserving raw content.
fn split_editor_slides(body: &str) -> Vec<EditorSlide> {
    let mut slides = Vec::new();
    let mut current_lines: Vec<&str> = Vec::new();
    let mut current_attrs = SeparatorAttrs::default();
    let mut first = true;

    for line in body.lines() {
        if is_separator(line) {
            if first {
                if !current_lines.is_empty() && current_lines.iter().any(|l| !l.trim().is_empty())
                {
                    let content = current_lines.join("\n");
                    slides.push(build_editor_slide(
                        &content,
                        std::mem::take(&mut current_attrs),
                    ));
                }
                first = false;
            } else {
                let content = current_lines.join("\n");
                slides.push(build_editor_slide(
                    &content,
                    std::mem::take(&mut current_attrs),
                ));
            }
            current_lines.clear();
            current_attrs = parse_separator_attrs(line);
        } else {
            current_lines.push(line);
        }
    }

    // Last slide
    if current_lines.iter().any(|l| !l.trim().is_empty()) {
        let content = current_lines.join("\n");
        slides.push(build_editor_slide(
            &content,
            std::mem::take(&mut current_attrs),
        ));
    }

    slides
}

/// Build an EditorSlide from raw content, extracting notes and layout.
fn build_editor_slide(content: &str, attrs: SeparatorAttrs) -> EditorSlide {
    // Extract block notes (:::notes ... :::)
    let (notes, content) = extract_notes_raw(content);

    // Extract layout (:::split/grid/stack ... :::)
    let (layout, remaining_content) = extract_layout_raw(&content);

    EditorSlide {
        content: remaining_content.trim().to_string(),
        transition: attrs.transition,
        class: attrs.class,
        title_size: attrs.title_size,
        body_size: attrs.body_size,
        notes,
        layout,
    }
}

/// Extract `:::notes ... :::` blocks from raw content, returning (notes_text, remaining_content).
fn extract_notes_raw(content: &str) -> (String, String) {
    let mut notes_parts = Vec::new();
    let mut cleaned = String::new();
    let mut in_notes = false;
    let mut note_buf = String::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed == ":::notes" {
            in_notes = true;
            note_buf.clear();
        } else if in_notes && trimmed == ":::" {
            in_notes = false;
            notes_parts.push(note_buf.trim().to_string());
        } else if in_notes {
            note_buf.push_str(line);
            note_buf.push('\n');
        } else {
            cleaned.push_str(line);
            cleaned.push('\n');
        }
    }

    (notes_parts.join("\n\n"), cleaned)
}

/// Extract layout directives from raw content, returning (layout, remaining_content).
/// If a layout is found, remaining_content will be empty (layouts consume the whole slide).
fn extract_layout_raw(content: &str) -> (Option<EditorLayout>, String) {
    let lines: Vec<&str> = content.lines().collect();

    // Find layout opening directive
    let mut start = None;
    let mut kind = String::new();
    let mut params = String::new();

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with(":::split") {
            start = Some(i);
            kind = "split".to_string();
            params = trimmed
                .strip_prefix(":::split")
                .unwrap_or("")
                .trim()
                .to_string();
        } else if trimmed.starts_with(":::grid") {
            start = Some(i);
            kind = "grid".to_string();
            params = trimmed
                .strip_prefix(":::grid")
                .unwrap_or("")
                .trim()
                .to_string();
        } else if trimmed == ":::stack" {
            start = Some(i);
            kind = "stack".to_string();
            params = String::new();
        } else if start.is_some() && trimmed == ":::" {
            // Found closing directive
            let inner_text = lines[start.unwrap() + 1..i].join("\n");
            let regions: Vec<String> = inner_text
                .split("\n+++\n")
                .map(|s| s.trim().to_string())
                .collect();

            // Remaining content is everything outside the layout block
            let mut remaining = String::new();
            for line in &lines[..start.unwrap()] {
                remaining.push_str(line);
                remaining.push('\n');
            }
            for line in &lines[i + 1..] {
                remaining.push_str(line);
                remaining.push('\n');
            }

            return (
                Some(EditorLayout {
                    kind,
                    params,
                    regions,
                }),
                remaining,
            );
        }
    }

    (None, content.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deck_to_editor_basic() {
        let input = "---\ntitle: Test\ntheme: minimal\n---\n\n# Slide 1\n\n---\n\n# Slide 2\n";
        let deck = deck_to_editor(input).unwrap();
        assert_eq!(deck.config.title.as_deref(), Some("Test"));
        assert_eq!(deck.slides.len(), 2);
        assert!(deck.slides[0].content.contains("# Slide 1"));
        assert!(deck.slides[1].content.contains("# Slide 2"));
    }

    #[test]
    fn test_deck_to_editor_no_frontmatter() {
        let input = "# Slide 1\n\n---\n\n# Slide 2\n";
        let deck = deck_to_editor(input).unwrap();
        assert!(deck.config.title.is_none());
        assert_eq!(deck.slides.len(), 2);
    }

    #[test]
    fn test_deck_to_editor_with_transition() {
        let input = "# Slide 1\n\n--- {transition: fade}\n\n# Slide 2\n";
        let deck = deck_to_editor(input).unwrap();
        assert!(deck.slides[0].transition.is_none());
        assert_eq!(deck.slides[1].transition.as_deref(), Some("fade"));
    }

    #[test]
    fn test_deck_to_editor_with_class() {
        let input = "# Slide 1\n\n--- {class: centered}\n\n# Slide 2\n";
        let deck = deck_to_editor(input).unwrap();
        assert_eq!(deck.slides[1].class.as_deref(), Some("centered"));
    }

    #[test]
    fn test_deck_to_editor_with_block_notes() {
        let input = "# Title\n\n:::notes\nSpeaker note here\n:::\n";
        let deck = deck_to_editor(input).unwrap();
        assert_eq!(deck.slides[0].notes, "Speaker note here");
        assert!(!deck.slides[0].content.contains(":::notes"));
    }

    #[test]
    fn test_deck_to_editor_inline_notes_preserved() {
        // Inline notes should stay in content (they're markdown syntax)
        let input = "Text ^[inline note] more\n";
        let deck = deck_to_editor(input).unwrap();
        assert!(deck.slides[0].content.contains("^[inline note]"));
    }

    #[test]
    fn test_deck_to_editor_with_split_layout() {
        let input = ":::split 60/40\nLeft content\n+++\nRight content\n:::\n";
        let deck = deck_to_editor(input).unwrap();
        let layout = deck.slides[0].layout.as_ref().unwrap();
        assert_eq!(layout.kind, "split");
        assert_eq!(layout.params, "60/40");
        assert_eq!(layout.regions.len(), 2);
        assert!(layout.regions[0].contains("Left content"));
        assert!(layout.regions[1].contains("Right content"));
    }

    #[test]
    fn test_deck_to_editor_with_grid_layout() {
        let input = ":::grid 2x2\nA\n+++\nB\n+++\nC\n+++\nD\n:::\n";
        let deck = deck_to_editor(input).unwrap();
        let layout = deck.slides[0].layout.as_ref().unwrap();
        assert_eq!(layout.kind, "grid");
        assert_eq!(layout.params, "2x2");
        assert_eq!(layout.regions.len(), 4);
    }

    #[test]
    fn test_deck_to_editor_with_stack_layout() {
        let input = ":::stack\nTop\n+++\nBottom\n:::\n";
        let deck = deck_to_editor(input).unwrap();
        let layout = deck.slides[0].layout.as_ref().unwrap();
        assert_eq!(layout.kind, "stack");
        assert!(layout.params.is_empty());
        assert_eq!(layout.regions.len(), 2);
    }

    #[test]
    fn test_deck_to_editor_layout_with_notes() {
        let input = ":::split 50/50\nLeft\n+++\nRight\n:::\n\n:::notes\nMy note\n:::\n";
        let deck = deck_to_editor(input).unwrap();
        assert!(deck.slides[0].layout.is_some());
        assert_eq!(deck.slides[0].notes, "My note");
    }

    #[test]
    fn test_deck_to_editor_single_slide() {
        let input = "# Just one slide\n\nContent here\n";
        let deck = deck_to_editor(input).unwrap();
        assert_eq!(deck.slides.len(), 1);
    }

    #[test]
    fn test_deck_to_editor_empty() {
        let input = "";
        let deck = deck_to_editor(input).unwrap();
        assert!(deck.slides.is_empty());
    }

    #[test]
    fn test_extract_notes_raw_basic() {
        let input = "Content\n\n:::notes\nNote text\n:::\n\nMore content";
        let (notes, cleaned) = extract_notes_raw(input);
        assert_eq!(notes, "Note text");
        assert!(!cleaned.contains(":::notes"));
        assert!(cleaned.contains("Content"));
        assert!(cleaned.contains("More content"));
    }

    #[test]
    fn test_extract_notes_raw_multiple() {
        let input = ":::notes\nNote 1\n:::\n\n:::notes\nNote 2\n:::";
        let (notes, _) = extract_notes_raw(input);
        assert!(notes.contains("Note 1"));
        assert!(notes.contains("Note 2"));
    }

    #[test]
    fn test_extract_notes_raw_none() {
        let input = "Just content";
        let (notes, cleaned) = extract_notes_raw(input);
        assert!(notes.is_empty());
        assert!(cleaned.contains("Just content"));
    }

    #[test]
    fn test_extract_layout_raw_split() {
        let input = ":::split 70/30\nLeft\n+++\nRight\n:::";
        let (layout, remaining) = extract_layout_raw(input);
        let layout = layout.unwrap();
        assert_eq!(layout.kind, "split");
        assert_eq!(layout.params, "70/30");
        assert_eq!(layout.regions.len(), 2);
        assert!(remaining.trim().is_empty());
    }

    #[test]
    fn test_extract_layout_raw_none() {
        let input = "Regular content\nNo layout here";
        let (layout, remaining) = extract_layout_raw(input);
        assert!(layout.is_none());
        assert!(remaining.contains("Regular content"));
    }

    // --- Round-trip tests ---

    #[test]
    fn test_round_trip_basic() {
        let deck = EditorDeck {
            config: EditorConfig {
                title: Some("Test".to_string()),
                theme: "minimal".to_string(),
                aspect: "16:9".to_string(),
                transition: "slide".to_string(),
                highlight_theme: "github".to_string(),
                color_scheme: "light".to_string(),
                auto_fit: "warn".to_string(),
                export_images: "relative".to_string(),
                title_size: "67px".to_string(),
                body_size: "32px".to_string(),
            },
            slides: vec![
                EditorSlide {
                    content: "# Slide 1\n\nContent".to_string(),
                    ..EditorSlide::default()
                },
                EditorSlide {
                    content: "# Slide 2".to_string(),
                    ..EditorSlide::default()
                },
            ],
        };
        let md = serialize::serialize_deck(&deck);
        let reparsed = deck_to_editor(&md).unwrap();
        assert_eq!(reparsed.config.title.as_deref(), Some("Test"));
        assert_eq!(reparsed.slides.len(), 2);
        assert!(reparsed.slides[0].content.contains("# Slide 1"));
        assert!(reparsed.slides[1].content.contains("# Slide 2"));
    }

    #[test]
    fn test_round_trip_with_notes() {
        let deck = EditorDeck {
            config: EditorConfig {
                title: None,
                theme: "dark".to_string(),
                aspect: "4:3".to_string(),
                transition: "fade".to_string(),
                highlight_theme: "github".to_string(),
                color_scheme: "dark".to_string(),
                auto_fit: "warn".to_string(),
                export_images: "relative".to_string(),
                title_size: "67px".to_string(),
                body_size: "32px".to_string(),
            },
            slides: vec![EditorSlide {
                content: "# Title".to_string(),
                notes: "Speaker note".to_string(),
                ..EditorSlide::default()
            }],
        };
        let md = serialize::serialize_deck(&deck);
        let reparsed = deck_to_editor(&md).unwrap();
        assert_eq!(reparsed.slides[0].notes, "Speaker note");
        assert!(reparsed.slides[0].content.contains("# Title"));
    }

    #[test]
    fn test_round_trip_with_layout() {
        let deck = EditorDeck {
            config: EditorConfig {
                title: Some("Layout Test".to_string()),
                theme: "minimal".to_string(),
                aspect: "16:9".to_string(),
                transition: "slide".to_string(),
                highlight_theme: "github".to_string(),
                color_scheme: "light".to_string(),
                auto_fit: "warn".to_string(),
                export_images: "relative".to_string(),
                title_size: "67px".to_string(),
                body_size: "32px".to_string(),
            },
            slides: vec![EditorSlide {
                layout: Some(EditorLayout {
                    kind: "split".to_string(),
                    params: "60/40".to_string(),
                    regions: vec!["Left content".to_string(), "Right content".to_string()],
                }),
                ..EditorSlide::default()
            }],
        };
        let md = serialize::serialize_deck(&deck);
        let reparsed = deck_to_editor(&md).unwrap();
        let layout = reparsed.slides[0].layout.as_ref().unwrap();
        assert_eq!(layout.kind, "split");
        assert_eq!(layout.params, "60/40");
        assert_eq!(layout.regions.len(), 2);
        assert!(layout.regions[0].contains("Left content"));
        assert!(layout.regions[1].contains("Right content"));
    }

    #[test]
    fn test_round_trip_with_transitions() {
        let deck = EditorDeck {
            config: EditorConfig {
                title: Some("Trans".to_string()),
                theme: "minimal".to_string(),
                aspect: "16:9".to_string(),
                transition: "slide".to_string(),
                highlight_theme: "github".to_string(),
                color_scheme: "light".to_string(),
                auto_fit: "warn".to_string(),
                export_images: "relative".to_string(),
                title_size: "67px".to_string(),
                body_size: "32px".to_string(),
            },
            slides: vec![
                EditorSlide {
                    content: "# First".to_string(),
                    ..EditorSlide::default()
                },
                EditorSlide {
                    content: "# Second".to_string(),
                    transition: Some("fade".to_string()),
                    class: Some("centered".to_string()),
                    ..EditorSlide::default()
                },
            ],
        };
        let md = serialize::serialize_deck(&deck);
        let reparsed = deck_to_editor(&md).unwrap();
        assert!(reparsed.slides[0].transition.is_none());
        assert_eq!(reparsed.slides[1].transition.as_deref(), Some("fade"));
        assert_eq!(reparsed.slides[1].class.as_deref(), Some("centered"));
    }

    #[test]
    fn test_round_trip_centered_class() {
        let deck = EditorDeck {
            config: EditorConfig {
                title: Some("Centered".to_string()),
                theme: "minimal".to_string(),
                aspect: "16:9".to_string(),
                transition: "slide".to_string(),
                highlight_theme: "github".to_string(),
                color_scheme: "light".to_string(),
                auto_fit: "warn".to_string(),
                export_images: "relative".to_string(),
                title_size: "67px".to_string(),
                body_size: "32px".to_string(),
            },
            slides: vec![
                EditorSlide {
                    content: "# Title Page".to_string(),
                    class: Some("centered".to_string()),
                    ..EditorSlide::default()
                },
                EditorSlide {
                    content: "# Normal Slide".to_string(),
                    ..EditorSlide::default()
                },
            ],
        };
        let md = serialize::serialize_deck(&deck);
        assert!(md.contains("class: centered"), "Markdown should contain class: centered");

        let reparsed = deck_to_editor(&md).unwrap();
        assert_eq!(reparsed.slides[0].class.as_deref(), Some("centered"));
        assert!(reparsed.slides[1].class.is_none());
    }

    #[test]
    fn test_centered_class_only_on_separator() {
        // First slide has no separator, so centered class comes from the separator of slide index 0
        // when it's not the first slide
        let input = "# First\n\n--- {class: centered}\n\n# Centered Title\n\n---\n\n# Normal\n";
        let deck = deck_to_editor(input).unwrap();
        assert!(deck.slides[0].class.is_none(), "First slide has no separator attrs");
        assert_eq!(deck.slides[1].class.as_deref(), Some("centered"));
        assert!(deck.slides[2].class.is_none());
    }

    #[test]
    fn test_round_trip_demo_file() {
        let demo = include_str!("../../examples/demo.md");
        let deck = deck_to_editor(demo).unwrap();

        // Verify structure
        assert_eq!(deck.config.title.as_deref(), Some("slides — Demo Deck"));
        assert_eq!(deck.config.theme, "minimal");
        assert!(deck.slides.len() >= 5, "Demo should have multiple slides");

        // Verify round-trip: serialize and re-parse
        let serialized = serialize::serialize_deck(&deck);
        let reparsed = deck_to_editor(&serialized).unwrap();
        assert_eq!(reparsed.slides.len(), deck.slides.len());
        assert_eq!(reparsed.config.title, deck.config.title);

        // Verify specific features survived
        // Slide with split layout
        let has_split = deck
            .slides
            .iter()
            .any(|s| s.layout.as_ref().is_some_and(|l| l.kind == "split"));
        assert!(has_split, "Demo should have a split layout");

        // Slide with grid layout
        let has_grid = deck
            .slides
            .iter()
            .any(|s| s.layout.as_ref().is_some_and(|l| l.kind == "grid"));
        assert!(has_grid, "Demo should have a grid layout");

        // Slide with notes
        let has_notes = deck.slides.iter().any(|s| !s.notes.is_empty());
        assert!(has_notes, "Demo should have speaker notes");

        // Slide with transition
        let has_transition = reparsed
            .slides
            .iter()
            .any(|s| s.transition.as_deref() == Some("fade"));
        assert!(has_transition, "Demo should have a fade transition");
    }
}
