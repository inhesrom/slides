# slides

**One markdown file in, a full presentation out.** No editor lock-in, no npm install, no fighting with CSS. Just write your talk and present it.

`slides` is a single Rust binary that turns a `.md` file into a browser-based presentation with live reload, speaker notes, multi-column layouts, and PDF export.

<p align="center">
  <img alt="Editor view" src="https://github.com/user-attachments/assets/5c08a389-425e-4f1e-b80a-fa7c08b89ce2" width="100%" />
  <em>Editor view</em>
</p>

<p align="center">
  <img alt="Presentation view" src="https://github.com/user-attachments/assets/4c12fc7e-6dfd-4ff9-a0b5-72dde3b7d152" width="100%" />
  <em>Presentation view</em>
</p>

## Quick start

```bash
# Generate a starter presentation with all syntax examples
slides init

# Preview with live reload
slides serve presentation.md

# Present with speaker notes, timer, and progress
slides present presentation.md

# Visual editor — create slides in the browser, markdown file updates in real time
slides edit presentation.md

# Export
slides export presentation.md -f html -o deck.html
slides export presentation.md -f pdf -o deck.pdf  # requires --features pdf
```

## What you get

- **Live preview** — saves trigger instant reload in the browser
- **Layouts** — split columns (`:::split 60/40`), grids (`:::grid 2x2`), vertical stacks (`:::stack`)
- **Speaker notes** — block (`:::notes`) or inline (`^[note]`), shown in presenter mode with a timer
- **Fragment reveals** — use `+` as a list marker for items that appear one at a time
- **Semantic styling** — annotate blocks with `{.emphasis}`, `{.callout}`, `{.aside}` and let the theme handle the rest
- **Themes** — `minimal` (light) and `dark` built-in, customizable via CSS custom properties
- **Export** — self-contained HTML or PDF via headless Chrome
- **Visual editor** — `slides edit` opens a browser-based editor with toolbar, layout selector, and live preview; the `.md` file updates in real time
- **Syntax reference** — visit `/help` in the browser while presenting, or run `slides init` for a commented template

## Syntax and usage

- **[SYNTAX.md](SYNTAX.md)** — complete reference for the authoring syntax (frontmatter, layouts, speaker notes, fragments, semantic annotations).
- **[USAGE.md](USAGE.md)** — keyboard shortcuts and CLI commands.
- Running `slides serve` or `slides present`? Visit `http://localhost:3030/help` for the same content rendered in-browser, or `http://localhost:3030/syntax.md` for the raw markdown.

## Install

```bash
# One-line install (macOS Apple Silicon / Linux x86_64)
curl -fsSL https://raw.githubusercontent.com/inhesrom/slides/master/install.sh | bash

# Update to latest
slides update

# Or build from source
cargo install --path .

# With PDF export support
cargo install --path . --features pdf
```

## License

MIT
