use super::types::{EditorDeck, EditorLayout, EditorSlide};

/// Reconstruct a full markdown file from editor state.
pub fn serialize_deck(deck: &EditorDeck) -> String {
    let mut out = String::new();

    // Frontmatter
    out.push_str("---\n");
    if let Some(title) = &deck.config.title {
        out.push_str(&format!("title: {}\n", yaml_value(title)));
    }
    out.push_str(&format!("theme: {}\n", deck.config.theme));
    out.push_str(&format!("aspect: \"{}\"\n", deck.config.aspect));
    out.push_str(&format!("transition: {}\n", deck.config.transition));
    if deck.config.color_scheme != "light" {
        out.push_str(&format!("color_scheme: {}\n", deck.config.color_scheme));
    }
    if deck.config.highlight_theme != "github" {
        out.push_str(&format!(
            "highlight_theme: {}\n",
            deck.config.highlight_theme
        ));
    }
    if deck.config.auto_fit != "warn" {
        out.push_str(&format!("auto_fit: {}\n", deck.config.auto_fit));
    }
    if deck.config.export_images != "relative" {
        out.push_str(&format!("export_images: {}\n", deck.config.export_images));
    }
    if deck.config.title_size != "67px" {
        out.push_str(&format!("title_size: \"{}\"\n", deck.config.title_size));
    }
    if deck.config.body_size != "32px" {
        out.push_str(&format!("body_size: \"{}\"\n", deck.config.body_size));
    }
    out.push_str("---\n");

    // Slides
    for (i, slide) in deck.slides.iter().enumerate() {
        if i > 0 {
            out.push('\n');
            out.push_str(&serialize_separator(slide));
            out.push('\n');
        } else if slide.transition.is_some() || slide.class.is_some() {
            // First slide with attrs needs a separator too
            out.push('\n');
            out.push_str(&serialize_separator(slide));
            out.push('\n');
        }

        out.push('\n');
        serialize_slide_body(&mut out, slide);
    }

    out
}

/// Serialize a slide separator line with optional attributes.
fn serialize_separator(slide: &EditorSlide) -> String {
    let mut attrs = Vec::new();
    if let Some(t) = &slide.transition {
        attrs.push(format!("transition: {}", t));
    }
    if let Some(c) = &slide.class {
        attrs.push(format!("class: {}", c));
    }

    if attrs.is_empty() {
        "---".to_string()
    } else {
        format!("--- {{{}}}", attrs.join(", "))
    }
}

/// Serialize the body of a slide (content + layout + notes).
fn serialize_slide_body(out: &mut String, slide: &EditorSlide) {
    if let Some(layout) = &slide.layout {
        serialize_layout(out, layout);
    } else {
        let content = slide.content.trim_end();
        if !content.is_empty() {
            out.push_str(content);
            out.push('\n');
        }
    }

    if !slide.notes.is_empty() {
        out.push('\n');
        out.push_str(":::notes\n");
        out.push_str(slide.notes.trim_end());
        out.push('\n');
        out.push_str(":::\n");
    }
}

/// Serialize a layout directive with its regions.
fn serialize_layout(out: &mut String, layout: &EditorLayout) {
    // Opening directive
    let directive = if layout.params.is_empty() {
        format!(":::{}", layout.kind)
    } else {
        format!(":::{} {}", layout.kind, layout.params)
    };
    out.push_str(&directive);
    out.push('\n');

    // Regions separated by +++
    for (i, region) in layout.regions.iter().enumerate() {
        if i > 0 {
            out.push_str("\n+++\n");
        }
        out.push('\n');
        let content = region.trim_end();
        if !content.is_empty() {
            out.push_str(content);
            out.push('\n');
        }
    }

    // Closing directive
    out.push_str("\n:::\n");
}

