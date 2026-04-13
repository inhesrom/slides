---
title: My Presentation
theme: minimal
aspect: "16:9"
transition: slide
color_scheme: light
# highlight_theme: github
# auto_fit: warn
# export_images: relative
---

# My Presentation

Subtitle or tagline here

^[These are inline speaker notes — only visible in presenter mode.]

---

## Basic Slide

Regular markdown works as expected:

- **Bold text** and *italic text*
- ~~Strikethrough~~ for deletions
- `inline code` for technical terms
- [Links](https://example.com) work too

---

## Code Blocks

```rust
fn main() {
    println!("Syntax highlighting works out of the box.");
}
```

```python
def greet(name: str) -> str:
    return f"Hello, {name}!"
```

---

## Tables

| Feature       | Status |
|---------------|--------|
| Markdown      | Done   |
| Layouts       | Done   |
| Speaker Notes | Done   |
| Export         | Done   |

---

## Task Lists

- [x] Write the slides
- [x] Add speaker notes
- [ ] Rehearse the talk
- [ ] Deliver with confidence

---

## Blockquotes

> "The best way to predict the future is to invent it."
> — Alan Kay

Footnotes work too[^1].

[^1]: This appears at the bottom of the slide.

--- {class: centered}

# Centered Slide

Use `--- {class: centered}` to center all content on a slide.

Perfect for title pages and section dividers.

--- {transition: fade}

## Transitions

This slide used `--- {transition: fade}` as its separator.

You can also set `class` and `timing` on separators:

```markdown
--- {transition: fade, class: centered, timing: 45s}
```

---

## Semantic Annotations

Add classes to any block with trailing `{.classname}` syntax:

This paragraph has emphasis styling. {.emphasis}

> This blockquote is a callout. {.callout}

This is supplementary context. {.aside}

---

## Fragment Reveals

Use `+` instead of `-` for list items that appear one at a time:

+ First point
+ Second point
+ Third point

Press → to reveal each fragment before advancing.

---

:::split 60/40

## Split Layout

Use `:::split` to create columns. This is a 60/40 split — content on the left, sidebar on the right.

Separate regions with `+++`.

+++

### Sidebar

- Supporting detail
- Extra context
- Related links

:::

---

:::split

## Even Split

Omit the ratio for a default 50/50 split.

+++

## Right Column

Both sides get equal space.

:::

---

:::split 33/34/33

### Left

Three-way splits work too.

+++

### Center

Use any number of regions.

+++

### Right

Just match the ratios.

:::

---

:::grid 2x2

### Fast

Hot reload in milliseconds.

+++

### Simple

One markdown file is all you need.

+++

### Portable

Single binary, no dependencies.

+++

### Beautiful

Clean defaults, customizable themes.

:::

---

:::stack

## Stacked Layout

`:::stack` arranges regions vertically with equal spacing.

+++

This is the second region in the stack.

+++

And this is the third.

:::

---

:::notes
These are block speaker notes.

They can span multiple lines and contain **markdown** formatting.

Use these for detailed talking points that you don't want the audience to see.
:::

## Speaker Notes

Two ways to add notes:

1. **Inline:** `^[Your note here]` — embedded in content
2. **Block:** `:::notes ... :::` — a dedicated notes section

^[This is an inline note attached to this slide.]

---

## Keyboard Shortcuts

| Key             | Action              |
|-----------------|---------------------|
| → ↓ Space PgDn  | Next slide/fragment |
| ← ↑ PgUp        | Previous            |
| Home             | First slide         |
| End              | Last slide          |
| F                | Toggle fullscreen   |
| D                | Toggle dark mode    |
| P                | Open presenter view |
| Esc              | Exit fullscreen     |

---

## Commands

```bash
# Live preview with hot reload
slides serve deck.md

# Export to static HTML
slides export deck.md -f html -o output.html

# Export to PDF (requires --features pdf)
slides export deck.md -f pdf -o output.pdf

# Presenter mode with notes & timer
slides present deck.md

# Visual editor in the browser
slides edit deck.md
```

---

## Get Started

1. Edit this file
2. Run `slides serve presentation.md`
3. Start presenting

That's it.
