# CLAUDE.md

Guidance for Claude Code when working in this repo.

## Single source of truth for slide syntax

`SYNTAX.md` at the repo root is the canonical reference for the markdown
authoring syntax. It is embedded into the binary via `include_str!` and served
at `/help` (rendered) and `/syntax.md` (raw) when the app is running.

**Any change to the parser that adds, removes, or alters an authoring feature
MUST be reflected in `SYNTAX.md` as part of the same change.** This includes:

- New or changed frontmatter keys (`src/parser/frontmatter.rs` → `DeckConfig`).
- New or changed slide separator attributes (`src/parser/mod.rs` → `SlideAttrs`).
- New or changed layout directives (`src/parser/directives.rs`).
- Changes to semantic annotations, fragment reveals, or speaker-note syntax
  (`src/parser/markdown.rs`, `src/parser/directives.rs`).
- Changes to enabled pulldown-cmark extensions.

If behavior changes but syntax does not, `SYNTAX.md` may not need an edit —
use judgment.

## Related documentation

- `USAGE.md` — keyboard shortcuts and CLI commands. Update when keybindings
  change (`static/js/slides.js` etc.) or clap definitions change
  (`src/main.rs`).
- `templates/init.md` — the starter deck written by `slides init`. Keep its
  examples in sync with `SYNTAX.md` when new features are introduced.
- `README.md` — intentionally terse; it points at `SYNTAX.md`, `USAGE.md`,
  and `/help`. Do not reintroduce a full syntax reference here.

## Where content lives

| Content | File | Served at |
|---|---|---|
| Authoring syntax | `SYNTAX.md` | `/help` (rendered), `/syntax.md` (raw) |
| Shortcuts + CLI | `USAGE.md` | `/help` (rendered) |
| Starter deck | `templates/init.md` | written by `slides init` |
| Help page chrome + renderer | `src/help.rs` | `/help` route |
| Plain markdown renderer | `src/parser/markdown.rs` → `render_plain` | — |

The plain renderer intentionally skips slide-specific transformations
(fragments from `+`, `{.class}` annotation stripping) so `SYNTAX.md`'s own
example snippets render faithfully.
