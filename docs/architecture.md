# Architecture

Diaview is a terminal-native diagram renderer built around a small, testable pipeline.

```text
Mermaid input → parser → Graph IR → layout/routing → Ratatui renderer → terminal
```

The current codebase is past the initial static-renderer MVP. Phase 1.5 added layout-owned route metadata, edge classification, lane reservation, shared endpoint bundling, telemetry treatment, back-edge perimeter routing, and Mermaid subgraph groups.

## Pipeline responsibilities

### Parser

Converts supported diagram languages into Diaview's shared graph model.

Current focus:

- Mermaid flowcharts
- `graph TD/TB/LR` and `flowchart TD/LR`
- common node shapes and edge styles
- Mermaid `subgraph` blocks as Graph IR groups
- database/cylinder syntax `A[(label)]` as `NodeShape::Database`

Future:

- D2 input
- additional Mermaid diagram types
- class/style declarations when they can map cleanly to themes or semantic graph metadata

### Graph IR

The `Graph` model is the central interchange format between parser, layout, renderer, interaction, and future agent integrations.

The IR is language-normalized rather than Mermaid-specific. It currently carries:

- graph direction
- nodes and edges
- optional node geometry filled by layout
- optional edge route metadata filled by layout
- group/cluster metadata parsed from Mermaid `subgraph` blocks

### Layout and routing

Assigns positions and dimensions to graph nodes, computes group bounds, inserts helper dummy nodes for long-edge routing, classifies edges, assigns ports, reserves lanes, and writes `RoutePlan` metadata onto edges.

Current layout is owned by Diaview and implemented in Rust. Do not introduce hidden layout subprocesses such as Graphviz or Dagre.

The public abstraction is:

```rust
pub trait LayoutEngine {
    fn layout(&self, graph: &mut Graph);
}
```

The default implementation is `SimpleLayoutEngine`.

### Renderer

Renders the laid-out graph with Ratatui.

The renderer should preserve Diaview's terminal-native identity:

- Unicode box/line drawing
- 24-bit truecolor fills
- neutral dark theme with blue semantic accents and Nerd Font artifact icons
- borderless filled cards with subtle shadows
- clean orthogonal edges from layout route metadata
- readable labels
- no raster image fallback

Renderer routing should be glyph painting first. It consumes layout-owned `RoutePlan`s when present and only falls back to local routing when route metadata is absent.

### Interaction

The planned interactive layer will add app state, selected nodes, keyboard/mouse navigation, viewport movement, status bar, and an action bar.

### Agent integration

The long-term Visual REPL loop uses structured JSON IPC so a selected node plus user instruction can be sent back to an AI agent.

## Source map

- `src/lib.rs` — library exports for testing and integration
- `src/model.rs` — graph data model, route metadata, and group metadata
- `src/parser/mermaid.rs` — Mermaid flowchart parser
- `src/layout.rs` — layout abstraction and public layout entry points
- `src/layout/simple.rs` — default native layout engine, group bounds, edge classification, route planning
- `src/renderer/canvas.rs` — Ratatui renderer, inline renderer, routed edge drawing
- `src/theme.rs` — static neutral dark theme with blue semantic accents
- `src/testdata.rs` — reusable graph and Mermaid fixtures
- `src/main.rs` — CLI entry point
- `tests/` — integration tests for parser, layout, renderer, and fixtures
- `fixtures/` — sample Mermaid diagrams used for smoke/visual inspection
