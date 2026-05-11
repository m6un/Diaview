# Phase 1.5 Layout/Routing

Status: complete baseline.

Goal: make 30–60 node architecture diagrams readable without replacing the owned Rust layout pipeline or changing the Mermaid-first model. The implemented baseline keeps the static renderer intact while moving routing decisions out of per-edge drawing and into layout-owned metadata.

## Implemented baseline

- `src/layout.rs` sizes nodes, assigns layers, inserts dummy nodes for long edges, orders layers, writes node coordinates, and computes route plans.
- `src/model.rs` includes route metadata (`RoutePlan`, ports, lane ids, edge class, label anchor) and subgraph group metadata.
- `src/renderer/canvas.rs` consumes route metadata when present and falls back to local routing when absent.
- The layout pass classifies primary, telemetry, error, back-edge, and external edges.
- High-degree/shared semantic endpoints are bundled into bus-like trunks/spokes.
- Telemetry routes prefer perimeter lanes and render dimmer than primary flow.
- Mermaid `subgraph` blocks parse into groups with computed bounds and muted terminal cluster rendering.

## Implementation record

### 1. Stabilize layout diagnostics first

Deliverables:
- Add reusable stress fixtures for fan-in sinks, fan-out routers, back-edges, telemetry overlays, and subgraph-like grouped diagrams.
- Add text/metric assertions before aesthetic assertions: crossings, node overlaps, edge-node collisions, label collisions, lane reuse, route length, and rendered bounds.
- Add a debug dump for route metadata so failures are inspectable without opening a terminal.

Definition of done:
- Current behavior is captured by failing or ignored tests that describe the Phase 1.5 target.
- Every later routing change has a fixture proving it improves one metric without regressing basic diagrams.

### 2. Move routing planning into layout

Deliverables:
- Introduce internal `RoutePlan` data computed after node coordinates: ordered points, source/target port, lane id, edge class, and label anchor.
- Keep renderer responsible for glyph painting only; renderer should consume route points instead of choosing midpoints per edge.
- Preserve compatibility by falling back to local routing when no route plan exists.

Definition of done:
- Simple TD/LR diagrams render the same or better.
- Route metadata can be tested independently from Ratatui buffers.

### 3. Port assignment per node side

Deliverables:
- Compute candidate ports on each side of each node after sizing: top/bottom for TD, left/right for LR, with side ports allowed for back-edges and dummy-routed long edges.
- Assign ports by sorted edge order and direction: outgoing primary edges near the flow axis center, fan-out distributed across the side, incoming fan-in distributed to avoid one-cell walls.
- Reserve distinct ports for edge classes where useful: primary, back-edge, telemetry, error/secondary.

Definition of done:
- Multiple edges from/to the same node do not all share one cell unless intentionally bundled.
- Back-edges enter side or reverse-flow ports and are visually distinguishable from forward flow.

### 4. Global edge lane reservation

Deliverables:
- Build an occupancy grid over layout cells after nodes are placed.
- Reserve node rectangles plus padding as hard obstacles; reserve labels and group borders as soft obstacles.
- Route edges in priority order with a cost function: avoid nodes, avoid labels, minimize crossings, prefer existing bundle trunks when compatible, penalize long detours.
- Assign horizontal/vertical lanes between layers globally rather than using per-edge midpoints.

Definition of done:
- Fan-in/fan-out diagrams use parallel lanes or intentional shared trunks instead of overwriting each other.
- Edge-node collisions and label collisions are measurable and near zero in stress fixtures.

### 5. Shared sink/source bundling

Deliverables:
- Detect high-degree shared sinks/sources (`in_degree`/`out_degree` threshold, initially 4+) and semantic names such as logs, metrics, alerts, events, queue, bus.
- Create bus trunks before the sink/source and short terminal spokes to individual nodes.
- Reuse one reserved lane per compatible bundle, with junction glyphs for solid primary buses and lighter glyphs for telemetry buses.
- Keep labels on spokes or bundle summaries; avoid repeating identical labels along a trunk.

Definition of done:
- Observability sinks and common completion nodes no longer create edge walls.
- The primary request path remains readable when a shared sink has many incoming edges.

### 6. Telemetry and secondary-edge treatment

Deliverables:
- Add an edge classification pass using style, label, and endpoint names: `primary`, `telemetry`, `error`, `back_edge`, `external`.
- Route primary edges first and give them the best lanes.
- Route telemetry after primary edges, prefer perimeter lanes or bundles, dim/dash them, and allow future hide/collapse modes.
- Treat dense telemetry to common sinks as summarized bundles when edge count exceeds a threshold.

Definition of done:
- Dotted/dashed monitoring edges do not cross through the middle of primary service flow unless no perimeter path exists.
- There is a documented path to `--hide-telemetry` or interactive toggles without changing parser syntax.

### 7. Subgraph and swimlane support

Deliverables:
- Parse Mermaid `subgraph` blocks into stable group metadata while preserving node order and labels.
- Extend the Graph IR with groups/clusters: id, label, member node ids, optional parent group, and layout bounds.
- Lay groups out as swimlanes for architecture diagrams: group-local ordering first, then global layer alignment across groups.
- Render group bounds behind nodes and route cross-group edges through group boundary ports.

Definition of done:
- A Mermaid diagram organized as Client / Edge / API / Services / Data / Observability / External renders as clear lanes or boxed clusters.
- Related nodes remain spatially local, and cross-group edges are fewer and more predictable.

## Suggested implementation order

1. Diagnostics and fixtures.
2. Route metadata skeleton and renderer fallback.
3. Port assignment for existing TD/LR layouts.
4. Global lane reservation for primary solid edges.
5. Shared sink/source bundling.
6. Telemetry classification and perimeter/bundle routing.
7. Mermaid subgraph parsing, Graph IR groups, and swimlane rendering.

Each step should land with tests and keep `cargo test` non-interactive. Avoid introducing Graphviz/dagre; this phase is about improving the owned layout/routing pipeline incrementally.
