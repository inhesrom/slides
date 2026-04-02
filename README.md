# slides

**One markdown file in, a full presentation out.** No editor lock-in, no npm install, no fighting with CSS. Just write your talk and present it.

`slides` is a single Rust binary that turns a `.md` file into a browser-based presentation with live reload, speaker notes, multi-column layouts, and PDF export.

## Quick start

```bash
# Generate a starter presentation with all syntax examples
slides init

# Preview with live reload
slides serve presentation.md

# Present with speaker notes, timer, and progress
slides present presentation.md

# Export
slides export presentation.md -f html -o deck.html
slides export presentation.md -f pdf -o deck.pdf  # requires --features pdf
```

## What you get

- **Live preview** — saves trigger instant reload in the browser
- **Layouts** — split columns (`:::split 60/40`), grids (`:::grid 2x2`), vertical stacks (`:::stack`)
- **Speaker notes** — block (`:::notes`) or inline (`^[note]`), shown in presenter mode with a timer
- **Fragment reveals** — step through `{.fragment}` elements before advancing
- **Semantic styling** — annotate blocks with `{.emphasis}`, `{.callout}`, `{.aside}` and let the theme handle the rest
- **Themes** — `minimal` (light) and `dark` built-in, customizable via CSS custom properties
- **Export** — self-contained HTML or PDF via headless Chrome
- **Syntax reference** — visit `/help` in the browser while presenting, or run `slides init` for a commented template

## Syntax at a glance

```markdown
---
title: My Talk
theme: minimal
aspect: "16:9"
---

# First Slide

Regular markdown: **bold**, *italic*, `code`, tables, lists, footnotes.

^[Inline speaker note — only visible in presenter mode.]

--- {transition: fade}

:::split 60/40

## Left Column

Code, text, images — whatever you need.

+++

## Right Column

Supporting content goes here.

:::

---

## Reveals

- First point {.fragment}
- Second point {.fragment}
- Third point {.fragment}
```

Full syntax reference: run `slides serve` and visit `http://localhost:3030/help`.

## Install

```bash
cargo install --path .

# With PDF export support
cargo install --path . --features pdf
```

## Keyboard shortcuts

| Key | Action |
|---|---|
| `->` `Space` `PgDn` | Next slide/fragment |
| `<-` `PgUp` | Previous |
| `Home` / `End` | First / last slide |
| `F` | Fullscreen |
| `D` | Toggle dark mode |

## License

MIT
