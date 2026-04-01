pub mod export;
pub mod theme;

use anyhow::Result;
use minijinja::{Environment, context};

use crate::layout::solver::{self, OverflowResult};
use crate::parser::Deck;

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
}
