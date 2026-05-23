# Layout

Diaview owns its layout and routing pipeline in Rust.

Do not introduce Dagre, Graphviz, or another hidden layout engine as a required runtime dependency without an explicit architecture decision.

## Current approach

The current layout is a layered graph layout designed for terminal rendering.

It includes:

- Kahn-style topological ordering
- rank/layer assignment
- barycenter-style crossing reduction
- aspect-ratio-aware node sizing
- dummy nodes for long-edge routing
- Mermaid subgraph group bounds
- route metadata with edge classes, port sides, lane ids, route points, and label anchors
- deterministic port assignment for fan-in/fan-out
- lane reservation for route bands
- shared sink/source bundling for bus-like endpoints
- telemetry/error/external/back-edge classification
- perimeter lanes for detected back-edges/cyclic return paths
- spacing tuned for terminal cell geometry

The layout pass mutates the parsed `Graph` by filling node geometry, group bounds, and edge `RoutePlan`s.

## Layout abstraction

The public abstraction is already in place:

```rust
pub trait LayoutEngine {
    fn layout(&self, graph: &mut Graph);
}
```

Current implementation:

- `SimpleLayoutEngine` — native layered layout with route planning

Public entry points:

```rust
layout::layout(&mut graph);
layout::layout_with(&engine, &mut graph);
```

## Route planning responsibilities

Layout should make the routing decisions. Renderer should primarily paint glyphs.

Current route planning includes:

- `RoutePlan.points`
- `RoutePlan.source_port`
- `RoutePlan.target_port`
- `RoutePlan.lane_id`
- `RoutePlan.class`
- `RoutePlan.label_anchor`

Back-edges and cyclic return paths currently route around the graph perimeter. Recent payment-workflow fixtures cover cases such as Stripe webhook and Temporal workflow return edges.

## Edge classes

Current edge classes:

- `Primary` — normal request/data flow
- `Telemetry` — metrics/logs/traces/observability-like side channels
- `Error` — failures, dead letters, blocked/failed responses
- `BackEdge` — reverse-flow/cyclic edges detected from positioned nodes
- `External` — external/provider-style endpoints

Class order matters: back-edge and error classification should win over telemetry when applicable. Tests cover cases such as `login` not being misclassified as telemetry.

## Why own layout for now?

Owning layout keeps Diaview:

- pure Rust
- portable as a CLI
- easy to test with `cargo test`
- terminal-aware instead of browser/SVG-oriented
- free from hidden subprocess/runtime dependencies

## Current strengths

The current baseline handles:

- simple flowcharts
- medium request pipelines
- moderately branched DAGs
- 30–60 node architecture fixtures better than the original midpoint router
- shared observability sinks through bundling
- telemetry side channels through dimmed/perimeter routes
- Mermaid subgraph clusters
- cyclic payment/workflow return edges through perimeter routes

## Known limits

The current layout is still heuristic. It can struggle with:

- very dense or deeply nested cycles
- large graphs with many cross-group edges
- labels in extremely crowded routing bands
- exact swimlane alignment for complex subgraph hierarchies
- semantic layout choices that require domain knowledge
- interactive hide/collapse modes, which do not exist yet

## Future layout direction

The likely direction is a fuller Sugiyama-style pipeline layered on top of the current abstraction:

1. stronger cycle detection and back-edge normalization
2. stronger rank assignment and dummy insertion
3. more crossing-minimization sweeps
4. coordinate assignment with edge/label spacing constraints
5. richer occupancy grid and obstacle-aware routing
6. group/swimlane-aware layout
7. collapsible bundles and telemetry toggles
8. optional future engines behind `LayoutEngine`

## Success criteria

A successful complex-layout pass should make diagrams with 30–60 architecture nodes readable, especially when they contain:

- shared logs/metrics/alerts sinks
- service fan-out/fan-in
- queue/event bus patterns
- back-edges and workflow callbacks
- grouped subsystems
- telemetry/error side channels
