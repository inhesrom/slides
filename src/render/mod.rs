pub mod export;
pub mod theme;

use anyhow::Result;
use minijinja::{Environment, context};

use crate::parser::Deck;

pub struct RenderedDeck {
    pub html: String,
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

    Ok(RenderedDeck { html })
}
