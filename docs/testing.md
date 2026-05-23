# Testing

Diaview should remain testable without opening a real terminal.

## Required handoff commands

Run these before handing off code changes:

```bash
cargo test
cargo check
```

## Test layout

Tests are currently integration tests:

- `tests/parser_mermaid.rs`
- `tests/layout.rs`
- `tests/renderer_canvas.rs`
- `tests/testdata_fixtures.rs`

The library itself intentionally keeps most behavior exposed and testable, while tests live outside `src/`.

## Useful visual inspection

```bash
cargo run -- --inline
cargo run -- --inline fixtures/simple.mmd
cargo run -- --inline fixtures/complex_architecture.mmd
cargo test --test renderer_canvas dump_default_graph -- --nocapture
cargo test --test renderer_canvas dump_complex_architecture_graph -- --nocapture
cargo test --test testdata_fixtures dump_phase15_diagnostic_fixture_metrics -- --nocapture
```

## Parser tests

Parser tests should assert graph structure:

- graph direction
- node ids and labels
- node shapes
- edge endpoints
- edge labels
- edge style and arrow behavior
- group ids, labels, parents, and members for `subgraph` parsing

Use small Mermaid inputs that isolate the syntax being tested.

## Layout tests

Layout tests should assert invariants rather than fragile exact pictures when possible:

- nodes receive positions
- nodes receive dimensions
- ranks respect graph direction
- long edges get expected routing helpers
- nodes do not collide
- layout remains deterministic
- routed edges receive `RoutePlan`s
- fan-in/fan-out ports are distinct where expected
- telemetry/error/back-edge classification is correct
- perimeter/back-edge routes leave normal graph bounds when expected
- group bounds cover member nodes

Complex diagrams should be kept as fixtures when they reveal layout weaknesses.

Current important fixtures include:

- simple medium request pipeline
- complex architecture diagram
- fan-in sink
- fan-out router
- back-edge cycle
- telemetry overlay
- grouped architecture
- Temporal/Stripe payment workflow with cyclic return edges

## Renderer tests

Renderer tests should use Ratatui `TestBackend` or helpers that render to a string/buffer.

Assert things like:

- labels appear
- expected edge/arrow glyphs appear
- routed endpoint stubs are visible
- dashed/dotted bends connect correctly
- telemetry edges use muted foreground
- group bounds render behind nodes
- inline output does not require alternate screen/raw mode
- selected nodes are highlighted when interaction lands

Avoid tests that require:

- a real terminal
- manual keypresses
- timing-sensitive event loops

## Interaction tests

Interaction behavior should be structured around testable state transitions:

- selection cycling
- spatial navigation
- pan/zoom updates
- mode transitions
- submit/cancel behavior for the future action bar

Keep terminal IO thin and push logic into pure functions where practical.
