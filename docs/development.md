# Development Guide

## Prerequisite

Diaview requires Nerd Fonts v3 glyph support. Configure a patched Nerd Font or install `Symbols Nerd Font Mono` as a terminal fallback.

On macOS with Homebrew:

```bash
brew install --cask font-symbols-only-nerd-font
```

## Local commands

Before handing off changes, run:

```bash
cargo test
cargo check
```

Useful renderer inspection commands:

```bash
cargo run -- --inline
cargo run -- --inline fixtures/simple.mmd
cargo run -- --inline fixtures/complex_architecture.mmd
cargo test --test renderer_canvas dump_default_graph -- --nocapture
cargo test --test renderer_canvas dump_complex_architecture_graph -- --nocapture
cargo test --test testdata_fixtures dump_phase15_diagnostic_fixture_metrics -- --nocapture
```

## Current workflow

The current CLI accepts either no file, a Mermaid file path, or `--inline`:

```bash
cargo run
cargo run -- fixtures/simple.mmd
cargo run -- --inline fixtures/simple.mmd
```

Without a file path, `main.rs` renders the medium simple fixture from `src/testdata.rs`.

## Docs links

- `README.md` — product overview and supported v0 usage
- `docs/architecture.md` — pipeline and module responsibilities
- `docs/parser.md` — Mermaid flowchart parser scope
- `docs/layout.md` — current layout strategy and limits
- `docs/rendering.md` — terminal rendering approach
- `docs/testing.md` — inspection and test guidance
- `docs/roadmap.md` — current phase and next steps

## Development principles

- Keep the project pure Rust where practical.
- Keep the renderer terminal-native; do not depend on Kitty graphics or raster images.
- Do not introduce Dagre, Graphviz, or other hidden layout engines as required runtime dependencies.
- Parse existing diagram languages; do not invent a Diaview-specific DSL.
- Keep parser, layout, renderer, and future interaction state testable with `cargo test` without opening an interactive terminal.

## When changing parser/layout/renderer behavior

Add or update tests close to the subsystem changed.

Good test targets:

- parser fixtures for Mermaid syntax
- layout invariants for node size/position/rank/order/routes/classes
- renderer buffer assertions via Ratatui `TestBackend` or `render_to_string`
- fixture-level smoke tests for complex diagrams

Avoid tests that require a real terminal, raw mode, alternate screen, timing-sensitive loops, or manual keypresses.

## Debugging visual output

Prefer inline rendering and dump tests for inspection:

```bash
cargo run -- --inline <diagram.mmd>
cargo run -- --inline fixtures/complex_architecture.mmd
cargo test --test renderer_canvas dump_default_graph -- --nocapture
```

When a visual regression appears, capture the Mermaid input as a fixture so future changes can be checked against it.

## Documentation expectations

If a change affects architecture, model shape, parser support, layout strategy, rendering philosophy, interaction behavior, or agent IPC, update the relevant file in `docs/`.
