# Rendering

Diaview treats the terminal as a first-class rendering target, not a degraded fallback.

The renderer should produce diagrams that feel native in Ghostty, tmux, neovim, and AI-agent terminal workflows.

## Rendering stack

- Ratatui for buffer/widget rendering
- Crossterm for terminal integration
- Unicode box drawing for structure
- 24-bit color where available

## Aesthetic direction

Recent visual polish moved the renderer toward:

- 24-bit truecolor fills
- soft Ayu Dark-inspired colors
- borderless filled cards where appropriate
- semantic icons for non-rectangular shapes, e.g. `◆`, `●`
- soft shadows
- muted orthogonal edges
- visible arrowheads
- clean junction glyphs

## Node rendering

Current node rendering maps model shape to terminal treatment:

| Shape | Treatment |
|-------|-----------|
| Rectangle | plain card/box |
| Rounded rectangle | rounded card/box |
| Diamond | decision styling, often with semantic icon |
| Circle | rounded/circular semantic treatment |

Exact glyph choices may evolve, but readability and terminal compatibility matter more than literal geometric perfection.

## Edge rendering

Edges should be readable on a fixed-width character grid.

Renderer responsibilities include:

- orthogonal line drawing
- corner and junction glyphs
- arrowhead placement
- dashed/solid distinction
- edge label placement
- avoiding node interiors

Long-term, global lane reservation and better routing should move into layout/routing so the renderer is not forced to make purely local decisions.

## Inline rendering

Inline rendering should emit ANSI output without taking over the terminal.

Goals:

- works in scrollback
- useful in chat/agent output
- no alternate screen
- no raw mode
- can be tested without an interactive terminal

## Testing guidance

Renderer tests should use Ratatui `TestBackend` or `render_to_string`-style helpers.

Avoid tests that require:

- a real terminal
- alternate screen
- raw mode
- manual keypresses

Useful inspection commands:

```bash
cargo run -- --inline
cargo test renderer::canvas::tests::dump_default_graph -- --nocapture
cargo test renderer::canvas::tests::dump_complex_architecture_graph -- --nocapture
```
