# Architecture

Diaview is a terminal-native diagram renderer built around a small, testable pipeline.

```text
Mermaid input → parser → Graph IR → layout → Ratatui renderer → terminal
```

## Pipeline responsibilities

### Parser

Converts supported diagram languages into Diaview's shared graph model.

Current focus:

- Mermaid flowcharts
- `graph TD/LR` and `flowchart TD/LR`
- common node shapes and edge styles

Future:

- D2 input
- additional Mermaid diagram types
- richer Mermaid constructs such as `subgraph`

### Graph IR

The `Graph` model is the central interchange format between parser, layout, renderer, interaction, and future agent integrations.

The IR should stay independent of Mermaid-specific syntax so other input languages can target the same pipeline.

### Layout

Assigns positions and dimensions to graph nodes, inserts routing helpers where needed, and prepares the graph for terminal rendering.

Current layout is owned by Diaview and implemented in Rust. Do not introduce hidden layout subprocesses such as Graphviz or Dagre.

### Renderer

Renders the laid-out graph with Ratatui.

The renderer should preserve Diaview's terminal-native identity:

- Unicode box drawing
- truecolor fills
- clean orthogonal edges
- readable labels
- no raster image fallback

### Interaction

The planned interactive layer will add app state, selected nodes, keyboard/mouse navigation, viewport movement, and an action bar.

### Agent integration

The long-term Visual REPL loop uses structured JSON IPC so a selected node plus user instruction can be sent back to an AI agent.

## Source map

- `src/lib.rs` — library exports for testing and integration
- `src/model.rs` — pure graph data model
- `src/parser/mermaid.rs` — Mermaid flowchart parser
- `src/layout.rs` — layout engine and long-edge routing prep
- `src/renderer/canvas.rs` — Ratatui renderer, inline renderer, edge routing, tests
- `src/theme.rs` — static Ayu Dark-inspired terminal theme
- `src/testdata.rs` — reusable graph and Mermaid fixtures
- `src/main.rs` — CLI entry point
