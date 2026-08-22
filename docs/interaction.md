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

Standalone fullscreen mode (`diaview diagram.mmd`) runs a persistent Ratatui event loop:

- enter raw mode and alternate screen
- draw the current frame
- read a terminal event
- update selection state
- redraw
- exit on `q` or `Esc` while browsing

A small terminal guard restores raw mode and alternate screen on normal and Result-based exits. Inline mode is unchanged: it prints ANSI output to scrollback and exits without raw mode or alternate screen. Piped/stdin input remains inline-only for v0.

## Herdr sidecar mode

`diaview --herdr diagram.mmd` is a launcher for Herdr-managed agent panes. It requires a real file and `HERDR_PANE_ID`, opens a pane to the right, starts an internal Diaview sidecar there, then returns immediately to avoid blocking the originating agent.

The sidecar adds an action prompt:

- `i` or `Enter` while browsing opens a one-line prompt for the selected node
- character keys append Unicode text
- `Backspace` removes one character
- `Esc` while prompting cancels and returns to browsing
- `Enter` submits only non-whitespace prompt text
- `q` is ordinary prompt text while prompting

Submit does not exit Diaview. The sidecar enters a local “Waiting for agent” state and runs `herdr agent prompt <origin-pane> <instruction>` with context for the selected node, Mermaid file path, and user instruction. `Esc` while waiting stops waiting locally and returns to browsing; it does not cancel the external agent. `q` while browsing exits the sidecar normally.

While waiting, Diaview polls the Mermaid file at a modest interval. When the file changes, valid Mermaid is parsed, laid out, and rendered. The selected node id is preserved if it still exists; otherwise selection falls back to the first visual-flow selectable node. Invalid intermediate Mermaid leaves the last valid graph on screen and shows an invalid-Mermaid status while continuing to wait for another file change. Agent/bridge delivery failures show as update errors, not Mermaid parse errors.

## App state

The terminal-independent state lives in `AppState` and tracks:

- current graph
- selected node id
- browsing, one-line prompt text, or waiting state with optional update/error status

Selection, prompt transitions, waiting transitions, and reload behavior are pure and testable without a real terminal.

## Selection

Implemented controls:

- `Tab` cycles forward through selectable nodes
- `Shift+Tab` / `BackTab` cycles backward
- order follows rendered flow: top-to-bottom for TB/TD graphs, left-to-right for LR graphs
- dummy routing nodes with ids beginning `__dummy` are never selectable
- the selected card renders with a strong blue highlight from the existing theme

Future work may add groups, edges, spatial navigation, mouse navigation, zoom, or configurable keybindings, but v0 selects real nodes only.

## Status bar

Fullscreen mode reserves the bottom terminal row for concise status:

- standalone browsing: selected node plus selection/quit hints only
- Herdr browsing: selected node plus selection, `i`/`Enter` action, and quit hints
- Herdr prompt: selected node, typed prompt, submit/cancel hints
- Herdr waiting: waiting/reload status, update error if any, and `Esc` local-stop hint

The graph renders centered in the remaining area above the status bar.

## Rendering notes

`render_inline`, `render_to_string`, and coordinate-stable `render_to_frame` behavior remain non-interactive.

## Testing guidance

Interaction logic should stay split so most behavior can be tested without a real terminal.

Covered v0 tests include:

- selection cycling order
- dummy node skipping
- prompt and waiting transitions
- valid/invalid reload behavior
- renderer coverage for centered app rendering, selected highlight, status bars, and 80x24 reachability

Thin terminal event-loop and Herdr subprocess code can remain lightly tested, but state transitions should be covered.
