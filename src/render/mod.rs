pub mod export;
pub mod theme;

use std::sync::Mutex;

use anyhow::Result;
use minijinja::{Environment, context};

use crate::layout::solver::{self, OverflowResult};
use crate::parser::Deck;

/// Last-emitted overflow signature (slide index + rounded pct) so that
/// repeated renders of an unchanged deck don't spam the log. `render_deck`
/// is called on every file save via the watcher, and the editor autosaves
/// frequently — without this dedupe, every keystroke would re-emit the
/// same warning.
static LAST_OVERFLOW_SIG: Mutex<Option<Vec<(usize, u32)>>> = Mutex::new(None);

#[allow(dead_code)]
pub struct RenderedDeck {
    /// Public HTML served at `/` — hidden slides are filtered out.
    pub html: String,
    /// Editor-preview HTML — all slides included, hidden ones marked with
    /// the `slide-hidden` class and `data-hidden="true"`.
    pub editor_html: String,
    pub overflows: Vec<OverflowResult>,
}

/// Render a single slide's `<section>…</section>` markup.
///
/// Used by the editor's live-preview path to patch just the active slide in
/// place inside the preview iframe, without reloading the whole document.
/// The markup produced here must stay structurally identical to the per-slide
/// block in `templates/deck.html.j2` so in-place replacement is seamless.
pub fn render_slide_html(deck: &Deck, index: usize) -> Result<String> {
    let slide = deck
        .slides
        .get(index)
        .ok_or_else(|| anyhow::anyhow!("Slide index {} out of range", index))?;

    let transition = slide
        .attrs
        .transition
        .as_deref()
        .unwrap_or(&deck.config.transition);
    let classes = slide_classes(slide);
    let notes: Vec<String> = slide.speaker_notes.iter().map(|n| n.text.clone()).collect();
    let style = slide_style(slide);
    let hidden = slide.attrs.hidden == Some(true);

    let template_str = r#"<section class="slide {{ classes }}" data-index="{{ index }}" data-transition="{{ transition }}"{% if hidden %} data-hidden="true"{% endif %}{% if style %} style="{{ style }}"{% endif %}{% if notes %} data-notes="{{ notes | join('\n') }}"{% endif %}>
{{ html | safe }}
</section>"#;

    let mut env = Environment::new();
    env.add_template("slide.html.j2", template_str)?;
    let tmpl = env.get_template("slide.html.j2")?;
    let html = tmpl.render(context! {
        index => index,
        html => slide.html.as_str(),
        transition => transition,
        classes => classes,
        notes => notes,
        style => style,
        hidden => hidden,
    })?;

    Ok(html)
}

/// Compose the space-separated class list for a slide's `<section>`, merging
/// per-slide `class:` attribute with any synthesized markers (e.g. `slide-hidden`).
fn slide_classes(slide: &crate::parser::Slide) -> String {
    let mut parts: Vec<&str> = Vec::new();
    if let Some(c) = slide.attrs.class.as_deref()
        && !c.is_empty()
    {
        parts.push(c);
    }
    if slide.attrs.hidden == Some(true) {
        parts.push("slide-hidden");
    }
    parts.join(" ")
}

/// Build an inline `style="…"` value from per-slide size overrides, or empty
/// if no overrides are set. Emitting as CSS custom properties on the section
/// lets the default `:root` values inherit normally while per-slide values
/// take precedence via the cascade.
fn slide_style(slide: &crate::parser::Slide) -> String {
    let mut parts: Vec<String> = Vec::new();
    if let Some(v) = slide.attrs.title_size.as_deref() {
        parts.push(format!("--title-size: {}", v));
    }
    if let Some(v) = slide.attrs.body_size.as_deref() {
        parts.push(format!("--body-size: {}", v));
    }
    parts.join("; ")
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

    let html = render_with(
        &tmpl,
        deck,
        core_css,
        &theme_css,
        slides_js,
        /* include_hidden */ false,
    )?;
    let editor_html = render_with(
        &tmpl,
        deck,
        core_css,
        &theme_css,
        slides_js,
        /* include_hidden */ true,
    )?;

    // Check for overflow across the whole deck (hidden slides included, since
    // a surprise un-hide shouldn't suddenly reveal an overflow the author
    // never saw).
    let slides_html: Vec<String> = deck.slides.iter().map(|s| s.html.clone()).collect();
    let overflows = solver::check_overflow(&slides_html, &deck.config.aspect);

    let sig: Vec<(usize, u32)> = overflows
        .iter()
        .map(|o| (o.slide_index, o.overflow_pct as u32))
        .collect();
    let mut last = LAST_OVERFLOW_SIG.lock().unwrap();
    if last.as_ref() != Some(&sig) {
        for o in &overflows {
            tracing::warn!(
                "Slide {}: content overflows by ~{}%",
                o.slide_index + 1,
                o.overflow_pct
            );
        }
        *last = Some(sig);
    }

    Ok(RenderedDeck {
        html,
        editor_html,
        overflows,
    })
}

