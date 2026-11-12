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

The interactive layer adds app state, selected nodes, keyboard navigation, status bar, and an action bar.

Implemented (`src/app.rs`, `src/renderer/canvas.rs`):

- `AppState` tracks the current graph, selected node id, and mode (`Browsing`, `Prompt`, `Waiting`)
- `Tab` / `Shift+Tab` cycles the selection through real nodes in visual flow order; dummy nodes are skipped
- The selected card renders with a strong blue highlight
- Fullscreen mode runs a persistent Ratatui event loop and reserves the bottom row for a status bar
- Herdr sidecar mode (`--herdr-sidecar`) enables an action bar: `i` or `Enter` opens a one-line prompt, `Enter` dispatches via `herdr agent prompt`, and the sidecar polls the file for agent-written updates

Unfinished: bounded panning for diagrams larger than the terminal.

### Agent integration

The Visual REPL loop has two active modes and one dormant protocol:

- **Inline rendering** (`diaview --inline`): renders a diagram to stdout as ANSI text; consumed by the Pi extension tool (`integrations/pi/index.ts`) which pipes Mermaid source to `diaview --inline --compact -` and shows the result directly in Pi's chat transcript.
- **Herdr sidecar** (`diaview --herdr diagram.mmd`): opens a Herdr-managed pane, runs the sidecar there, and lets a user select a node and dispatch instructions via `herdr agent prompt`. The sidecar polls the file for agent-written edits and live-reloads the diagram.
- **ActionDocument JSON IPC** (dormant): a v1 schema exists for future use, but no JSON is written to stdout in current runtime paths.

## Source map

- `src/lib.rs` — library exports for testing and integration
- `src/model.rs` — graph data model, route metadata, and group metadata
- `src/parser/mermaid.rs` — Mermaid flowchart parser
- `src/layout.rs` — layout abstraction and public layout entry points
- `src/layout/simple.rs` — default native layout engine, group bounds, edge classification, route planning
- `src/app.rs` — `AppState`, mode transitions (Browsing/Prompt/Waiting), selection logic, reload behavior
- `src/renderer/canvas.rs` — Ratatui renderer, inline renderer, routed edge drawing, `render_app_to_frame`, status bar
- `src/theme.rs` — static neutral dark theme with blue semantic accents
- `src/testdata.rs` — reusable graph and Mermaid fixtures
- `src/main.rs` — CLI entry point (`--inline`, `--herdr`, `--herdr-sidecar`, stdin)
- `tests/` — integration tests for parser, layout, renderer, app state, and fixtures
- `fixtures/` — sample Mermaid diagrams used for smoke/visual inspection
- `integrations/pi/index.ts` — Pi extension: model-callable `diaview` tool for inline chat rendering
