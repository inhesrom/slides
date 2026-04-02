pub mod export;
pub mod theme;

use anyhow::Result;
use minijinja::{Environment, context};

use crate::layout::solver::{self, OverflowResult};
use crate::parser::Deck;

#[allow(dead_code)]
pub struct RenderedDeck {
    pub html: String,
    pub overflows: Vec<OverflowResult>,
}

/// Render a parsed Deck into a complete HTML document.
pub fn render_deck(deck: &Deck) -> Result<RenderedDeck> {
    let core_css = include_str!("../../static/css/core.css");
    let theme_css = theme::load_theme(&deck.config.theme);
    let slides_js = include_str!("../../static/js/slides.js");

    let template_str = include_str!("../../templates/deck.html.j2");

    let mut env = Environment::new();
    env.add_template("deck.html.j2", template_str)?;

    let tmpl = env.get_template("deck.html.j2")?;

    let slides_data: Vec<minijinja::Value> = deck
        .slides
        .iter()
        .enumerate()
        .map(|(i, slide)| {
            let transition = slide
                .attrs
                .transition
                .as_deref()
                .unwrap_or(&deck.config.transition);
            let classes = slide.attrs.class.as_deref().unwrap_or("");
            let notes: Vec<String> = slide.speaker_notes.iter().map(|n| n.text.clone()).collect();

            context! {
                index => i,
                html => slide.html,
                transition => transition,
                classes => classes,
                notes => notes,
            }
        })
        .collect();

    let title = deck
        .config
        .title
        .as_deref()
        .unwrap_or("Slides");
    let aspect = deck.config.aspect.class_name();
    let color_scheme = &deck.config.color_scheme;

    let html = tmpl.render(context! {
        title => title,
        aspect => aspect,
        color_scheme => color_scheme,
        core_css => core_css,
        theme_css => theme_css,
        slides_js => slides_js,
        slides => slides_data,
    })?;

    // Check for overflow
    let slides_html: Vec<String> = deck.slides.iter().map(|s| s.html.clone()).collect();
    let overflows = solver::check_overflow(&slides_html, &deck.config.aspect);

    for o in &overflows {
        tracing::warn!(
            "Slide {}: content overflows by ~{}%",
            o.slide_index + 1,
            o.overflow_pct
        );
    }

    Ok(RenderedDeck { html, overflows })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser;

    #[test]
    fn test_render_basic_deck() {
        let deck = parser::parse("# Hello\n\n---\n\n# World\n").unwrap();
        let rendered = render_deck(&deck).unwrap();
        assert!(rendered.html.contains("<!DOCTYPE html>"));
        assert!(rendered.html.contains("<title>Slides</title>"));
        assert!(rendered.html.contains("<section class=\"slide"));
        assert!(rendered.html.contains("<h1>Hello</h1>"));
        assert!(rendered.html.contains("<h1>World</h1>"));
    }

    #[test]
    fn test_render_with_title() {
        let deck = parser::parse("---\ntitle: My Talk\n---\n\n# Slide\n").unwrap();
        let rendered = render_deck(&deck).unwrap();
        assert!(rendered.html.contains("<title>My Talk</title>"));
    }

    #[test]
    fn test_render_aspect_ratio() {
        let deck = parser::parse("---\naspect: \"4:3\"\n---\n\n# Slide\n").unwrap();
        let rendered = render_deck(&deck).unwrap();
        assert!(rendered.html.contains("aspect-4-3"));
    }

    #[test]
    fn test_render_dark_color_scheme() {
        let deck = parser::parse("---\ncolor_scheme: dark\n---\n\n# Slide\n").unwrap();
        let rendered = render_deck(&deck).unwrap();
        assert!(rendered.html.contains("class=\"dark\""));
    }

    #[test]
    fn test_render_contains_css_and_js() {
        let deck = parser::parse("# Slide\n").unwrap();
        let rendered = render_deck(&deck).unwrap();
        assert!(rendered.html.contains("<style>"));
        assert!(rendered.html.contains("<script>"));
        assert!(rendered.html.contains("showSlide"));
    }

    #[test]
    fn test_render_slide_transition() {
        let deck = parser::parse("A\n--- {transition: fade}\nB\n").unwrap();
        let rendered = render_deck(&deck).unwrap();
        assert!(rendered.html.contains("data-transition=\"fade\""));
    }

    #[test]
    fn test_render_speaker_notes() {
        let deck = parser::parse("Text ^[My note]\n").unwrap();
        let rendered = render_deck(&deck).unwrap();
        assert!(rendered.html.contains("data-notes=\"My note\""));
    }

    #[test]
    fn test_render_progress_bar() {
        let deck = parser::parse("# Slide\n").unwrap();
        let rendered = render_deck(&deck).unwrap();
        assert!(rendered.html.contains("id=\"progress-bar\""));
        assert!(rendered.html.contains("id=\"progress-fill\""));
    }

    #[test]
    fn test_render_slide_count() {
        let input = "A\n---\nB\n---\nC\n";
        let deck = parser::parse(input).unwrap();
        let rendered = render_deck(&deck).unwrap();
        let count = rendered.html.matches("<section class=\"slide").count();
        assert_eq!(count, 3);
    }

    #[test]
    fn test_render_centered_class() {
        let input = "# Normal\n\n--- {class: centered}\n\n# Centered Slide\n";
        let deck = parser::parse(input).unwrap();
        let rendered = render_deck(&deck).unwrap();
        // Second slide should have centered class
        assert!(
            rendered.html.contains("class=\"slide centered\""),
            "Rendered HTML should contain centered class: {}",
            rendered.html
        );
    }

    #[test]
    fn test_render_block_notes_in_data_attr() {
        let input = "# Title\n\n:::notes\nBlock speaker note\n:::\n";
        let deck = parser::parse(input).unwrap();
        let rendered = render_deck(&deck).unwrap();
        assert!(
            rendered.html.contains("data-notes=\"Block speaker note\""),
            "Block notes should appear in data-notes: {}",
            rendered.html
        );
    }

    #[test]
    fn test_render_notes_not_in_visible_content() {
        let input = "Visible text\n\n:::notes\nHidden note\n:::\n";
        let deck = parser::parse(input).unwrap();
        let rendered = render_deck(&deck).unwrap();
        // Notes should be in data-notes but not rendered as visible HTML
        assert!(rendered.html.contains("data-notes=\"Hidden note\""));
        // The note text should NOT appear in a <p> tag
        assert!(
            !rendered.html.contains("<p>Hidden note</p>"),
            "Notes should not be visible content"
        );
    }

    #[test]
    fn test_render_inline_notes_in_data_attr() {
        let input = "Some text ^[Inline note here] more text\n";
        let deck = parser::parse(input).unwrap();
        let rendered = render_deck(&deck).unwrap();
        assert!(
            rendered.html.contains("data-notes=\"Inline note here\""),
            "Inline notes should appear in data-notes: {}",
            rendered.html
        );
        // Inline note should be stripped from visible content
        assert!(
            !rendered.html.contains("^["),
            "Inline note syntax should be stripped"
        );
    }

    #[test]
    fn test_render_unchanged_html_is_equal() {
        // Rendering the same input twice should produce identical HTML
        // (important for watcher's content-comparison optimization)
        let input = "---\ntitle: Test\n---\n\n# Slide 1\n\n---\n\n# Slide 2\n";
        let deck1 = parser::parse(input).unwrap();
        let rendered1 = render_deck(&deck1).unwrap();
        let deck2 = parser::parse(input).unwrap();
        let rendered2 = render_deck(&deck2).unwrap();
        assert_eq!(rendered1.html, rendered2.html, "Same input should produce identical HTML");
    }

    #[test]
    fn test_render_different_content_produces_different_html() {
        let input1 = "# Slide A\n";
        let input2 = "# Slide B\n";
        let rendered1 = render_deck(&parser::parse(input1).unwrap()).unwrap();
        let rendered2 = render_deck(&parser::parse(input2).unwrap()).unwrap();
        assert_ne!(rendered1.html, rendered2.html, "Different input should produce different HTML");
    }
}
