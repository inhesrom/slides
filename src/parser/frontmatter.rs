use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct DeckConfig {
    pub title: Option<String>,
    pub theme: String,
    pub aspect: AspectRatio,
    pub transition: String,
    pub highlight_theme: String,
    pub color_scheme: String,
    pub auto_fit: String,
    pub export_images: String,
    pub title_size: String,
    pub body_size: String,
}

impl Default for DeckConfig {
    fn default() -> Self {
        Self {
            title: None,
            theme: "minimal".to_string(),
            aspect: AspectRatio::Wide,
            transition: "slide".to_string(),
            highlight_theme: "github".to_string(),
            color_scheme: "light".to_string(),
            auto_fit: "warn".to_string(),
            export_images: "relative".to_string(),
            title_size: "67px".to_string(),
            body_size: "28px".to_string(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub enum AspectRatio {
    #[serde(rename = "16:9")]
    Wide,
    #[serde(rename = "4:3")]
    Standard,
}

impl Default for AspectRatio {
    fn default() -> Self {
        AspectRatio::Wide
    }
}

impl AspectRatio {
    pub fn class_name(&self) -> &'static str {
        match self {
            AspectRatio::Wide => "16-9",
            AspectRatio::Standard => "4-3",
        }
    }
}

/// Extract YAML frontmatter from the beginning of a markdown file.
/// Returns (config, remaining_body).
pub fn extract(input: &str) -> Result<(DeckConfig, String)> {
    let trimmed = input.trim_start();

    if !trimmed.starts_with("---") {
        return Ok((DeckConfig::default(), input.to_string()));
    }

    // Find the closing ---
    let after_open = &trimmed[3..];
    let after_open = after_open.trim_start_matches('-');

    if let Some(newline_pos) = after_open.find('\n') {
        let rest = &after_open[newline_pos + 1..];
        if let Some(close_pos) = rest.find("\n---") {
            let yaml_str = &rest[..close_pos];
            let body = &rest[close_pos + 4..];
            // Skip past any remaining hyphens and the newline
            let body = body.trim_start_matches('-');
            let body = body.strip_prefix('\n').unwrap_or(body);

            let config: DeckConfig =
                serde_yaml::from_str(yaml_str).context("Failed to parse frontmatter YAML")?;

            return Ok((config, body.to_string()));
        }
    }

    // No closing delimiter found, treat entire input as body
    Ok((DeckConfig::default(), input.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_frontmatter() {
        let (config, body) = extract("# Hello\n\nWorld").unwrap();
        assert!(config.title.is_none());
        assert_eq!(config.theme, "minimal");
        assert!(body.contains("# Hello"));
    }

    #[test]
    fn test_basic_frontmatter() {
        let input = "---\ntitle: My Deck\ntheme: dark\n---\n\n# Slide 1";
        let (config, body) = extract(input).unwrap();
        assert_eq!(config.title.as_deref(), Some("My Deck"));
        assert_eq!(config.theme, "dark");
        assert!(body.contains("# Slide 1"));
        assert!(!body.contains("title:"));
    }

    #[test]
    fn test_frontmatter_defaults() {
        let input = "---\ntitle: Test\n---\n\nBody";
        let (config, _body) = extract(input).unwrap();
        assert_eq!(config.transition, "slide");
        assert_eq!(config.color_scheme, "light");
        assert_eq!(config.auto_fit, "warn");
    }

    #[test]
    fn test_aspect_ratio_wide() {
        let input = "---\naspect: \"16:9\"\n---\n\nBody";
        let (config, _) = extract(input).unwrap();
        assert_eq!(config.aspect.class_name(), "16-9");
    }

    #[test]
    fn test_aspect_ratio_standard() {
        let input = "---\naspect: \"4:3\"\n---\n\nBody";
        let (config, _) = extract(input).unwrap();
        assert_eq!(config.aspect.class_name(), "4-3");
    }

    #[test]
    fn test_full_frontmatter() {
        let input = "---\ntitle: Full\ntheme: dark\naspect: \"4:3\"\ntransition: fade\nhighlight_theme: monokai\ncolor_scheme: dark\nauto_fit: shrink\nexport_images: inline\n---\n\nContent";
        let (config, body) = extract(input).unwrap();
        assert_eq!(config.title.as_deref(), Some("Full"));
        assert_eq!(config.theme, "dark");
        assert_eq!(config.transition, "fade");
        assert_eq!(config.highlight_theme, "monokai");
        assert_eq!(config.color_scheme, "dark");
        assert_eq!(config.auto_fit, "shrink");
        assert_eq!(config.export_images, "inline");
        assert!(body.contains("Content"));
    }

    #[test]
    fn test_empty_input() {
        let (config, body) = extract("").unwrap();
        assert!(config.title.is_none());
        assert_eq!(body, "");
    }

    #[test]
    fn test_unclosed_frontmatter() {
        let input = "---\ntitle: Broken\n\n# No closing delimiter";
        let (config, body) = extract(input).unwrap();
        // Should treat entire input as body
        assert!(config.title.is_none());
        assert!(body.contains("---"));
    }

    #[test]
    fn test_invalid_yaml() {
        let input = "---\n[invalid yaml\n---\n\nBody";
        assert!(extract(input).is_err());
    }
}
