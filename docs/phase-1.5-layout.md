# Phase 1.5 Layout/Routing

Status: complete baseline, with follow-up fixes for cyclic payment workflows.

Goal: make 30–60 node architecture diagrams readable without replacing the owned Rust layout pipeline or changing the Mermaid-first model. The implemented baseline keeps the static renderer intact while moving routing decisions out of per-edge drawing and into layout-owned metadata.

## Implemented baseline

- `src/layout.rs` exposes the layout abstraction and public entry points.
- `src/layout/simple.rs` sizes nodes, assigns layers, inserts dummy nodes for long edges, orders layers, writes node coordinates, computes group bounds, and computes route plans.
- `src/model.rs` includes route metadata (`RoutePlan`, ports, lane ids, edge class, label anchor) and subgraph group metadata.
- `src/renderer/canvas.rs` consumes route metadata when present and falls back to local routing when absent.
- The layout pass classifies primary, telemetry, error, back-edge, and external edges.
- High-degree/shared semantic endpoints are bundled into bus-like trunks/spokes.
- Telemetry routes can use perimeter lanes and render dimmer than primary flow.
- Error/back-edge/external edges have distinct route classes and rendering colors.
- Mermaid `subgraph` blocks parse into groups with computed bounds and muted terminal cluster rendering.
- Cyclic return edges in payment/workflow diagrams route around the outer perimeter instead of cutting through the primary flow.
- Mermaid database/cylinder syntax `A[(label)]` parses and is normalized to rectangle rendering for now.

## Current implementation notes

### Route metadata

Every routed edge can receive:

- route points
- source/target port
- lane id
- edge class
- label anchor

Renderer uses this metadata to draw glyphs. This is the key architectural shift from local renderer-chosen midpoints to layout-owned routing.

### Back-edges and cyclic return paths

Back-edges are detected after node coordinates are assigned by comparing the main-axis position of source and target. For example:

- in top-down diagrams, a target above the source is a back-edge
- in left-right diagrams, a target left of the source is a back-edge

Current behavior routes back-edges through outer perimeter lanes. In top-down diagrams, back-edge ports use the right side. In left-right diagrams, they use the bottom side.

The Temporal/Stripe payment fixture validates return paths such as webhook callbacks and workflow-to-API final status updates.

### Telemetry and secondary edges

Telemetry-like labels/endpoints are classified separately from primary flow. Dense telemetry routes can prefer perimeter/bundled lanes and render with muted styling. Classification order matters: error and back-edge classification should take precedence over telemetry where applicable.

### Bundling

High-degree shared sinks/sources and semantic bus endpoints such as logs, metrics, alerts, events, queue, and bus-like nodes can share trunk-like route structure instead of producing independent edge walls.

### Groups

Mermaid `subgraph` blocks are parsed into `Graph.groups`. Layout computes bounds around member nodes and renderer draws muted cluster boxes behind nodes.

## Implemented sequence

1. Added diagnostics and complex/stress fixtures.
2. Added route metadata skeleton and renderer fallback.
3. Assigned ports for existing TD/LR layouts.
4. Added lane reservation for routed edges.
5. Added shared sink/source bundling.
6. Added telemetry classification and perimeter/bundle routing.
7. Added Mermaid subgraph parsing, Graph IR groups, and group rendering.
8. Added dashed/dotted routed bend rendering fixes.
9. Added endpoint stubs so arrows visibly connect to node ports.
10. Added cyclic payment/workflow routing fixes and parser support for database/cylinder syntax.

## Important fixtures/tests

- `fixtures/simple.mmd`
- `fixtures/complex_architecture.mmd`
- `fixtures::phase15_fan_in_sink_mermaid()`
- `fixtures::phase15_fan_out_router_mermaid()`
- `fixtures::phase15_back_edge_cycle_mermaid()`
- `fixtures::phase15_telemetry_overlay_mermaid()`
- `fixtures::phase15_grouped_architecture_mermaid()`
- `fixtures::temporal_stripe_payment_mermaid()`

Useful commands:

```bash
cargo test --test layout
cargo test --test testdata_fixtures
cargo test --test testdata_fixtures dump_phase15_diagnostic_fixture_metrics -- --nocapture
cargo run -- --inline fixtures/complex_architecture.mmd
```

## Remaining limitations

This is a baseline, not a complete graph drawing engine.

Known remaining limits:

- no full obstacle-routing/A* grid yet
- no interactive hide/collapse toggles for telemetry or bundles
- dense labels can still crowd route bands
- group layout is bounds rendering, not full swimlane-aware ranking
- database/cylinder nodes do not yet have dedicated shape/rendering
- complex cyclic graphs beyond payment-style callbacks may still need stronger normalization

Each improvement should land with tests and keep `cargo test` non-interactive. Avoid introducing Graphviz/dagre; this phase is about improving the owned layout/routing pipeline incrementally.
