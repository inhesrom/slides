use pulldown_cmark::{html, Event, Options, Parser, Tag};

/// Marker inserted for `+` list items so we can find them in the HTML output.
///
/// Uses an inline HTML `<span>` rather than a lone invisible character because
/// the marker sits directly before list-item content, and CommonMark's
/// emphasis flanking rules require a preceding whitespace or punctuation
/// character for `**_...` style nested emphasis to open. The marker's
/// trailing `>` is ASCII punctuation and satisfies the rule. A bare U+FEFF
/// is neither whitespace nor punctuation, so it broke `**_text_**`. An HTML
/// comment (`<!--f-->`) is worse: CommonMark treats it as a block-HTML start,
/// which swallows the rest of the line as raw HTML and prevents inline
/// markdown parsing entirely. A `<span>` tag is never block-level in
/// CommonMark's block-HTML rules, so pulldown-cmark keeps it as inline HTML
/// and parses surrounding emphasis normally.
const FRAGMENT_MARKER: &str = "<span data-f></span>";

/// The visual style of a lettered / roman-numeral ordered list.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum ListStyle {
    LowerAlpha,
    UpperAlpha,
    LowerRoman,
    UpperRoman,
    /// A plain numeric item that adjoins a lettered run: it needs an explicit
    /// `value` (the merged `<ol>` counter would otherwise continue from the
    /// lettered items) but no `list-style-type` class.
    Decimal,
}

impl ListStyle {
    /// Short code embedded in the marker span's `data-ls` attribute.
    fn code(self) -> &'static str {
        match self {
            ListStyle::LowerAlpha => "la",
            ListStyle::UpperAlpha => "ua",
            ListStyle::LowerRoman => "lr",
            ListStyle::UpperRoman => "ur",
            ListStyle::Decimal => "dec",
        }
    }

    /// CSS class that sets the matching `list-style-type` (see `core.css`).
    /// Empty for [`ListStyle::Decimal`], which only needs the `value`.
    fn class(self) -> &'static str {
        match self {
            ListStyle::LowerAlpha => "list-lower-alpha",
            ListStyle::UpperAlpha => "list-upper-alpha",
            ListStyle::LowerRoman => "list-lower-roman",
            ListStyle::UpperRoman => "list-upper-roman",
            ListStyle::Decimal => "",
        }
    }

    fn from_code(code: &str) -> Option<ListStyle> {
        match code {
            "la" => Some(ListStyle::LowerAlpha),
            "ua" => Some(ListStyle::UpperAlpha),
            "lr" => Some(ListStyle::LowerRoman),
            "ur" => Some(ListStyle::UpperRoman),
            "dec" => Some(ListStyle::Decimal),
            _ => None,
        }
    }

    /// Classify a single marker letter into a style and its starting ordinal.
    /// `i`/`I` mean roman (always start at 1, since only the single letter is
    /// recognized); every other letter is alphabetic and honors its position
    /// (`a`->1, `c`->3, `A`->1, `C`->3) so `c. ` starts a list at "c".
    fn from_letter(letter: char) -> (ListStyle, u32) {
        match letter {
            'i' => (ListStyle::LowerRoman, 1),
            'I' => (ListStyle::UpperRoman, 1),
            c if c.is_ascii_lowercase() => {
                (ListStyle::LowerAlpha, (c as u32) - ('a' as u32) + 1)
            }
            c => (ListStyle::UpperAlpha, (c as u32) - ('A' as u32) + 1),
        }
    }
}

/// Build the marker span for a lettered / roman list item, carrying both the
/// style and the item's explicit ordinal `value`.
///
/// Uses the same inline `<span>` form as [`FRAGMENT_MARKER`] for the same reason
/// (see its doc comment): it sits directly before the list-item content and its
/// trailing `>` satisfies CommonMark's emphasis flanking rules, so `**_text_**`
/// still parses. Post-processing ([`apply_list_styles`]) turns the span into a
/// `class="list-…"` plus a `value="N"` attribute on the owning `<li>`. The
/// explicit `value` makes each item render the correct letter even when
/// pulldown-cmark merges adjacent ordered lists into one `<ol>` (the CSS
/// `list-style-type` formats each item's own `value`, not its DOM position).
fn list_marker_span(style: ListStyle, value: u32) -> String {
    format!(
        "<span data-ls=\"{}\" data-v=\"{}\"></span>",
        style.code(),
        value
    )
}

/// Render markdown to HTML without any slide-specific transformations.
///
/// Enables the same pulldown-cmark extensions as the slide renderer (tables,
/// footnotes, strikethrough, task lists) but does NOT convert `+` list items
/// into fragment reveals or strip `{.class}` trailing annotations. Use this
/// for rendering prose documents like SYNTAX.md where those transformations
/// would mis-render example snippets.
pub fn render_plain(markdown: &str) -> String {
    let options = Options::ENABLE_TABLES
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS;

    let parser = Parser::new_ext(markdown, options);
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);
    html_output
}

/// Render markdown to HTML, applying semantic style annotations.
pub fn render(markdown: &str) -> String {
    let (cleaned, annotations) = extract_annotations(markdown);

    // Note: we don't enable ENABLE_HEADING_ATTRIBUTES because we handle
    // {.class} annotations ourselves in the pre-processing pass.
    let options = Options::ENABLE_TABLES
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS;

    let parser = Parser::new_ext(&cleaned, options);
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);

    let html_output = apply_annotations(&html_output, &annotations);
    let html_output = apply_li_marker(&html_output, FRAGMENT_MARKER, "fragment");
    apply_list_styles(&html_output)
}

/// Replace a marker span with a class on the `<li>` tag that owns it.
/// Handles both bare `<li>` and `<li class="existing">` cases, merging the
/// class in rather than overwriting. Used for fragment reveals and for
/// lettered / roman-numeral list styling.
fn apply_li_marker(html: &str, marker: &str, class: &str) -> String {
    let mut result = String::with_capacity(html.len());
    let mut remaining = html;

    while let Some(pos) = remaining.find(marker) {
        // Look backwards for the `<li` that owns this marker
        let before = &remaining[..pos];
        if let Some(li_start) = before.rfind("<li") {
            let tag_region = &remaining[li_start..pos];
            if let Some(class_pos) = tag_region.find("class=\"") {
                // Already has a class — prepend ours to it
                let abs_class = li_start + class_pos + 7; // after `class="`
                result.push_str(&remaining[..abs_class]);
                result.push_str(class);
                result.push(' ');
                result.push_str(&remaining[abs_class..pos]);
            } else if let Some(rel_close) = before[li_start..].find('>') {
                // No class yet — add one to the <li> tag
                let tag_close = rel_close + li_start;
                result.push_str(&remaining[..tag_close]);
                result.push_str(" class=\"");
                result.push_str(class);
                result.push('"');
                result.push_str(&remaining[tag_close..pos]);
            } else {
                // Malformed raw HTML (`<li` never closed) — leave untouched.
                result.push_str(before);
            }
        } else {
            result.push_str(before);
        }
        // Skip the marker span
        remaining = &remaining[pos + marker.len()..];
    }
    result.push_str(remaining);
    result
}