fn render_with(
    tmpl: &minijinja::Template<'_, '_>,
    deck: &Deck,
    core_css: &str,
    theme_css: &str,
    slides_js: &str,
    include_hidden: bool,
) -> Result<String> {
    let slides_data: Vec<minijinja::Value> = deck
        .slides
        .iter()
        .filter(|s| include_hidden || s.attrs.hidden != Some(true))
        .enumerate()
        .map(|(i, slide)| {
            let transition = slide
                .attrs
                .transition
                .as_deref()
                .unwrap_or(&deck.config.transition);
            let classes = slide_classes(slide);
            let notes: Vec<String> = slide.speaker_notes.iter().map(|n| n.text.clone()).collect();
            let style = slide_style(slide);
            let hidden = slide.attrs.hidden == Some(true);

            context! {
                index => i,
                html => slide.html,
                transition => transition,
                classes => classes,
                notes => notes,
                style => style,
                hidden => hidden,
            }
        })
        .collect();

    let title = deck.config.title.as_deref().unwrap_or("Slides");
    let aspect = deck.config.aspect.class_name();
    let color_scheme = &deck.config.color_scheme;

    let title_size = &deck.config.title_size;
    let body_size = &deck.config.body_size;

    let html = tmpl.render(context! {
        title => title,
        aspect => aspect,
        color_scheme => color_scheme,
        title_size => title_size,
        body_size => body_size,
        core_css => core_css,
        theme_css => theme_css,
        slides_js => slides_js,
        slides => slides_data,
    })?;

    Ok(html)
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
    fn test_render_slide_html_single_slide() {
        let deck = parser::parse("# First\n\n---\n\n# Second\n").unwrap();
        let html = render_slide_html(&deck, 1).unwrap();
        assert!(html.starts_with("<section class=\"slide"));
        assert!(html.contains("data-index=\"1\""));
        assert!(html.contains("<h1>Second</h1>"));
        assert!(!html.contains("First"), "should only contain the requested slide");
    }

    #[test]
    fn test_render_slide_html_out_of_range_errors() {
        let deck = parser::parse("# Only\n").unwrap();
        assert!(render_slide_html(&deck, 5).is_err());
    }

    #[test]
    fn test_render_slide_html_matches_deck_section() {
        // The inner slide markup should be byte-identical to what render_deck
        // produces for the same slide (modulo surrounding whitespace).
        let input = "# A\n\n--- {transition: fade, class: centered}\n\n# B\n";
        let deck = parser::parse(input).unwrap();
        let full = render_deck(&deck).unwrap().html;
        let piece = render_slide_html(&deck, 1).unwrap();
        let first_line = piece.lines().next().unwrap();
        assert!(
            full.contains(first_line),
            "deck html should contain the opening <section> produced by render_slide_html:\n{}",
            first_line
        );
    }

    #[test]
    fn test_render_per_slide_title_size() {
        let input = "# A\n\n--- {title_size: 96px}\n\n# B\n";
        let deck = parser::parse(input).unwrap();
        let rendered = render_deck(&deck).unwrap();
        assert!(
            rendered.html.contains("style=\"--title-size: 96px\""),
            "expected inline style=\"--title-size: 96px\" on slide 2: {}",
            rendered.html
        );
    }

    #[test]
    fn test_render_per_slide_both_sizes() {
        let input = "# A\n\n--- {title_size: 40px, body_size: 20px}\n\n# B\n";
        let deck = parser::parse(input).unwrap();
        let rendered = render_deck(&deck).unwrap();
        assert!(rendered
            .html
            .contains("style=\"--title-size: 40px; --body-size: 20px\""));
    }

    #[test]
    fn test_render_no_style_when_no_overrides() {
        let input = "# A\n\n---\n\n# B\n";
        let deck = parser::parse(input).unwrap();
        let rendered = render_deck(&deck).unwrap();
        assert!(
            !rendered.html.contains("style=\""),
            "no slide should have an inline style when no overrides are set"
        );
    }

    #[test]
    fn test_render_slide_html_emits_style() {
        let input = "# A\n\n--- {body_size: 20px}\n\n# B\n";
        let deck = parser::parse(input).unwrap();
        let html = render_slide_html(&deck, 1).unwrap();
        assert!(html.contains("style=\"--body-size: 20px\""));
    }

    #[test]
    fn test_render_hidden_slide_absent_from_public_html() {
        let input = "# A\n\n--- {hidden: true}\n\n# Draft\n\n---\n\n# C\n";
        let deck = parser::parse(input).unwrap();
        let rendered = render_deck(&deck).unwrap();
        assert!(rendered.html.contains("<h1>A</h1>"));
        assert!(rendered.html.contains("<h1>C</h1>"));
        assert!(
            !rendered.html.contains("<h1>Draft</h1>"),
            "hidden slide must not appear in public HTML: {}",
            rendered.html
        );
        let public_count = rendered.html.matches("<section class=\"slide").count();
        assert_eq!(public_count, 2, "public HTML should contain 2 slides");
    }

    #[test]
    fn test_render_hidden_slide_present_in_editor_html() {
        let input = "# A\n\n--- {hidden: true}\n\n# Draft\n\n---\n\n# C\n";
        let deck = parser::parse(input).unwrap();
        let rendered = render_deck(&deck).unwrap();
        assert!(rendered.editor_html.contains("<h1>Draft</h1>"));
        assert!(
            rendered.editor_html.contains("data-hidden=\"true\""),
            "editor HTML should mark hidden slide: {}",
            rendered.editor_html
        );
        assert!(
            rendered.editor_html.contains("slide-hidden"),
            "editor HTML should include slide-hidden class"
        );
        let editor_count = rendered
            .editor_html
            .matches("<section class=\"slide")
            .count();
        assert_eq!(editor_count, 3, "editor HTML should contain 3 slides");
    }

    #[test]
    fn test_render_public_indices_compact_when_hidden() {
        // With a hidden middle slide, public-view indices should be 0, 1
        // (compacted) — not 0, 2.
        let input = "# A\n\n--- {hidden: true}\n\n# Draft\n\n---\n\n# C\n";
        let deck = parser::parse(input).unwrap();
        let rendered = render_deck(&deck).unwrap();
        assert!(rendered.html.contains("data-index=\"0\""));
        assert!(rendered.html.contains("data-index=\"1\""));
        assert!(!rendered.html.contains("data-index=\"2\""));
    }

    #[test]
    fn test_render_no_hidden_slides_identical_variants_content() {
        // When nothing is hidden, both variants should reference the same
        // three slides (they may differ only in inconsequential whitespace).
        let input = "# A\n\n---\n\n# B\n\n---\n\n# C\n";
        let deck = parser::parse(input).unwrap();
        let rendered = render_deck(&deck).unwrap();
        let public_count = rendered.html.matches("<section class=\"slide").count();
        let editor_count = rendered
            .editor_html
            .matches("<section class=\"slide")
            .count();
        assert_eq!(public_count, editor_count);
        assert_eq!(public_count, 3);
    }

    #[test]
    fn test_render_all_hidden_produces_empty_public_deck() {
        // Frontmatter block guards the leading separator from being swallowed
        // as a YAML open.
        let input = "---\ntitle: T\n---\n\n--- {hidden: true}\n\n# A\n\n--- {hidden: true}\n\n# B\n";
        let deck = parser::parse(input).unwrap();
        assert_eq!(deck.slides.len(), 2, "expected 2 parsed slides");
        assert_eq!(deck.slides[0].attrs.hidden, Some(true));
        assert_eq!(deck.slides[1].attrs.hidden, Some(true));
        let rendered = render_deck(&deck).unwrap();
        assert!(
            !rendered.html.contains("<section class=\"slide"),
            "public HTML should be empty of slides when all are hidden"
        );
        assert!(rendered.editor_html.contains("<h1>A</h1>"));
        assert!(rendered.editor_html.contains("<h1>B</h1>"));
    }

    #[test]
    fn test_render_slide_html_marks_hidden() {
        let input = "# A\n\n--- {hidden: true}\n\n# Draft\n";
        let deck = parser::parse(input).unwrap();
        let html = render_slide_html(&deck, 1).unwrap();
        assert!(html.contains("data-hidden=\"true\""));
        assert!(html.contains("slide-hidden"));
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
