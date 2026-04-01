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
