# Testing

Diaview should remain testable without opening a real terminal.

## Required handoff commands

Run these before handing off code changes:

```bash
cargo test
cargo check
```

## Useful visual inspection

```bash
cargo run -- --inline
cargo test renderer::canvas::tests::dump_default_graph -- --nocapture
cargo test renderer::canvas::tests::dump_complex_architecture_graph -- --nocapture
```

## Parser tests

Parser tests should assert graph structure:

- graph direction
- node ids and labels
- node shapes
- edge endpoints
- edge labels
- edge style and arrow behavior

Use small Mermaid inputs that isolate the syntax being tested.

## Layout tests

Layout tests should assert invariants rather than fragile exact pictures when possible:

- nodes receive positions
- nodes receive dimensions
- ranks respect graph direction
- long edges get expected routing helpers
- nodes do not collide
- layout remains deterministic

Complex diagrams should be kept as fixtures when they reveal layout weaknesses.

## Renderer tests

Renderer tests should use Ratatui `TestBackend` or helpers that render to a string/buffer.

Assert things like:

- labels appear
- expected edge/arrow glyphs appear
- selected nodes are highlighted when interaction lands
- inline output does not require alternate screen/raw mode

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
