use serde::{Deserialize, Serialize};

use crate::parser::frontmatter::{AspectRatio, DeckConfig};

/// Editor-side representation of a full presentation deck.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorDeck {
    pub config: EditorConfig,
    pub slides: Vec<EditorSlide>,
}

/// Editor-side deck configuration (all string fields for easy JSON serialization).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorConfig {
    pub title: Option<String>,
    pub theme: String,
    pub aspect: String,
    pub transition: String,
    pub highlight_theme: String,
    pub color_scheme: String,
    pub auto_fit: String,
    pub export_images: String,
    pub title_size: String,
    pub body_size: String,
}

impl From<&DeckConfig> for EditorConfig {
    fn from(config: &DeckConfig) -> Self {
        Self {
            title: config.title.clone(),
            theme: config.theme.clone(),
            aspect: match config.aspect {
                AspectRatio::Wide => "16:9".to_string(),
                AspectRatio::Standard => "4:3".to_string(),
            },
            transition: config.transition.clone(),
            highlight_theme: config.highlight_theme.clone(),
            color_scheme: config.color_scheme.clone(),
            auto_fit: config.auto_fit.clone(),
            export_images: config.export_images.clone(),
            title_size: config.title_size.clone(),
            body_size: config.body_size.clone(),
        }
    }
}

/// Editor-side representation of a single slide.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorSlide {
    /// Raw markdown content (empty if layout is used).
    pub content: String,
    /// Per-slide transition override.
    pub transition: Option<String>,
    /// Per-slide CSS class.
    pub class: Option<String>,
    /// Speaker notes (block notes content, may contain markdown).
    pub notes: String,
    /// Layout directive with region content.
    pub layout: Option<EditorLayout>,
}

impl Default for EditorSlide {
    fn default() -> Self {
        Self {
            content: String::new(),
            transition: None,
            class: None,
            notes: String::new(),
            layout: None,
        }
    }
}

/// Editor-side layout with type, parameters, and region content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorLayout {
    /// Layout type: "split", "grid", "stack".
    pub kind: String,
    /// Layout parameters: "60/40", "2x2", "" etc.
    pub params: String,
    /// Raw markdown content for each region.
    pub regions: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_editor_config_from_deck_config() {
        let deck_config = DeckConfig {
            title: Some("Test".to_string()),
            theme: "dark".to_string(),
            aspect: AspectRatio::Standard,
            transition: "fade".to_string(),
            ..DeckConfig::default()
        };
        let editor_config = EditorConfig::from(&deck_config);
        assert_eq!(editor_config.title.as_deref(), Some("Test"));
        assert_eq!(editor_config.theme, "dark");
        assert_eq!(editor_config.aspect, "4:3");
        assert_eq!(editor_config.transition, "fade");
    }

    #[test]
    fn test_editor_config_from_default() {
        let editor_config = EditorConfig::from(&DeckConfig::default());
        assert!(editor_config.title.is_none());
        assert_eq!(editor_config.theme, "minimal");
        assert_eq!(editor_config.aspect, "16:9");
        assert_eq!(editor_config.transition, "slide");
    }

    #[test]
    fn test_editor_slide_default() {
        let slide = EditorSlide::default();
        assert!(slide.content.is_empty());
        assert!(slide.transition.is_none());
        assert!(slide.class.is_none());
        assert!(slide.notes.is_empty());
        assert!(slide.layout.is_none());
    }

    #[test]
    fn test_editor_deck_serialization() {
        let deck = EditorDeck {
            config: EditorConfig::from(&DeckConfig::default()),
            slides: vec![EditorSlide {
                content: "# Hello".to_string(),
                ..EditorSlide::default()
            }],
        };
        let json = serde_json::to_string(&deck).unwrap();
        let parsed: EditorDeck = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.slides[0].content, "# Hello");
    }
}
