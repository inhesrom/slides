---
title: slides — Demo Deck
theme: minimal
aspect: "16:9"
transition: slide
color_scheme: light
---

# slides

### Markdown presentations, done right

^[Welcome everyone. This is the demo deck for the slides tool.]

---

## The Problem

Every markdown slide tool either:

- Forces you into an editor ecosystem
- Has terrible layout support
- Breaks when your content overflows
- Requires raw HTML for anything beyond basic text

^[Pause here — let the audience relate to the pain]

---

:::split 60/40

## Code + Explanation

This layout puts code on the left and explanation on the right — the most common slide pattern in technical talks.

```rust
fn main() {
    println!("Hello from slides!");
}
```

+++

### Why Rust?

- Fast file watching
- Zero-cost abstractions for the parser
- Single binary distribution
- Great error handling

:::

--- {transition: fade}

## Semantic Styles

> You annotate *intent*, not CSS. Themes handle the rest.

Regular text flows normally between styled blocks.

This paragraph is supplementary context.

---

:::grid 2x2

### Fast

Hot reload in milliseconds.

+++

### Simple

One file in, presentation out.

+++

### Portable

Single binary, no dependencies.

+++

### Beautiful

Clean defaults, customizable themes.

:::

---

:::notes
This is the summary slide. Keep it brief.
Reiterate the three key points and direct people to the repo.
:::

## Get Started

```bash
$ slides serve deck.md
```

That's it. Start writing.
