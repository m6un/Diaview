# Interaction

Phase 2 turns Diaview from a static renderer into a navigable interactive TUI.

## Goals

- select nodes
- inspect node details
- navigate with keyboard and mouse
- pan large diagrams
- later issue actions against selected nodes

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

## Selection

Planned selection behavior:

- `Tab` cycles forward through nodes
- `Shift+Tab` cycles backward
- arrow keys / `hjkl` move spatially to the nearest node in that direction
- mouse click selects a node directly
- selected node gets a strong visual highlight

## Event loop

The current render-once behavior should become a persistent Ratatui event loop.

Expected behavior:

- draw current frame
- poll/read terminal events
- update app state
- redraw
- exit on `q` or configured quit command

## Status bar

A bottom status bar should show useful context such as:

- selected node id
- selected node label
- shape/type
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

Thin terminal event-loop code can remain lightly tested, but state transitions should be covered.
