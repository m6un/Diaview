# Layout

Diaview currently owns its layout pipeline in Rust.

Do not introduce Dagre, Graphviz, or another hidden layout engine as a required runtime dependency without an explicit architecture decision.

## Current approach

The current layout is a layered graph layout designed for terminal rendering.

It includes:

- Kahn-style topological ordering
- rank/layer assignment
- barycenter-style crossing reduction
- aspect-ratio-aware node sizing
- dummy nodes for long-edge routing
- route metadata with edge classes, port sides, and lane ids
- outer perimeter lanes for detected back-edges/cyclic return paths
- spacing tuned for terminal cell geometry

## Why own layout for now?

Owning layout keeps Diaview:

- pure Rust
- portable as a CLI
- easy to test with `cargo test`
- terminal-aware instead of browser/SVG-oriented
- free from hidden subprocess/runtime dependencies

## Known limits

The current layout works well for tree-shaped, pipeline-shaped, and moderately branched DAG diagrams.

It struggles with dense real architecture diagrams:

- shared observability sinks create routing walls
- dense or nested cycles can still stress the forward-flow assumption
- dense fan-in/fan-out creates spaghetti
- edge labels collide in crowded branches
- telemetry/error paths visually overwhelm primary flows
- lack of `subgraph`/cluster awareness hurts locality

## Future layout direction

The likely direction is a fuller Sugiyama-style pipeline:

1. detect cycles and mark/reroute back-edges
2. assign stronger layers/ranks
3. insert dummy nodes for all long edges
4. run multiple crossing-minimization sweeps
5. assign coordinates with node/edge/label spacing constraints
6. reserve global orthogonal routing lanes
7. assign ports on node sides
8. bundle shared-source/shared-sink edges
9. support clusters/swimlanes from Mermaid `subgraph`

## Layout abstraction

As layout grows, introduce a layout abstraction so the current implementation can evolve or be swapped without touching parser and renderer code.

Example shape:

```rust
pub trait LayoutEngine {
    fn layout(&self, graph: &mut Graph) -> anyhow::Result<()>;
}
```

Potential implementations:

- `SimpleLayoutEngine` — current native layout
- `SugiyamaLayoutEngine` — future fuller graph drawing pipeline

## Success criteria

A successful complex-layout pass should make diagrams with 30–60 architecture nodes readable, especially when they contain:

- shared logs/metrics/alerts sinks
- service fan-out/fan-in
- queue/event bus patterns
- back-edges
- grouped subsystems
