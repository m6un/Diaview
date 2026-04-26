# Diaview

The most beautiful terminal diagram renderer. Parses Mermaid (and later D2) and renders to the terminal as a first-class output — not a degraded fallback.

Built for terminal-native dev workflows where Claude Code, Ghostty, neovim, and tmux users want diagrams without leaving the terminal.

## Pipeline

```
Mermaid input → parser → Graph (model) → layout → Ratatui Canvas → terminal
```

The `Graph` type in `src/model.rs` is the intermediate representation. Parser produces it; layout fills in positions/sizes; renderer consumes it.

## Module layout

- `src/model.rs` — `Graph`, `Node`, `Edge` and their enums. Pure data, no methods. `x/y/width/height` are `Option<f64>` because the parser doesn't know them — layout fills them in.
- `src/parser/mermaid.rs` — Mermaid flowchart syntax → `Graph`
- `src/layout.rs` — assigns positions and sizes to nodes/edges
- `src/renderer/canvas.rs` — `Graph` → terminal output via Ratatui's Canvas widget
- `src/main.rs` — entry point, wires the pipeline

## Tech decisions

- **Ratatui (Rust)** is the rendering foundation. Canvas widget with HalfBlock/Octant markers for sub-character resolution; `Context::print` for crisp text labels composited over canvas layers.
- **No Kitty graphics protocol.** We're rendering natively, not piping in raster images.
- **No dagre or external layout engine.** Ratatui's own layout system is enough for the MVP. Revisit if diagrams get complex.
- **No new DSL.** Parse existing ones (Mermaid first, D2 later). Compete on rendering quality, not language adoption.

## MVP scope

Mermaid flowcharts only:
- Rectangles, rounded rectangles, diamonds, circles
- Solid/dashed/dotted edges with labels
- Top-down and left-right directions
- Box-drawing characters + 24-bit truecolor + theming

Not in MVP: D2, sequence diagrams, Excalidraw, custom DSL, interactive TUI features (pan/zoom).

## Visual targets (what "beautiful" means)

- Rounded corners (`╭╮╰╯`) by default
- Colored borders, semantic per node type
- `▶` arrowheads, clean orthogonal routing
- Padding inside nodes so text breathes
- Theme support (Catppuccin, Dracula, Nord)
- Account for the ~1:2 character aspect ratio when sizing/spacing

The benchmark to beat: `mermaid-ascii`, `graphs-tui`, D2's native ASCII output. They all treat terminal as a fallback. We don't.

## Build & run

```
cargo build
cargo run
cargo check    # fast type-check
```

## Workflow

This project uses a parallel worktree workflow — independent workstreams (parser, renderer, etc.) get spawned as agents in separate worktrees, reviewed in lazygit, then merged to `main`. The orchestrator thread stays clean by delegating implementation to subagents.
