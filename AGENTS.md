# Diaview Agent Guide

Treat this file as the **table of contents for the `docs/` folder**.

For project context, read the relevant docs listed below instead of expanding this file into a full project manual.

## Docs index

- `docs/README.md` — docs index and recommended reading order.
- `docs/roadmap.md` — product roadmap, current phase, and long-term Visual REPL direction.
- `docs/architecture.md` — end-to-end pipeline and module responsibilities.
- `docs/development.md` — local workflow, commands, and contribution expectations.
- `docs/model.md` — shared graph IR: nodes, edges, shapes, styles, and layout fields.
- `docs/parser.md` — Mermaid flowchart parsing scope and expected behavior.
- `docs/layout.md` — current layout strategy, known limits, and future layout direction.
- `docs/rendering.md` — terminal rendering approach, aesthetic direction, and Ratatui testing.
- `docs/interaction.md` — planned interactive TUI state, navigation, viewport, and input model.
- `docs/agent-integration.md` — Visual REPL, JSON IPC, and Pi integration plan.
- `docs/testing.md` — test commands, renderer inspection, and non-interactive test expectations.
- `docs/decisions.md` — durable architectural and product decisions.

## Quick project summary

Diaview is a terminal-native diagram renderer. It parses Mermaid flowcharts into a shared graph model, lays them out, and renders polished terminal diagrams with Ratatui.

```text
Mermaid input → parser → Graph IR → layout → Ratatui renderer → terminal
```

## Source map

- `src/lib.rs` — library exports for testing and integration
- `src/model.rs` — pure data model: `Graph`, `Node`, `Edge`, and enums
- `src/parser/mermaid.rs` — Mermaid flowchart parser
- `src/layout.rs` — assigns positions/sizes and inserts dummy nodes for long-edge routing
- `src/renderer/canvas.rs` — Ratatui renderer, inline renderer, edge routing, tests
- `src/theme.rs` — static Ayu Dark-inspired terminal theme
- `src/testdata.rs` — reusable graph and Mermaid fixtures
- `src/main.rs` — CLI entry point

## Standing guidance

- Use Rust + Ratatui + Crossterm.
- Keep rendering terminal-native; do not use Kitty graphics or raster images.
- Own the layout pipeline for now; do not introduce dagre/graphviz as hidden dependencies.
- Parse existing diagram languages; do not invent a new DSL.
- Keep parser, layout, and renderer testable with `cargo test` without opening a real terminal.

## Before handing off changes

```bash
cargo test
cargo check
```

Useful renderer inspection:

```bash
cargo run -- --inline
cargo test renderer::canvas::tests::dump_default_graph -- --nocapture
cargo test renderer::canvas::tests::dump_complex_architecture_graph -- --nocapture
```
