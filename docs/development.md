# Development Guide

## Local commands

Before handing off changes, run:

```bash
cargo test
cargo check
```

Useful renderer inspection commands:

```bash
cargo run -- --inline
cargo test renderer::canvas::tests::dump_default_graph -- --nocapture
cargo test renderer::canvas::tests::dump_complex_architecture_graph -- --nocapture
```

## Development principles

- Keep the project pure Rust where practical.
- Keep the renderer terminal-native; do not depend on Kitty graphics or raster images.
- Do not introduce Dagre, Graphviz, or other hidden layout engines as required runtime dependencies.
- Parse existing diagram languages; do not invent a Diaview-specific DSL.
- Keep parser, layout, and renderer testable with `cargo test` without opening an interactive terminal.

## When changing parser/layout/renderer behavior

Add or update tests close to the subsystem changed.

Good test targets:

- parser fixtures for Mermaid syntax
- layout invariants for node size/position/rank/order
- renderer snapshots/dumps via Ratatui `TestBackend` or `render_to_string`

Avoid tests that require a real terminal, raw mode, alternate screen, or manual keypresses.

## Debugging visual output

Prefer inline rendering and dump tests for inspection:

```bash
cargo run -- --inline < diagram.mmd
cargo test renderer::canvas::tests::dump_default_graph -- --nocapture
```

When a visual regression appears, capture the Mermaid input as a fixture so future changes can be checked against it.

## Documentation expectations

If a change affects architecture, layout strategy, rendering philosophy, interaction behavior, or agent IPC, update the relevant file in `docs/`.
