# Interaction

Phase 2 turns Diaview from a static renderer into a small fit-first interactive TUI.

## v0 scope

Diaview v0 targets small and medium diagrams that fit in the terminal. Fullscreen mode centers the fitted diagram in the graph area above the status bar.

Out of scope for v0:

- viewport panning or auto-pan
- scaling or zoom
- responsive relayout on resize
- heavy/oversized diagram navigation

## Implemented behavior

Fullscreen mode runs a persistent Ratatui event loop:

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

Selection state is pure and testable without a real terminal.

## Selection

Implemented controls:

- `Tab` cycles forward through selectable nodes
- `Shift+Tab` / `BackTab` cycles backward
- order is deterministic by node id
- dummy routing nodes with ids beginning `__dummy` are never selectable
- the selected card renders with a strong blue highlight from the existing theme

Future work may add groups, edges, spatial navigation, mouse navigation, zoom, or configurable keybindings, but v0 selects real nodes only.

## Status bar

Fullscreen mode reserves the bottom terminal row for a concise status bar showing:

- selected node id and label, or `no selection`
- key hints for selection and quit

The graph renders centered in the remaining area above the status bar.

## Rendering notes

`render_inline`, `render_to_string`, and coordinate-stable `render_to_frame` behavior remain non-interactive.

## Testing guidance

Interaction logic should stay split so most behavior can be tested without a real terminal.

Covered v0 tests include:

- selection cycling order
- dummy node skipping
- renderer coverage for centered app rendering, selected highlight, status bar, and 80x24 reachability

Thin terminal event-loop code can remain lightly tested, but state transitions should be covered.