/// Read the quoted value of `key` (e.g. `data-v="`) starting from `s`.
fn quoted_attr<'a>(s: &'a str, key: &str) -> Option<&'a str> {
    let start = s.find(key)? + key.len();
    let rest = &s[start..];
    let end = rest.find('"')?;
    Some(&rest[..end])
}

/// Turn each lettered / roman marker span into `class="list-…"` + `value="N"`
/// on the owning `<li>`, then strip the span. The explicit `value` resets that
/// item's ordinal, so it renders the correct letter/numeral regardless of how
/// pulldown-cmark grouped items into `<ol>` elements.
///
/// Only spans that sit at an item's start (nothing but whitespace or a `<p>`
/// between the `<li…>` tag and the span — exactly where the pre-pass puts
/// them) are consumed; anything else is user-authored HTML and passes through
/// untouched.
fn apply_list_styles(html: &str) -> String {
    let needle = "<span data-ls=\"";
    let span_end = "></span>";
    let mut result = String::with_capacity(html.len());
    let mut remaining = html;

    while let Some(pos) = remaining.find(needle) {
        // Bound the span so attribute lookups stay inside it.
        let end = match remaining[pos..].find(span_end) {
            Some(e) => pos + e + span_end.len(),
            None => break, // malformed; leave the rest untouched
        };
        let span = &remaining[pos..end];
        let style = quoted_attr(span, "data-ls=\"").and_then(ListStyle::from_code);
        let value = quoted_attr(span, "data-v=\"").and_then(|v| v.parse::<u32>().ok());

        let before = &remaining[..pos];
        let stamped = match (before.rfind("<li"), style, value) {
            (Some(li_start), Some(style), Some(value)) => {
                match stamp_li(remaining, li_start, pos, style, value) {
                    Some(replacement) => {
                        result.push_str(&remaining[..li_start]);
                        result.push_str(&replacement);
                        true
                    }
                    None => false,
                }
            }
            _ => false,
        };
        if !stamped {
            // Not one of our generated spans — keep it verbatim.
            result.push_str(&remaining[..end]);
        }
        remaining = &remaining[end..];
    }
    result.push_str(remaining);
    result
}

/// Build the replacement for the region `[li_start, span_pos)` of `html`: the
/// reconstructed `<li value=… class=…>` tag plus the interstitial content.
/// Returns None when the region does not look like a generated item start
/// (malformed tag, or real content between the tag and the span).
fn stamp_li(
    html: &str,
    li_start: usize,
    span_pos: usize,
    style: ListStyle,
    value: u32,
) -> Option<String> {
    let tag_close = html[li_start..span_pos].find('>')? + li_start;
    let interstitial = &html[tag_close + 1..span_pos];
    if !interstitial.trim().is_empty() && interstitial.trim() != "<p>" {
        return None;
    }
    let tag = &html[li_start..tag_close]; // `<li` .. before `>`

    // Merge with any class already on the <li> (e.g. from a {.class}
    // annotation); Decimal has no class of its own.
    let existing = quoted_attr(tag, "class=\"");
    let classes = match (style.class(), existing) {
        ("", None) => None,
        ("", Some(e)) => Some(e.to_string()),
        (c, None) => Some(c.to_string()),
        (c, Some(e)) => Some(format!("{} {}", c, e)),
    };

    let mut out = format!("<li value=\"{}\"", value);
    if let Some(classes) = classes {
        out.push_str(&format!(" class=\"{}\"", classes));
    }
    out.push('>');
    out.push_str(interstitial);
    Some(out)
}

/// An annotation extracted from a markdown line.
struct Annotation {
    classes: Vec<String>,
}

/// A pending lettered/roman list "run" at a given indent: its style, indent
/// width, and the ordinal to assign to the next item.
type ListRun = (ListStyle, usize, u32);

/// Per-line block-structure facts derived from a preliminary pulldown-cmark
/// parse of the ORIGINAL text. Using the real parser's block detection (code
/// blocks, HTML blocks, headings, paragraph starts) means the line rewriting
/// below can never disagree with how CommonMark will actually parse the text —
/// hand-rolled fence/indent tracking systematically diverged in edge cases.
#[derive(Clone, Copy, Default)]
struct LineInfo {
    /// Inside a code block (fenced or indented) or an HTML block: the line is
    /// literal content and must be emitted verbatim, with no marker rewriting
    /// and no `{.class}` annotation stripping.
    verbatim: bool,
    /// Inside a heading (relevant for setext headings, whose first line would
    /// otherwise look like a lettered item): annotations still apply, list
    /// markers do not.
    no_marker: bool,
    /// A paragraph begins exactly at this line's first non-whitespace byte.
    /// This is the authoritative "block boundary" signal for lettered-marker
    /// recognition (and naturally excludes blockquote interiors, where the
    /// paragraph starts after the `> ` prefix).
    para_initial: bool,
}

/// Run the preliminary parse and compute [`LineInfo`] for every line.
fn block_info(markdown: &str) -> Vec<LineInfo> {
    // Byte offset of each line start (aligned with `markdown.lines()`).
    let mut line_starts = vec![0usize];
    for (i, b) in markdown.bytes().enumerate() {
        if b == b'\n' {
            line_starts.push(i + 1);
        }
    }
    let mut infos = vec![LineInfo::default(); line_starts.len()];
    let line_of = |offset: usize| line_starts.partition_point(|&s| s <= offset) - 1;

    let options = Options::ENABLE_TABLES
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS;

    for (event, range) in Parser::new_ext(markdown, options).into_offset_iter() {
        match event {
            // Start-event ranges span the whole element, fences included.
            Event::Start(Tag::CodeBlock(_)) | Event::Start(Tag::HtmlBlock) => {
                for info in &mut infos[line_of(range.start)..=line_of(range.end.saturating_sub(1))]
                {
                    info.verbatim = true;
                }
            }
            Event::Start(Tag::Heading { .. }) => {
                for info in &mut infos[line_of(range.start)..=line_of(range.end.saturating_sub(1))]
                {
                    info.no_marker = true;
                }
            }
            Event::Start(Tag::Paragraph) => {
                let line = line_of(range.start);
                let line_start = line_starts[line];
                let indent_len = markdown[line_start..range.start]
                    .bytes()
                    .take_while(|b| b.is_ascii_whitespace())
                    .count();
                if line_start + indent_len == range.start {
                    infos[line].para_initial = true;
                }
            }
            _ => {}
        }
    }
    infos
}

