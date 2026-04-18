# Slide Syntax Reference

Everything you can put in a `.md` presentation file.

This document is the single source of truth for `slides` authoring syntax. It is
rendered at `/help` when the app is running and served raw at `/syntax.md`.

---

## Frontmatter

Configure the deck with YAML at the top of the file. All fields are optional.

```markdown
---
title: My Presentation
theme: minimal
aspect: "16:9"
transition: slide
color_scheme: light
highlight_theme: github
auto_fit: warn
export_images: relative
title_size: 67px
body_size: 32px
---
```

| Key | Values | Default | Purpose |
|---|---|---|---|
| `title` | string | *(none)* | Deck title, used in the browser tab |
| `theme` | `minimal` \| `dark` | `minimal` | Built-in theme |
| `aspect` | `"16:9"` \| `"4:3"` | `"16:9"` | Slide aspect ratio |
| `transition` | `slide` \| `fade` | `slide` | Default transition between slides |
| `color_scheme` | `light` \| `dark` | `light` | Color scheme applied to the theme |
| `highlight_theme` | syntax theme name | `github` | Code block syntax highlighting theme |
| `auto_fit` | `warn` \| `shrink` | `warn` | Behavior when slide content overflows |
| `export_images` | `relative` \| `inline` | `relative` | How images are embedded during export |
| `title_size` | CSS length | `67px` | Base size for slide titles |
| `body_size` | CSS length | `32px` | Base size for body text |

Any CSS length is accepted in frontmatter (`px`, `rem`, `em`, etc.), but the
editor UI only works with `px` values and will rewrite other units to `px`
when you save changes from the editor.

---

## Slide Separators

Separate slides with `---` on its own line:

```markdown
# First Slide

Content here.

---

# Second Slide

More content.
```

### Separator Attributes

Attach per-slide attributes in curly braces after the separator:

```markdown
--- {transition: fade}
--- {class: centered}
--- {timing: 45s}
--- {title_size: 96px, body_size: 20px}
--- {transition: fade, class: centered, timing: 30s}
```

| Attribute | Values | Purpose |
|---|---|---|
| `transition` | `slide` \| `fade` | Override the deck default for this slide |
| `class` | CSS class name (e.g. `centered`) | Apply a CSS class to the slide wrapper |
| `timing` | duration (e.g. `45s`) | Target speaking time, surfaced in presenter mode |
| `title_size` | CSS length (e.g. `96px`) | Override the deck `title_size` for this slide only |
| `body_size` | CSS length (e.g. `20px`) | Override the deck `body_size` for this slide only |

Size overrides accept any CSS length in hand-authored markdown, but — just like
the deck-level `title_size` / `body_size` — the editor UI only works with `px`
values and will rewrite other units on save. Omitting either key falls back to
the deck default.

### Centered Slides

`class: centered` centers all content horizontally and vertically — good for
title pages and section dividers.

```markdown
--- {class: centered}

# My Presentation

Subtitle goes here
```

---

## Markdown Features

Standard CommonMark plus these extensions.

### Text Formatting

```markdown
**bold**  *italic*  ~~strikethrough~~
`inline code`  [links](https://example.com)
```

### Code Blocks

Fenced code blocks are syntax-highlighted using the `highlight_theme` from
frontmatter.

````markdown
```rust
fn main() {
    println!("highlighted!");
}
```
````

### Tables

```markdown
| Column A | Column B |
|----------|----------|
| Cell 1   | Cell 2   |
```

### Task Lists

```markdown
- [x] Completed item
- [ ] Pending item
```

### Footnotes

```markdown
Text with a footnote[^1].

[^1]: The footnote content.
```

### Blockquotes

```markdown
> "Quoted text here."
> — Attribution
```

---

## Semantic Annotations

Add CSS classes to a block element by appending `{.classname}` to the end of
its line. Multiple classes are space-separated inside the braces.

```markdown
This paragraph is emphasized. {.emphasis}

> This blockquote is styled as a callout. {.callout}

This is secondary text. {.aside}

> A more urgent callout. {.callout .warning}
```

Annotations work on paragraphs, blockquotes, headings, and list items.

### Built-in Classes

| Class | Meaning |
|---|---|
| `.emphasis` | Visual emphasis (theme-dependent) |
| `.callout` | Highlighted callout block |
| `.aside` | De-emphasized secondary text |

Any other class name is passed through to the rendered HTML for you to style
with custom CSS.

---

## Fragment Reveals

Use `+` instead of `-` as a list marker. Fragment items appear one at a time as
you press the advance key.

```markdown
+ First point
+ Second point
+ Third point
```

You can mix normal and fragment items in the same list — only the `+` items
reveal.

```markdown
- Always visible
+ Appears on first reveal
+ Appears on second reveal
- Always visible
```

For numbered lists, replace the `.` after the number with `+` to make that
item a fragment. The list still renders as a normal `1. 2. 3.` ordered list.

```markdown
1+ First point
1+ Second point
1+ Third point
```

Mixing static and fragment items in a numbered list works the same way:

```markdown
1. Always visible
1+ Appears on first reveal
1+ Appears on second reveal
1. Always visible
```

---

## Layout Directives

Multi-region layouts use `:::` fenced blocks. Inside the block, separate
regions with `+++`, and close the block with `:::` on its own line.

### Split (Columns)

```markdown
:::split 60/40

Left column content (60% width).

+++

Right column content (40% width).

:::
```

- Omit the ratio for an even 50/50 split: `:::split`
- Use three or more values for multi-column layouts: `:::split 33/34/33`

### Grid

```markdown
:::grid 2x2

Top-left cell.

+++

Top-right cell.

+++

Bottom-left cell.

+++

Bottom-right cell.

:::
```

Format: `:::grid COLSxROWS`. Provide one region per cell, in row-major order.

### Stack (Vertical)

```markdown
:::stack

First region.

+++

Second region.

+++

Third region.

:::
```

Arranges regions vertically with equal spacing.

---

## Speaker Notes

Notes are visible only in presenter mode (`slides present`).

### Block Notes

```markdown
:::notes
Detailed talking points go here.

Supports **markdown** formatting across multiple lines.
:::
```

### Inline Notes

```markdown
Visible content ^[This note is hidden from the audience.]
```

Inline notes are stripped from the rendered slide and collected into the notes
panel. Nested brackets inside the note are supported.
