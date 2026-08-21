# Rendering

Diaview treats the terminal as a first-class rendering target, not a degraded fallback.

The renderer should produce diagrams that feel native in Ghostty, tmux, neovim, and AI-agent terminal workflows.

## Rendering stack

- Ratatui for buffer/widget rendering
- Crossterm for terminal integration
- Unicode box/line drawing for structure
- Nerd Fonts v3 Material Design glyphs for artifact icons
- 24-bit color where available
- Ratatui `TestBackend` for non-interactive tests and inline rendering

## Font requirement

The active terminal font must be a Nerd Font-patched font or use `Symbols Nerd Font Mono` as a fallback. Diaview emits Nerd Fonts Private Use Area codepoints directly; it does not draw or bundle raster icons.

## Aesthetic direction

Current visual direction:

- 24-bit truecolor fills
- neutral grey surfaces with blue highlights
- borderless filled cards
- semantic icons for shapes and common engineering artifacts
- subtle one-cell shadows
- muted orthogonal edges
- distinct edge colors for telemetry/error/back-edge/external classes
- visible arrowheads
- clean junction glyphs
- fullscreen diagrams centered in the available viewport when they fit
- group boxes drawn behind nodes

## Node rendering

Current node rendering maps model shape to terminal treatment:

| Shape | Treatment |
|-------|-----------|
| Rectangle | filled card |
| Rounded rectangle | filled card with rounded semantic treatment where applicable |
| Diamond | decision styling with `◆` icon |
| Circle | circular semantic treatment with `●` icon |
| Database | database card with the `nf-md-database` icon |

Common engineering artifacts use a curated `nf-md-*` icon vocabulary, including bucket, queue, event, function, worker, cache, router, shield, pulse, monitor, and cloud symbols.

## Group rendering

Mermaid `subgraph` blocks become `Graph.groups`.

Layout computes group bounds after member nodes are positioned. Renderer draws muted cluster boxes behind nodes and edges so grouped architecture sections remain visible without overpowering the primary flow.

## Edge rendering

Edges should be readable on a fixed-width character grid.

Renderer responsibilities include:

- consuming layout-owned `RoutePlan`s when available
- falling back to local routing only when an edge has no route metadata
- converting route segments to box/line glyphs
- merging solid edge junctions where routes share cells
- placing arrowheads at target ports
- drawing dashed/dotted distinctions
- drawing labels at route-provided label anchors
- avoiding node interiors

Layout owns route decisions such as ports, lanes, edge class, perimeter routes, and label anchors. Renderer should not reintroduce global routing policy.

## Inline rendering

Inline rendering is implemented.

`--inline` renders to an in-memory Ratatui backend and emits ANSI output directly to stdout without alternate screen or raw mode.

Useful commands:

```bash
cargo run -- --inline
cargo run -- --inline fixtures/simple.mmd
cargo run -- --inline fixtures/complex_architecture.mmd
```

Goals:

- works in scrollback
- useful in chat/agent output
- no alternate screen
- no raw mode
- can be tested without an interactive terminal

## Testing guidance

Renderer tests live in `tests/renderer_canvas.rs` and should use Ratatui `TestBackend` or `render_to_string`-style helpers.

Avoid tests that require:

- a real terminal
- alternate screen
- raw mode
- manual keypresses

Useful inspection commands:

```bash
cargo run -- --inline
cargo test --test renderer_canvas dump_default_graph -- --nocapture
cargo test --test renderer_canvas dump_complex_architecture_graph -- --nocapture
cargo test --test testdata_fixtures dump_phase15_diagnostic_fixture_metrics -- --nocapture
```