/// Pre-process markdown before CommonMark parsing:
/// - extract `{.class}` trailing annotations,
/// - convert `+`/`<digits>+` markers into fragment list items,
/// - convert single-letter markers (`a.`/`A.`/`i.`/`I.`) into ordered items
///   carrying a style + explicit ordinal (see [`list_marker_span`]).
///
/// Block structure (what is code, what is a heading, where paragraphs start)
/// comes from [`block_info`]'s preliminary parse; this loop only tracks the
/// lettered-run state needed to assign ordinals.
fn extract_annotations(markdown: &str) -> (String, Vec<Annotation>) {
    let infos = block_info(markdown);
    let mut cleaned = String::new();
    let mut annotations = Vec::new();

    // Was the previous line a list item (bullet, numeric, fragment, lettered)?
    // A lettered marker directly below one is an item even mid-paragraph.
    let mut prev_list_item = false;
    // Was the previous line blank? Distinguishes a deliberately new list from a
    // loose (blank-separated) continuation of the current one.
    let mut prev_blank = false;
    // Stack of active lettered runs, ordered by increasing indent.
    let mut runs: Vec<ListRun> = Vec::new();

    for (idx, line) in markdown.lines().enumerate() {
        let info = infos.get(idx).copied().unwrap_or_default();
        let trimmed = line.trim_end();
        let body = trimmed.trim_start();
        let indent = &trimmed[..trimmed.len() - body.len()];
        let indent_len = indent.len();

        // Code / HTML block content: emit the line untouched (trailing
        // whitespace can be significant in code).
        if info.verbatim {
            cleaned.push_str(line);
            cleaned.push('\n');
            // A block shallower than a run's content column interrupts the
            // run; an indented one (e.g. a fence inside the item) does not.
            clear_runs_interrupted_at(&mut runs, indent_len);
            prev_list_item = false;
            prev_blank = false;
            continue;
        }

        if body.is_empty() {
            cleaned.push('\n');
            prev_list_item = false;
            prev_blank = true;
            continue;
        }

        let mut is_list_item = false;
        let working_line = if let Some(rest) = strip_plus_marker(trimmed) {
            is_list_item = true;
            format!("{}- {}{}", indent, FRAGMENT_MARKER, rest)
        } else if let Some((digits, rest)) = strip_ordered_plus_marker(trimmed) {
            is_list_item = true;
            format!("{}{}. {}{}", indent, digits, FRAGMENT_MARKER, rest)
        } else if !info.no_marker
            && is_alpha_list_marker(body)
            && (info.para_initial || prev_list_item || !runs.is_empty())
        {
            is_list_item = true;
            let (letter, reveal, rest) = strip_alpha_marker(body).unwrap();
            let (item_style, start) = ListStyle::from_letter(letter);
            let (style, ordinal) =
                resolve_run(&mut runs, item_style, start, indent_len, prev_blank);
            // Lettered items are fragment reveals by default (`a. ` / `a+ `);
            // the `a) ` form opts out.
            let frag = if reveal { FRAGMENT_MARKER } else { "" };
            format!("{}1. {}{}{}", indent, frag, list_marker_span(style, ordinal), rest)
        } else if let Some((digits, rest)) = strip_numeric_dot_marker(body) {
            is_list_item = true;
            if run_at_same_level(&runs, indent_len) {
                // A numeric item adjoining a lettered run merges into the same
                // <ol>, whose counter sits after the lettered values. Stamp the
                // typed number so it renders as written; the browser counts on
                // from it for any following plain numeric items.
                clear_runs_interrupted_at(&mut runs, indent_len);
                let value = digits.parse::<u32>().unwrap_or(1);
                format!(
                    "{}{}. {}{}",
                    indent,
                    digits,
                    list_marker_span(ListStyle::Decimal, value),
                    rest
                )
            } else {
                trimmed.to_string()
            }
        } else {
            if is_bullet_or_numeric_line(body) {
                is_list_item = true;
                // A bullet (or `N)`) list at the run's level starts a separate
                // HTML list, so the run is over.
                clear_runs_interrupted_at(&mut runs, indent_len);
            } else {
                // Prose, headings, tables, …: a genuinely new block at the
                // run's level ends it. A line that is NOT a paragraph start is
                // a (lazy or indented) continuation of the current item and
                // leaves the run alive.
                let lazy_continuation = !info.para_initial && !info.no_marker;
                if !lazy_continuation {
                    clear_runs_interrupted_at(&mut runs, indent_len);
                }
            }
            trimmed.to_string()
        };

        if let Some((before, classes)) = parse_trailing_annotation(&working_line) {
            if !classes.is_empty() {
                annotations.push(Annotation { classes });
                cleaned.push_str(before);
                cleaned.push('\n');
                prev_list_item = is_list_item;
                prev_blank = false;
                continue;
            }
        }

        cleaned.push_str(&working_line);
        cleaned.push('\n');
        prev_list_item = is_list_item;
        prev_blank = false;
    }

    (cleaned, annotations)
}

/// Is `body` (left-trimmed) a `-`/`*`/`+` bullet, a `N)` item, or a `N+`
/// fragment? (`N. ` and the single-letter shape are handled by dedicated
/// branches before this is consulted.)
fn is_bullet_or_numeric_line(body: &str) -> bool {
    if body.starts_with("- ") || body.starts_with("* ") || body.starts_with("+ ") {
        return true;
    }
    let digits = body.bytes().take_while(|b| b.is_ascii_digit()).count();
    if digits > 0 {
        let rest = &body[digits..];
        return rest.starts_with(". ") || rest.starts_with(") ") || rest.starts_with("+ ");
    }
    false
}

/// If the line body is a numeric `N. ` ordered item, return the digits and the
/// content after the marker.
fn strip_numeric_dot_marker(body: &str) -> Option<(&str, &str)> {
    let digit_end = body.bytes().take_while(|b| b.is_ascii_digit()).count();
    if digit_end == 0 {
        return None;
    }
    let rest = body[digit_end..].strip_prefix(". ")?;
    Some((&body[..digit_end], rest))
}

/// A line is a lettered/roman list item only if it strips to a single-letter
/// marker AND the remaining text is not itself another initial (`J. R. R. …`),
/// which almost always means the line is a name, not a list.
fn is_alpha_list_marker(body: &str) -> bool {
    match strip_alpha_marker(body) {
        Some((_, _, rest)) => strip_alpha_marker(rest).is_none(),
        None => false,
    }
}

/// CommonMark needs an item indented to at least its parent's content column
/// (marker + ". " = 3) to nest; anything closer is the same list. Two runs are
/// "the same level" when their indents differ by less than this.
const NEST_INDENT: usize = 3;

/// Is the top run at the same level as a line at `indent_len`?
fn run_at_same_level(runs: &[ListRun], indent_len: usize) -> bool {
    matches!(runs.last(), Some(&(_, ind, _)) if ind.abs_diff(indent_len) < NEST_INDENT)
}

