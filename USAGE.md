# Usage

Keyboard shortcuts and CLI commands for `slides`.

For authoring syntax (what goes *in* a deck file), see [SYNTAX.md](SYNTAX.md)
or `/help` when the app is running.

---

## Keyboard Shortcuts

Active in the browser while viewing or presenting a deck.

| Key | Action |
|---|---|
| `→` `↓` `Space` `PgDn` | Next slide or fragment |
| `←` `↑` `PgUp` | Previous slide or fragment |
| `Home` | First slide |
| `End` | Last slide |
| `F` | Toggle fullscreen |
| `D` | Toggle dark mode |
| `P` | Open presenter view |
| `+` / `=` | (Presenter view) Increase notes font size for current slide |
| `-` | (Presenter view) Decrease notes font size for current slide |
| `Esc` | Exit fullscreen |

Touch: swipe left / right to navigate.

---

## CLI Commands

### `slides init [file]`

Create a new presentation from the starter template. Defaults to
`presentation.md` if no path is given. Refuses to overwrite an existing file.

```bash
slides init
slides init my-talk.md
```

### `slides serve <file>`

Live preview with hot reload. Opens the deck in your default browser and
watches the file for changes.

```bash
slides serve deck.md
slides serve deck.md --port 8080
slides serve deck.md --open false  # don't auto-open the browser
```

Options:

- `-p`, `--port <PORT>` — port to serve on (default `3030`)
- `--open <BOOL>` — open browser automatically (default `true`)

### `slides present <file>`

Opens presenter mode — speaker notes, timer, and progress in the browser.

```bash
slides present deck.md
slides present deck.md --port 8080
```

### `slides edit <file>`

Visual editor in the browser. The `.md` file updates in real time as you edit.
Creates the file if it does not exist.

```bash
slides edit deck.md
```

### `slides export <file>`

Export to a self-contained HTML file or PDF.

```bash
# HTML (default)
slides export deck.md -f html -o output.html

# PDF (requires building with `--features pdf`)
slides export deck.md -f pdf -o output.pdf
```

Options:

- `-f`, `--format <html|pdf>` — output format (default `html`)
- `-o`, `--output <PATH>` — output file path

### `slides update`

Update to the latest release from GitHub.

### `slides reinstall`

Reinstall the latest release, even if already up to date.

---

## Endpoints

While `slides serve`, `slides present`, or `slides edit` is running:

| Path | Description |
|---|---|
| `/` | The rendered deck |
| `/help` | This page plus [SYNTAX.md](SYNTAX.md), rendered |
| `/syntax.md` | Raw `SYNTAX.md` (`Content-Type: text/markdown`) |
| `/presenter` | Presenter view with notes and timer |
| `/edit` | Visual editor (only when started with `slides edit`) |