/// Escape a YAML string value if it contains special characters.
fn yaml_value(s: &str) -> String {
    if s.contains(':') || s.contains('#') || s.contains('"') || s.starts_with(' ') || s.ends_with(' ') {
        format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor::types::{EditorConfig, EditorDeck, EditorSlide};

    fn default_config() -> EditorConfig {
        EditorConfig {
            title: Some("Test Deck".to_string()),
            theme: "minimal".to_string(),
            aspect: "16:9".to_string(),
            transition: "slide".to_string(),
            highlight_theme: "github".to_string(),
            color_scheme: "light".to_string(),
            auto_fit: "warn".to_string(),
            export_images: "relative".to_string(),
            title_size: "67px".to_string(),
            body_size: "28px".to_string(),
        }
    }

    #[test]
    fn test_serialize_frontmatter() {
        let deck = EditorDeck {
            config: default_config(),
            slides: vec![EditorSlide {
                content: "# Hello".to_string(),
                ..EditorSlide::default()
            }],
        };
        let md = serialize_deck(&deck);
        assert!(md.starts_with("---\n"));
        assert!(md.contains("title: Test Deck\n"));
        assert!(md.contains("theme: minimal\n"));
        assert!(md.contains("aspect: \"16:9\"\n"));
        assert!(md.contains("transition: slide\n"));
        // Defaults should be omitted
        assert!(!md.contains("color_scheme:"));
        assert!(!md.contains("highlight_theme:"));
    }

    #[test]
    fn test_serialize_non_default_config() {
        let deck = EditorDeck {
            config: EditorConfig {
                color_scheme: "dark".to_string(),
                highlight_theme: "monokai".to_string(),
                ..default_config()
            },
            slides: vec![EditorSlide::default()],
        };
        let md = serialize_deck(&deck);
        assert!(md.contains("color_scheme: dark\n"));
        assert!(md.contains("highlight_theme: monokai\n"));
    }

    #[test]
    fn test_serialize_single_slide() {
        let deck = EditorDeck {
            config: default_config(),
            slides: vec![EditorSlide {
                content: "# Title\n\nSome content".to_string(),
                ..EditorSlide::default()
            }],
        };
        let md = serialize_deck(&deck);
        assert!(md.contains("# Title\n\nSome content\n"));
        // The only --- should be from frontmatter, no slide separator
        let after_frontmatter = md.split("---\n").nth(2).unwrap_or("");
        assert!(!after_frontmatter.contains("---"), "Found slide separator in single-slide deck");
    }

    #[test]
    fn test_serialize_multiple_slides() {
        let deck = EditorDeck {
            config: default_config(),
            slides: vec![
                EditorSlide {
                    content: "# Slide 1".to_string(),
                    ..EditorSlide::default()
                },
                EditorSlide {
                    content: "# Slide 2".to_string(),
                    ..EditorSlide::default()
                },
            ],
        };
        let md = serialize_deck(&deck);
        assert!(md.contains("# Slide 1"));
        assert!(md.contains("\n---\n"));
        assert!(md.contains("# Slide 2"));
    }

    #[test]
    fn test_serialize_slide_with_transition() {
        let deck = EditorDeck {
            config: default_config(),
            slides: vec![
                EditorSlide::default(),
                EditorSlide {
                    content: "Content".to_string(),
                    transition: Some("fade".to_string()),
                    ..EditorSlide::default()
                },
            ],
        };
        let md = serialize_deck(&deck);
        assert!(md.contains("--- {transition: fade}"));
    }

    #[test]
    fn test_serialize_slide_with_class() {
        let deck = EditorDeck {
            config: default_config(),
            slides: vec![
                EditorSlide::default(),
                EditorSlide {
                    content: "Content".to_string(),
                    class: Some("centered".to_string()),
                    ..EditorSlide::default()
                },
            ],
        };
        let md = serialize_deck(&deck);
        assert!(md.contains("--- {class: centered}"));
    }

    #[test]
    fn test_serialize_slide_with_multiple_attrs() {
        let deck = EditorDeck {
            config: default_config(),
            slides: vec![
                EditorSlide::default(),
                EditorSlide {
                    content: "Content".to_string(),
                    transition: Some("fade".to_string()),
                    class: Some("centered".to_string()),
                    ..EditorSlide::default()
                },
            ],
        };
        let md = serialize_deck(&deck);
        assert!(md.contains("--- {transition: fade, class: centered}"));
    }

    #[test]
    fn test_serialize_slide_with_notes() {
        let deck = EditorDeck {
            config: default_config(),
            slides: vec![EditorSlide {
                content: "# Title".to_string(),
                notes: "Speaker note here".to_string(),
                ..EditorSlide::default()
            }],
        };
        let md = serialize_deck(&deck);
        assert!(md.contains(":::notes\nSpeaker note here\n:::"));
    }

    #[test]
    fn test_serialize_split_layout() {
        let deck = EditorDeck {
            config: default_config(),
            slides: vec![EditorSlide {
                layout: Some(EditorLayout {
                    kind: "split".to_string(),
                    params: "60/40".to_string(),
                    regions: vec!["Left content".to_string(), "Right content".to_string()],
                }),
                ..EditorSlide::default()
            }],
        };
        let md = serialize_deck(&deck);
        assert!(md.contains(":::split 60/40\n"));
        assert!(md.contains("Left content"));
        assert!(md.contains("\n+++\n"));
        assert!(md.contains("Right content"));
        assert!(md.contains("\n:::\n"));
    }

    #[test]
    fn test_serialize_grid_layout() {
        let deck = EditorDeck {
            config: default_config(),
            slides: vec![EditorSlide {
                layout: Some(EditorLayout {
                    kind: "grid".to_string(),
                    params: "2x2".to_string(),
                    regions: vec![
                        "A".to_string(),
                        "B".to_string(),
                        "C".to_string(),
                        "D".to_string(),
                    ],
                }),
                ..EditorSlide::default()
            }],
        };
        let md = serialize_deck(&deck);
        assert!(md.contains(":::grid 2x2\n"));
        assert_eq!(md.matches("+++").count(), 3);
    }

    #[test]
    fn test_serialize_stack_layout() {
        let deck = EditorDeck {
            config: default_config(),
            slides: vec![EditorSlide {
                layout: Some(EditorLayout {
                    kind: "stack".to_string(),
                    params: String::new(),
                    regions: vec!["Top".to_string(), "Bottom".to_string()],
                }),
                ..EditorSlide::default()
            }],
        };
        let md = serialize_deck(&deck);
        assert!(md.contains(":::stack\n"));
    }

    #[test]
    fn test_serialize_layout_with_notes() {
        let deck = EditorDeck {
            config: default_config(),
            slides: vec![EditorSlide {
                layout: Some(EditorLayout {
                    kind: "split".to_string(),
                    params: "50/50".to_string(),
                    regions: vec!["Left".to_string(), "Right".to_string()],
                }),
                notes: "Layout notes".to_string(),
                ..EditorSlide::default()
            }],
        };
        let md = serialize_deck(&deck);
        assert!(md.contains(":::split 50/50\n"));
        assert!(md.contains(":::notes\nLayout notes\n:::"));
    }

    #[test]
    fn test_yaml_value_plain() {
        assert_eq!(yaml_value("hello"), "hello");
    }

    #[test]
    fn test_yaml_value_needs_quoting() {
        assert_eq!(yaml_value("key: value"), "\"key: value\"");
    }

    #[test]
    fn test_serialize_no_title() {
        let deck = EditorDeck {
            config: EditorConfig {
                title: None,
                ..default_config()
            },
            slides: vec![EditorSlide::default()],
        };
        let md = serialize_deck(&deck);
        assert!(!md.contains("title:"));
    }
}