/// Assign a lettered item its (style, ordinal), maintaining per-level runs.
/// Continues the run at this level (keeping the run's established style) unless
/// a blank line preceded a differently-styled marker, which starts a new list.
fn resolve_run(
    runs: &mut Vec<ListRun>,
    item_style: ListStyle,
    start_ordinal: u32,
    indent_len: usize,
    prev_blank: bool,
) -> (ListStyle, u32) {
    // Drop runs nested deeper than this line (we have outdented past them).
    while matches!(runs.last(), Some(&(_, ind, _)) if ind >= indent_len + NEST_INDENT) {
        runs.pop();
    }
    let continue_top = match runs.last() {
        Some(&(run_style, ind, next)) if ind.abs_diff(indent_len) < NEST_INDENT => {
            // An `i.`/`I.` marker classifies as roman, but when the same-case
            // ALPHA run has counted up to exactly its 9th item, the letter is
            // the alphabetic `i` — continue the alpha run.
            let alpha_i = next == 9
                && matches!(
                    (item_style, run_style),
                    (ListStyle::LowerRoman, ListStyle::LowerAlpha)
                        | (ListStyle::UpperRoman, ListStyle::UpperAlpha)
                );
            run_style == item_style || !prev_blank || alpha_i
        }
        _ => false,
    };
    if continue_top {
        let top = runs.last_mut().unwrap();
        let ordinal = top.2;
        top.2 += 1;
        return (top.0, ordinal);
    }
    // Start a new run at this level, replacing any run already at it.
    if run_at_same_level(runs, indent_len) {
        runs.pop();
    }
    runs.push((item_style, indent_len, start_ordinal + 1));
    (item_style, start_ordinal)
}

/// Drop runs interrupted by non-run content at `indent_len`: any run whose
/// content column (marker indent + [`NEST_INDENT`]) lies beyond the line's
/// indent is over; content indented past it belongs to the run's current item
/// and leaves it alive.
fn clear_runs_interrupted_at(runs: &mut Vec<ListRun>, indent_len: usize) {
    while matches!(runs.last(), Some(&(_, ind, _)) if ind + NEST_INDENT > indent_len) {
        runs.pop();
    }
}

/// If the line is a `+ ` list item, return the content after the marker.
fn strip_plus_marker(line: &str) -> Option<&str> {
    let trimmed = line.trim_start();
    trimmed.strip_prefix("+ ")
}

/// If the line is a `<digits>+ ` list item (ordered-list fragment),
/// return the digit prefix and the content after the marker.
fn strip_ordered_plus_marker(line: &str) -> Option<(&str, &str)> {
    let trimmed = line.trim_start();
    let digit_end = trimmed.bytes().take_while(|b| b.is_ascii_digit()).count();
    if digit_end == 0 {
        return None;
    }
    let rest = trimmed[digit_end..].strip_prefix("+ ")?;
    Some((&trimmed[..digit_end], rest))
}

/// If the line is a single-letter ordered-list marker, return the marker
/// letter, whether the item is a fragment reveal, and the content after it.
/// Three delimiters are accepted after the letter:
/// - `". "` — reveal (the default form, `a. `)
/// - `"+ "` — reveal (explicit alias, mirroring `1+`)
/// - `") "` — always visible (static)
///
/// Requires exactly one leading ASCII letter followed by the delimiter, so
/// multi-character prefixes (`aa. `), abbreviations (`e.g. `), and numeric
/// markers (`1. `) do not match.
fn strip_alpha_marker(line: &str) -> Option<(char, bool, &str)> {
    let trimmed = line.trim_start();
    let first = trimmed.chars().next()?;
    if !first.is_ascii_alphabetic() {
        return None;
    }
    // The leading letter is one ASCII byte, so `trimmed[1..]` is a valid slice.
    let after = &trimmed[1..];
    if let Some(rest) = after.strip_prefix(". ").or_else(|| after.strip_prefix("+ ")) {
        Some((first, true, rest))
    } else if let Some(rest) = after.strip_prefix(") ") {
        Some((first, false, rest))
    } else {
        None
    }
}

/// Parse a trailing `{.class1 .class2}` from a line.
/// Returns (line_without_annotation, classes) or None if no annotation found.
fn parse_trailing_annotation(line: &str) -> Option<(&str, Vec<String>)> {
    let close = line.rfind('}')?;
    let open = line[..close].rfind('{')?;
    let inner = &line[open + 1..close];

    // Must start with a dot-class
    if !inner.trim_start().starts_with('.') {
        return None;
    }

    let classes: Vec<String> = inner
        .split_whitespace()
        .filter_map(|s| s.strip_prefix('.').map(|c| c.to_string()))
        .collect();

    if classes.is_empty() {
        return None;
    }

    let before = line[..open].trim_end();
    Some((before, classes))
}

/// Apply extracted annotations to rendered HTML by wrapping annotated blocks
/// in `<div class="...">` wrappers.
fn apply_annotations(html: &str, annotations: &[Annotation]) -> String {
    if annotations.is_empty() {
        return html.to_string();
    }

    let mut result = html.to_string();

    // For each annotation, find the HTML block element that corresponds to
    // the annotated line and add classes to it.
    // Strategy: work on the HTML line by line, matching annotations by content proximity.
    for annotation in annotations {
        let class_attr = annotation.classes.join(" ");
        result = add_class_to_block(&result, &class_attr);
    }

    result
}

