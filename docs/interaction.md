# Interaction

Phase 2 turns Diaview from a static renderer into a navigable interactive TUI.

## Goals

- select nodes
- inspect node details
- navigate with keyboard and mouse
- pan large diagrams
- later issue actions against selected nodes

## Current baseline

Current renderer behavior is still render-once:

- fullscreen mode enters alternate screen, renders, then waits for `q`/`Esc`
- inline mode prints ANSI output to scrollback and exits

There is no persistent app state or selection state yet.

The graph produced after layout already has node coordinates, group bounds, and edge route metadata, which should make Phase 2 navigation and hit-testing easier.

## App state

Expected state includes:

- current graph
- selected node id
- viewport offset
- zoom/spacing level
- interaction mode, e.g. normal vs command input
- optional status/error messages

Example direction:

```rust
struct AppState {
    selected_node: Option<String>,
    viewport_x: i16,
    viewport_y: i16,
}
```

Keep state transitions testable outside the terminal event loop.

## Selection

Planned selection behavior:

- `Tab` cycles forward through visible/selectable nodes
- `Shift+Tab` cycles backward
- arrow keys / `hjkl` move spatially to the nearest node in that direction
- mouse click selects a node directly
- selected node gets a strong visual highlight

Dummy routing nodes should not be selectable.

Future selection may include groups and edges, but node selection should land first.

## Event loop

The current render-once behavior should become a persistent Ratatui event loop.

Expected behavior:

- draw current frame
- poll/read terminal events
- update app state
- redraw
- exit on `q` or configured quit command

Terminal IO should remain thin. Pure functions should handle selection, navigation, viewport updates, and mode transitions.

## Status bar

A bottom status bar should show useful context such as:

- selected node id
- selected node label
- shape/type
- incident edge count or edge classes
- group membership if present
- available keybindings
- current mode

## Pan and zoom

Large diagrams need viewport controls.

Planned controls:

- pan with directional keys or mouse drag
- zoom/spacing changes with `+` and `-`
- keep selected node visible when navigating

## Testing guidance

Interaction logic should be split so most behavior can be tested without a real terminal.

Good tests:

- selection cycling order
- spatial navigation target choice
- viewport offset updates
- command mode transitions
- dummy nodes are skipped
- selected node remains visible after navigation/pan updates

Thin terminal event-loop code can remain lightly tested, but state transitions should be covered.
