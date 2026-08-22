# Interaction

Phase 2 turns Diaview from a static renderer into a navigable interactive TUI.

## Implemented P0 behavior

Fullscreen mode now runs a persistent Ratatui event loop:

- enter raw mode and alternate screen
- draw the current frame
- read a terminal event
- update app state
- redraw
- exit on `q` or `Esc`

A small terminal guard restores raw mode and alternate screen on Result-based exits.

Inline mode is unchanged: it prints ANSI output to scrollback and exits without raw mode or alternate screen.

## App state

The terminal-independent state lives in `AppState` and tracks:

- current graph
- selected node id
- viewport x/y offset

State transitions for selection, panning, pan bounds, and keeping the selected node visible are pure and testable without a real terminal.

## Selection

Implemented controls:

- `Tab` cycles forward through selectable nodes
- `Shift+Tab` / `BackTab` cycles backward
- order is deterministic by node id
- dummy routing nodes with ids beginning `__dummy` are never selectable
- the selected card renders with a strong blue highlight from the existing theme
- when selection changes, the viewport is adjusted so the selected node remains fully visible when it fits in the viewport

Future work may add groups or edges as selectable targets, but P0 selects real nodes only.

## Panning

Implemented controls:

- left/right/up/down arrows pan the viewport
- `h`/`j`/`k`/`l` also pan the viewport
- panning is clamped to graph bounds
- resize events redraw and keep the current selected node visible

Spatial nearest-node navigation, mouse navigation, zoom, and configurable keybindings are not implemented in P0.

## Status bar

Fullscreen mode reserves the bottom terminal row for a concise status bar showing:

- selected node id and label, or `no selection`
- key hints for selection, panning, and quit

The graph renders in the remaining area above the status bar.

## Rendering notes

Fullscreen diagrams remain centered initially when the graph fits in the available graph area. Larger diagrams use the viewport offset.

`render_inline`, `render_to_string`, and coordinate-stable `render_to_frame` behavior remain non-interactive.

## Testing guidance

Interaction logic should stay split so most behavior can be tested without a real terminal.

Covered P0 tests include:

- selection cycling order
- dummy node skipping
- viewport pan bounds
- selected-node ensure-visible behavior
- cycling through more nodes than fit in an 80x23 graph area while keeping each selected node visible
- renderer coverage for selected highlight, status bar, and 80x24 reachability

Thin terminal event-loop code can remain lightly tested, but state transitions should be covered.