/// Find the last block-level opening tag without a class and add the given classes.
fn add_class_to_block(html: &str, classes: &str) -> String {
    // Find block tags that don't have a class attribute yet
    let block_tags = [
        "<p>", "<blockquote>\n<p>", "<li>",
        "<h1>", "<h2>", "<h3>", "<h4>", "<h5>", "<h6>",
    ];

    // Try to find the last unclassed block tag
    for tag in &block_tags {
        if let Some(pos) = html.rfind(tag) {
            let target = if *tag == "<blockquote>\n<p>" {
                "<blockquote>"
            } else {
                *tag
            };

            let actual_pos = html.rfind(target).unwrap_or(pos);

            let mut result = String::with_capacity(html.len() + classes.len() + 10);
            let tag_no_close = target.trim_end_matches('>');
            result.push_str(&html[..actual_pos]);
            result.push_str(tag_no_close);
            result.push_str(&format!(" class=\"{}\">", classes));
            result.push_str(&html[actual_pos + target.len()..]);
            return result;
        }
    }

    html.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_markdown() {
        let result = render("# Hello\n\nWorld");
        assert!(result.contains("<h1>Hello</h1>"));
        assert!(result.contains("<p>World</p>"));
    }

    #[test]
    fn test_code_block() {
        let result = render("```rust\nfn main() {}\n```");
        assert!(result.contains("<code class=\"language-rust\">"));
    }

    #[test]
    fn test_semantic_emphasis_class() {
        let result = render("## Important Point {.emphasis}");
        assert!(result.contains("emphasis"), "Got: {}", result);
        assert!(!result.contains("{.emphasis}"), "Annotation not stripped: {}", result);
    }

    #[test]
    fn test_semantic_paragraph_class() {
        let result = render("Some text {.aside}");
        assert!(result.contains("class=\"aside\""), "Got: {}", result);
        assert!(!result.contains("{.aside}"), "Annotation not stripped: {}", result);
    }

    #[test]
    fn test_semantic_multiple_classes() {
        let result = render("> Quote text {.callout .warning}");
        assert!(result.contains("callout"), "Got: {}", result);
        assert!(result.contains("warning"), "Got: {}", result);
    }

    #[test]
    fn test_no_annotation() {
        let result = render("Just a normal paragraph");
        assert!(result.contains("<p>Just a normal paragraph</p>"));
    }

    #[test]
    fn test_tables() {
        let result = render("| A | B |\n|---|---|\n| 1 | 2 |");
        assert!(result.contains("<table>"));
    }

    #[test]
    fn test_lists() {
        let result = render("- item 1\n- item 2");
        assert!(result.contains("<li>item 1</li>"));
    }

    #[test]
    fn test_fragment_class_legacy() {
        let result = render("- item {.fragment}");
        assert!(result.contains("fragment"), "Got: {}", result);
    }

    #[test]
    fn test_plus_marker_fragment() {
        let result = render("+ revealed item");
        assert!(result.contains("fragment"), "Got: {}", result);
        assert!(result.contains("revealed item"), "Got: {}", result);
    }

    #[test]
    fn test_plus_marker_mixed_list() {
        let result = render("- normal\n+ revealed\n- also normal");
        // Only the middle item should be a fragment
        assert_eq!(result.matches("class=\"fragment\"").count(), 1, "Got: {}", result);
        assert!(result.contains("<li class=\"fragment\">revealed</li>"), "Got: {}", result);
        assert!(result.contains("<li>normal</li>"), "Got: {}", result);
        assert!(result.contains("<li>also normal</li>"), "Got: {}", result);
    }

    #[test]
    fn test_plus_marker_with_annotation() {
        let result = render("+ item {.highlight}");
        assert!(result.contains("fragment"), "Got: {}", result);
        assert!(result.contains("highlight"), "Got: {}", result);
    }

    #[test]
    fn test_strip_plus_marker() {
        assert_eq!(strip_plus_marker("+ hello"), Some("hello"));
        assert_eq!(strip_plus_marker("  + nested"), Some("nested"));
        assert_eq!(strip_plus_marker("- normal"), None);
        assert_eq!(strip_plus_marker("+no space"), None);
    }

    #[test]
    fn test_strip_ordered_plus_marker() {
        assert_eq!(strip_ordered_plus_marker("1+ hello"), Some(("1", "hello")));
        assert_eq!(strip_ordered_plus_marker("  2+ nested"), Some(("2", "nested")));
        assert_eq!(strip_ordered_plus_marker("12+ multi"), Some(("12", "multi")));
        assert_eq!(strip_ordered_plus_marker("1. normal"), None);
        assert_eq!(strip_ordered_plus_marker("+ bullet"), None);
        assert_eq!(strip_ordered_plus_marker("1+no space"), None);
        assert_eq!(strip_ordered_plus_marker("a+ not digit"), None);
    }

    #[test]
    fn test_strip_alpha_marker() {
        assert_eq!(strip_alpha_marker("a. hello"), Some(('a', true, "hello")));
        assert_eq!(strip_alpha_marker("A. hello"), Some(('A', true, "hello")));
        assert_eq!(strip_alpha_marker("  i. nested"), Some(('i', true, "nested")));
        assert_eq!(strip_alpha_marker("I. Intro"), Some(('I', true, "Intro")));
        // `+` is an explicit-reveal alias; `)` is the static form.
        assert_eq!(strip_alpha_marker("a+ hello"), Some(('a', true, "hello")));
        assert_eq!(strip_alpha_marker("a) hello"), Some(('a', false, "hello")));
        assert_eq!(strip_alpha_marker("I) Intro"), Some(('I', false, "Intro")));
        assert_eq!(strip_alpha_marker("1. numeric"), None);
        assert_eq!(strip_alpha_marker("aa. multi"), None);
        assert_eq!(strip_alpha_marker("e.g. abbrev"), None);
        assert_eq!(strip_alpha_marker("a.m. time"), None);
        assert_eq!(strip_alpha_marker("a.no space"), None);
        assert_eq!(strip_alpha_marker("a)no space"), None);
        assert_eq!(strip_alpha_marker("- bullet"), None);
    }

    #[test]
    fn test_lower_alpha_list() {
        let result = render("a. first\na. second\na. third");
        assert!(result.contains("<ol>"), "Got: {}", result);
        assert!(!result.contains("start="), "Stray start attr: {}", result);
        assert_eq!(
            result.matches("class=\"list-lower-alpha fragment\"").count(),
            3,
            "Got: {}",
            result
        );
        assert!(
            result.contains("<li value=\"1\" class=\"list-lower-alpha fragment\">first</li>"),
            "Got: {}",
            result
        );
    }

    #[test]
    fn test_upper_alpha_list() {
        let result = render("A. first\nA. second");
        assert!(result.contains("<ol>"), "Got: {}", result);
        assert_eq!(
            result.matches("class=\"list-upper-alpha fragment\"").count(),
            2,
            "Got: {}",
            result
        );
    }

    #[test]
    fn test_lower_roman_list() {
        let result = render("i. first\ni. second\ni. third");
        assert!(result.contains("<ol>"), "Got: {}", result);
        assert_eq!(
            result.matches("class=\"list-lower-roman fragment\"").count(),
            3,
            "Got: {}",
            result
        );
    }

    #[test]
    fn test_upper_roman_list() {
        let result = render("I. first\nI. second");
        assert!(result.contains("<ol>"), "Got: {}", result);
        assert_eq!(
            result.matches("class=\"list-upper-roman fragment\"").count(),
            2,
            "Got: {}",
            result
        );
    }

    #[test]
    fn test_alpha_marker_preserves_inline_bold() {
        let result = render("a. **Bold** text");
        assert!(result.contains("<strong>Bold</strong>"), "Got: {}", result);
        assert!(!result.contains("**"), "Literal ** leaked: {}", result);
    }

    #[test]
    fn test_alpha_marker_with_annotation() {
        let result = render("a. item {.highlight}");
        assert!(result.contains("list-lower-alpha"), "Got: {}", result);
        assert!(result.contains("highlight"), "Got: {}", result);
    }

    #[test]
    fn test_render_plain_does_not_make_alpha_lists() {
        let result = render_plain("a. one\na. two");
        assert!(!result.contains("list-lower-alpha"), "Got: {}", result);
        assert!(result.contains("a. one"), "Got: {}", result);
    }

    #[test]
    fn test_alpha_items_carry_sequential_values() {
        let result = render("a. one\na. two\na. three");
        assert!(result.contains("<li value=\"1\" class=\"list-lower-alpha fragment\">one</li>"), "Got: {}", result);
        assert!(result.contains("<li value=\"2\" class=\"list-lower-alpha fragment\">two</li>"), "Got: {}", result);
        assert!(result.contains("<li value=\"3\" class=\"list-lower-alpha fragment\">three</li>"), "Got: {}", result);
        // Exactly three <li> — the marker span must not leave a spurious empty one.
        assert_eq!(result.matches("<li").count(), 3, "Spurious <li>: {}", result);
    }

    #[test]
    fn test_adjacent_different_style_lists_restart() {
        // Two lettered lists separated by a blank line merge into one <ol> in
        // CommonMark; explicit `value` must still make the second restart.
        // (A blank line makes this a CommonMark "loose" list, so items wrap
        // their content in <p>; assert the <li> opening tags, not tight text.)
        let result = render("a. one\na. two\n\nA. three\nA. four");
        assert!(result.contains("<li value=\"1\" class=\"list-lower-alpha fragment\">"), "Got: {}", result);
        assert!(result.contains("<li value=\"2\" class=\"list-lower-alpha fragment\">"), "Got: {}", result);
        assert!(result.contains("<li value=\"1\" class=\"list-upper-alpha fragment\">"), "Got: {}", result);
        assert!(result.contains("<li value=\"2\" class=\"list-upper-alpha fragment\">"), "Got: {}", result);
        assert!(result.contains("three"), "Got: {}", result);
    }

    #[test]
    fn test_numbered_then_lettered_keeps_own_counters() {
        let result = render("1. one\n2. two\na. three");
        assert!(result.contains("<li>one</li>"), "Got: {}", result);
        assert!(result.contains("<li>two</li>"), "Got: {}", result);
        // The lettered item restarts at "a" (value=1) instead of continuing to 3.
        assert!(result.contains("<li value=\"1\" class=\"list-lower-alpha fragment\">three</li>"), "Got: {}", result);
    }

    #[test]
    fn test_sequential_letters_through_i_stay_alpha() {
        // A hand-typed a..i list must render its 9th item as alpha "i"
        // (value=9, lower-alpha), NOT roman "ix".
        let src = "a. 1\nb. 2\nc. 3\nd. 4\ne. 5\nf. 6\ng. 7\nh. 8\ni. 9";
        let result = render(&src);
        assert_eq!(result.matches("class=\"list-lower-alpha fragment\"").count(), 9, "Got: {}", result);
        assert!(!result.contains("list-lower-roman"), "9th item wrongly roman: {}", result);
        assert!(result.contains("<li value=\"9\" class=\"list-lower-alpha fragment\">9</li>"), "Got: {}", result);
    }

    #[test]
    fn test_typed_letter_sets_start() {
        let result = render("c. gamma\nc. delta");
        assert!(result.contains("<li value=\"3\" class=\"list-lower-alpha fragment\">gamma</li>"), "Got: {}", result);
        assert!(result.contains("<li value=\"4\" class=\"list-lower-alpha fragment\">delta</li>"), "Got: {}", result);
    }

    #[test]
    fn test_loose_alpha_list_keeps_counting() {
        let result = render("a. one\n\na. two\n\na. three");
        assert!(result.contains("<li value=\"1\" class=\"list-lower-alpha fragment\">"), "Got: {}", result);
        assert!(result.contains("<li value=\"2\" class=\"list-lower-alpha fragment\">"), "Got: {}", result);
        assert!(result.contains("<li value=\"3\" class=\"list-lower-alpha fragment\">"), "Got: {}", result);
    }

    #[test]
    fn test_nested_lettered_returns_to_parent_counter() {
        // Parent alpha list with a nested alpha sublist; the parent must resume
        // at "b" after the nested items, not restart at "a".
        let result = render("a. top\n   a. sub one\n   a. sub two\na. top two");
        assert!(result.contains("<li value=\"1\" class=\"list-lower-alpha fragment\">top"), "Got: {}", result);
        assert!(result.contains("<li value=\"2\" class=\"list-lower-alpha fragment\">top two</li>"), "Got: {}", result);
        // Nested items form their own run starting at 1.
        assert!(result.contains("<li value=\"1\" class=\"list-lower-alpha fragment\">sub one</li>"), "Got: {}", result);
        assert!(result.contains("<li value=\"2\" class=\"list-lower-alpha fragment\">sub two</li>"), "Got: {}", result);
    }

    #[test]
    fn test_code_fence_not_rewritten() {
        let result = render("```\na. this is code\nb. still code\n```");
        assert!(!result.contains("list-lower-alpha"), "Code fence hijacked: {}", result);
        assert!(!result.contains("data-ls"), "Marker leaked into code: {}", result);
        assert!(result.contains("a. this is code"), "Got: {}", result);
    }

    #[test]
    fn test_prose_not_hijacked_mid_paragraph() {
        let result = render("Some running text\nI. mean it, really.");
        assert!(!result.contains("list-upper-roman"), "Prose hijacked: {}", result);
        assert!(result.contains("I. mean it"), "Got: {}", result);
    }

    #[test]
    fn test_initials_not_hijacked() {
        let result = render("J. R. R. Tolkien wrote this.");
        assert!(!result.contains("<ol>"), "Initials hijacked into a list: {}", result);
        assert!(result.contains("<p>J. R. R. Tolkien wrote this.</p>"), "Got: {}", result);
    }

    #[test]
    fn test_standalone_lettered_line_is_a_list() {
        // A single lettered marker at a block boundary is still a list.
        let result = render("a. only item");
        assert!(result.contains("<li value=\"1\" class=\"list-lower-alpha fragment\">only item</li>"), "Got: {}", result);
    }

    #[test]
    fn test_is_alpha_list_marker() {
        assert!(is_alpha_list_marker("a. hello"));
        assert!(is_alpha_list_marker("I. Introduction"));
        assert!(!is_alpha_list_marker("J. R. R. Tolkien"));
        assert!(!is_alpha_list_marker("e.g. thing"));
        assert!(!is_alpha_list_marker("1. numeric"));
    }

    // --- Review round 2 regressions ---

    #[test]
    fn test_multiline_lettered_item_keeps_next_marker() {
        // A continuation line (indented to the content column) must not block
        // the next lettered marker.
        let result = render("a. first\n   more about first\na. second");
        assert!(result.contains("<li value=\"1\" class=\"list-lower-alpha fragment\">first"), "Got: {}", result);
        assert!(result.contains("<li value=\"2\" class=\"list-lower-alpha fragment\">second</li>"), "Got: {}", result);
        assert!(result.contains("more about first"), "Got: {}", result);
        assert!(!result.contains("a. second"), "Second marker left literal: {}", result);
    }

    #[test]
    fn test_lazy_continuation_keeps_next_marker() {
        let result = render("a. one\ncontinued\na. two");
        assert!(result.contains("<li value=\"2\" class=\"list-lower-alpha fragment\">two</li>"), "Got: {}", result);
        assert!(!result.contains("a. two"), "Got: {}", result);
    }

    #[test]
    fn test_indented_code_block_not_rewritten() {
        let result = render("A paragraph.\n\n    a. code line\n    b. more");
        assert!(!result.contains("data-ls"), "Marker leaked into indented code: {}", result);
        assert!(!result.contains("list-lower-alpha"), "Got: {}", result);
        assert!(result.contains("a. code line"), "Got: {}", result);
    }

    #[test]
    fn test_tab_indented_code_not_rewritten() {
        let result = render("A paragraph.\n\n\ta. tabbed code");
        assert!(!result.contains("data-ls"), "Marker leaked into tab code: {}", result);
        assert!(result.contains("a. tabbed code"), "Got: {}", result);
    }

    #[test]
    fn test_annotated_prose_still_blocks_marker() {
        // The {.class} early-continue path must not reset the prose guard.
        let result = render("Some text {.aside}\nI. mean it, really.");
        assert!(!result.contains("list-upper-roman"), "Hijacked after annotation: {}", result);
        assert!(result.contains("I. mean it"), "Got: {}", result);
    }

    #[test]
    fn test_numeric_after_lettered_renders_typed_number() {
        // The numeric item merges into the same <ol>; it must carry its typed
        // value instead of continuing the lettered counter.
        let result = render("a. one\na. two\n1. three");
        assert!(result.contains("<li value=\"1\">three</li>"), "Got: {}", result);
    }

    #[test]
    fn test_plain_numeric_lists_untouched() {
        let result = render("1. one\n2. two\n3. three");
        assert!(!result.contains("value="), "Plain numeric list gained values: {}", result);
        assert!(!result.contains("data-ls"), "Got: {}", result);
    }

    #[test]
    fn test_inline_backtick_run_is_not_a_phantom_fence() {
        // A paragraph line starting with an inline triple-backtick span must
        // not open a phantom fence that disables later transforms.
        let result = render("```a = b``` is assignment.\n\na. one\na. two");
        assert!(result.contains("<li value=\"1\" class=\"list-lower-alpha fragment\">one</li>"), "Got: {}", result);
        assert!(result.contains("<li value=\"2\" class=\"list-lower-alpha fragment\">two</li>"), "Got: {}", result);
    }

    #[test]
    fn test_fence_inside_item_keeps_run_alive() {
        let result = render("a. first\n\n   ```\n   code\n   ```\n\na. second");
        assert!(result.contains("<li value=\"1\" class=\"list-lower-alpha fragment\">"), "Got: {}", result);
        assert!(result.contains("<li value=\"2\" class=\"list-lower-alpha fragment\">"), "Got: {}", result);
        assert!(result.contains("<pre><code>code"), "Got: {}", result);
    }

    #[test]
    fn test_two_space_indent_is_same_level() {
        // CommonMark keeps a 2-space-indented item in the SAME list (nesting
        // needs the content column, 3); the ordinal must match.
        let result = render("a. one\n  a. two");
        assert!(result.contains("<li value=\"1\" class=\"list-lower-alpha fragment\">one"), "Got: {}", result);
        assert!(result.contains("<li value=\"2\" class=\"list-lower-alpha fragment\">two"), "Got: {}", result);
    }

    #[test]
    fn test_blank_then_i_continues_alpha_run_at_ninth() {
        let result = render("g. seven\nh. eight\n\ni. nine\nj. ten");
        assert!(!result.contains("list-lower-roman"), "i wrongly roman: {}", result);
        assert!(result.contains("<li value=\"9\" class=\"list-lower-alpha fragment\">"), "Got: {}", result);
        assert!(result.contains("<li value=\"10\" class=\"list-lower-alpha fragment\">"), "Got: {}", result);
    }

    #[test]
    fn test_blank_then_i_after_alpha_start_is_roman() {
        // With no ordinal match, i after a blank still starts a roman list.
        let result = render("a. one\n\ni. two\ni. three");
        assert!(result.contains("list-lower-roman"), "Got: {}", result);
    }

    #[test]
    fn test_setext_heading_then_lettered_list() {
        // The `===` underline must not block a following list.
        let result = render("My Heading\n==========\na. one\na. two");
        assert!(result.contains("<h1>My Heading</h1>"), "Got: {}", result);
        assert!(result.contains("<li value=\"1\" class=\"list-lower-alpha fragment\">one</li>"), "Got: {}", result);
    }

    #[test]
    fn test_lettered_line_with_setext_underline_is_heading() {
        let result = render("a. Title\n===");
        assert!(result.contains("<h1>a. Title</h1>"), "Got: {}", result);
        assert!(!result.contains("list-lower-alpha"), "Got: {}", result);
    }

    #[test]
    fn test_raw_html_li_does_not_panic() {
        // Previously hit an unwrap: raw `<li` with no `>` before a data-ls span.
        let result = render("<li\n<span data-ls=\"la\" data-v=\"1\"></span>");
        assert!(!result.is_empty());
    }

    #[test]
    fn test_user_authored_span_passes_through() {
        // A hand-written data-ls span in prose must not retro-stamp an earlier
        // closed list item.
        let result = render("- bullet item\n\nDemo of <span data-ls=\"la\" data-v=\"5\"></span> internals.");
        assert!(result.contains("<li>bullet item</li>"), "Bullet retro-stamped: {}", result);
        assert!(result.contains("data-ls"), "User span swallowed: {}", result);
    }

    #[test]
    fn test_backslash_escapes_initial() {
        let result = render("W\\. Edwards Deming wrote:");
        assert!(!result.contains("<ol>"), "Escaped initial hijacked: {}", result);
        assert!(result.contains("W. Edwards Deming"), "Got: {}", result);
    }

    #[test]
    fn test_single_initial_line_still_a_list() {
        // Documented residual: a lone single-initial line at a block boundary
        // is a list (use `W\.` to escape). Locks in the current tradeoff.
        let result = render("W. Edwards Deming wrote:");
        assert!(result.contains("list-upper-alpha"), "Got: {}", result);
    }

    // --- Reveal-by-default lettered items ---

    #[test]
    fn test_lettered_items_are_fragments_by_default() {
        let result = render("a. one\na. two");
        assert_eq!(result.matches("fragment").count(), 2, "Got: {}", result);
        assert!(result.contains("class=\"list-lower-alpha fragment\""), "Got: {}", result);
    }

    #[test]
    fn test_paren_lettered_items_are_static() {
        let result = render("a) one\na) two");
        assert!(!result.contains("fragment"), "a) must not reveal: {}", result);
        assert!(result.contains("<li value=\"1\" class=\"list-lower-alpha\">one</li>"), "Got: {}", result);
        assert!(result.contains("<li value=\"2\" class=\"list-lower-alpha\">two</li>"), "Got: {}", result);
    }

    #[test]
    fn test_plus_lettered_alias_equals_dot() {
        let dot = render("a. one\na. two");
        let plus = render("a+ one\na+ two");
        assert_eq!(dot, plus);
    }

    #[test]
    fn test_mixed_reveal_and_static_share_one_run() {
        let result = render("a) shown\na. revealed\na) also shown");
        assert!(result.contains("<li value=\"1\" class=\"list-lower-alpha\">shown</li>"), "Got: {}", result);
        assert!(result.contains("<li value=\"2\" class=\"list-lower-alpha fragment\">revealed</li>"), "Got: {}", result);
        assert!(result.contains("<li value=\"3\" class=\"list-lower-alpha\">also shown</li>"), "Got: {}", result);
        assert_eq!(result.matches("fragment").count(), 1, "Got: {}", result);
    }

    #[test]
    fn test_roman_items_reveal_by_default() {
        let result = render("i. one\ni. two");
        assert!(result.contains("class=\"list-lower-roman fragment\""), "Got: {}", result);
        let stat = render("I) one\nI) two");
        assert!(!stat.contains("fragment"), "Got: {}", stat);
        assert!(stat.contains("list-upper-roman"), "Got: {}", stat);
    }

    #[test]
    fn test_numeric_dec_stamp_has_no_fragment() {
        let result = render("a. one\n1. two");
        assert!(result.contains("<li value=\"1\">two</li>"), "Got: {}", result);
        assert_eq!(result.matches("fragment").count(), 1, "Only the a. item reveals: {}", result);
    }

    #[test]
    fn test_render_plain_leaves_paren_and_plus_literal() {
        let result = render_plain("a) one\na+ two");
        assert!(!result.contains("list-lower-alpha"), "Got: {}", result);
        assert!(result.contains("a) one"), "Got: {}", result);
        assert!(result.contains("a+ two"), "Got: {}", result);
    }

    #[test]
    fn test_initials_guard_applies_to_new_delimiters() {
        // Content beginning with an initial rejects the marker in every form.
        assert!(!is_alpha_list_marker("a) J. R. R. Tolkien"));
        assert!(!is_alpha_list_marker("a+ J. R. R. Tolkien"));
    }

    #[test]
    fn test_bullet_between_lettered_lists_restarts_run() {
        // After rewrite the bullet forms a separate <ul>, so the second
        // lettered list is a fresh <ol> and must restart at value 1.
        let result = render("a. one\n- bullet\na. two");
        assert_eq!(result.matches("<li value=\"1\" class=\"list-lower-alpha fragment\">").count(), 2, "Got: {}", result);
    }

    #[test]
    fn test_ordered_plus_marker_fragment() {
        let result = render("1+ first\n1+ second\n1+ third");
        assert!(result.contains("<ol>"), "Got: {}", result);
        assert_eq!(result.matches("class=\"fragment\"").count(), 3, "Got: {}", result);
        assert!(result.contains("<li class=\"fragment\">first</li>"), "Got: {}", result);
        assert!(result.contains("<li class=\"fragment\">second</li>"), "Got: {}", result);
    }

    #[test]
    fn test_ordered_plus_marker_mixed_list() {
        let result = render("1. always\n1+ revealed\n1. also always");
        assert!(result.contains("<ol>"), "Got: {}", result);
        assert_eq!(result.matches("class=\"fragment\"").count(), 1, "Got: {}", result);
        assert!(result.contains("<li class=\"fragment\">revealed</li>"), "Got: {}", result);
        assert!(result.contains("<li>always</li>"), "Got: {}", result);
        assert!(result.contains("<li>also always</li>"), "Got: {}", result);
    }

    #[test]
    fn test_ordered_plus_marker_with_annotation() {
        let result = render("1+ item {.highlight}");
        assert!(result.contains("fragment"), "Got: {}", result);
        assert!(result.contains("highlight"), "Got: {}", result);
    }

    #[test]
    fn test_plus_marker_preserves_inline_bold() {
        let result = render("+ **Bold** text");
        assert!(result.contains("<strong>Bold</strong>"), "Got: {}", result);
        assert!(!result.contains("**"), "Literal ** leaked: {}", result);
    }

    #[test]
    fn test_ordered_plus_marker_preserves_inline_bold() {
        let result = render("1+ **The Title** of Ecclesiastes");
        assert!(result.contains("<strong>The Title</strong>"), "Got: {}", result);
        assert!(!result.contains("**"), "Literal ** leaked: {}", result);
    }

    #[test]
    fn test_ordered_plus_marker_preserves_inline_italic() {
        let result = render("1+ *italic* text");
        assert!(result.contains("<em>italic</em>"), "Got: {}", result);
    }

    #[test]
    fn test_plus_marker_preserves_bold_italic_combo() {
        let result = render("+ **_The Title_** of Ecclesiastes");
        assert!(
            result.contains("<strong><em>The Title</em></strong>"),
            "Got: {}",
            result
        );
        assert!(!result.contains("**"), "Literal ** leaked: {}", result);
    }

    #[test]
    fn test_ordered_plus_marker_preserves_bold_italic_combo() {
        let result = render("1+ **_The Title_** of Ecclesiastes");
        assert!(
            result.contains("<strong><em>The Title</em></strong>"),
            "Got: {}",
            result
        );
        assert!(!result.contains("**"), "Literal ** leaked: {}", result);
    }



    #[test]
    fn test_render_plain_does_not_make_ordered_fragments() {
        let result = render_plain("1+ one\n1+ two");
        assert!(!result.contains("fragment"), "Got: {}", result);
    }

    #[test]
    fn test_parse_trailing_annotation() {
        let (before, classes) = parse_trailing_annotation("Some text {.aside}").unwrap();
        assert_eq!(before, "Some text");
        assert_eq!(classes, vec!["aside"]);
    }

    #[test]
    fn test_parse_multiple_trailing_classes() {
        let (before, classes) = parse_trailing_annotation("> text {.callout .warning}").unwrap();
        assert_eq!(before, "> text");
        assert_eq!(classes, vec!["callout", "warning"]);
    }

    #[test]
    fn test_no_trailing_annotation() {
        let result = parse_trailing_annotation("Just normal text");
        assert!(result.is_none());
    }

    #[test]
    fn test_render_plain_basic_headings() {
        let result = render_plain("# Hello\n\nWorld");
        assert!(result.contains("<h1>Hello</h1>"));
        assert!(result.contains("<p>World</p>"));
    }

    #[test]
    fn test_render_plain_does_not_make_fragments() {
        // `+` list items must remain normal list items in plain render,
        // not be converted into fragment-classed <li>s.
        let result = render_plain("+ item one\n+ item two");
        assert!(!result.contains("fragment"), "Got: {}", result);
        assert!(result.contains("<li>item one</li>"), "Got: {}", result);
    }

    #[test]
    fn test_render_plain_does_not_strip_annotations() {
        // `{.class}` trailing annotations are slide-specific; plain render
        // leaves them as literal text.
        let result = render_plain("Some text {.aside}");
        assert!(result.contains("{.aside}"), "Got: {}", result);
        assert!(!result.contains("class=\"aside\""), "Got: {}", result);
    }

    #[test]
    fn test_render_plain_tables_and_code() {
        let table = render_plain("| A | B |\n|---|---|\n| 1 | 2 |");
        assert!(table.contains("<table>"), "Got: {}", table);

        let code = render_plain("```rust\nfn main() {}\n```");
        assert!(code.contains("<code class=\"language-rust\">"), "Got: {}", code);
    }

    #[test]
    fn test_extract_annotations() {
        let input = "# Title\n\nSome text {.aside}\n\nMore text";
        let (cleaned, annotations) = extract_annotations(input);
        assert!(!cleaned.contains("{.aside}"));
        assert_eq!(annotations.len(), 1);
        assert_eq!(annotations[0].classes, vec!["aside"]);
    }
}
