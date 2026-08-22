# Agent Integration

Diaview's long-term goal is to become a Visual REPL for human + AI architecture planning.

Instead of only editing Mermaid text, a user can select a node, type an instruction, and send structured context back to the agent that owns the current Herdr pane.

## Current integration surface

- `diaview --inline` renders Mermaid diagrams as ANSI terminal output without alternate screen/raw mode.
- `diaview diagram.mmd` opens a standalone fullscreen viewer with visual-flow node selection only.
- `diaview --herdr diagram.mmd` launches a Herdr sidecar pane for persistent agent-backed edits.
- The parser/layout/renderer pipeline can be called from the library for tests or future integrations.
- Route metadata, groups, and edge classes are present in the Graph IR after layout, so future tools can inspect more than just nodes and edges.

Not implemented yet:

- ACP
- Pi extension/tool wrapper
- normalized graph IPC

## Herdr v0 Visual REPL loop

Implemented loop:

1. An agent runs `diaview --herdr diagram.mmd` from a Herdr-managed pane.
2. Diaview reads `HERDR_PANE_ID`, resolves the Mermaid path to an absolute file path, creates a pane to the right with `herdr pane split`, starts an internal sidecar with `herdr pane run`, then returns immediately.
3. The sidecar renders the diagram and lets the user select a real node.
4. The user presses `i` or `Enter`, types an instruction, and presses `Enter`.
5. The sidecar stays open, enters “Waiting for agent”, and invokes `herdr agent prompt <origin-pane> <instruction>` without a shell.
6. The instruction includes selected node id/label, absolute Mermaid path, the user instruction, and tells the agent to edit/save that file directly within Diaview's supported Mermaid subset.
7. The sidecar polls the file. Valid changed Mermaid is parsed, laid out, and displayed; invalid intermediate Mermaid keeps the last valid graph and shows an invalid-Mermaid status while waiting. Agent/bridge delivery failures show as update errors.

`Esc` while prompting cancels the prompt. `Esc` while waiting only stops local waiting; it does not cancel the external agent. `q` while browsing exits the sidecar.

## JSON IPC shape

The existing protocol v1 schema remains in the codebase for future use:

```json
{
  "protocol": "diaview.action",
  "version": 1,
  "selected_node": {
    "id": "B",
    "label": "JWT Validator"
  },
  "prompt": "Add a Redis cache layer before this for token revocation",
  "mermaid": "graph TD\n    A[Request] --> B{JWT Validator}\n    ..."
}
```

This ActionDocument stdout handoff is **not** the active runtime path for v0. Herdr sidecar mode uses `herdr agent prompt` and file polling instead. No ActionDocument JSON is written to stdout on prompt submission.

Notes for the dormant schema:

- `protocol` is fixed to `diaview.action`.
- `version` is `1`.
- `selected_node` only includes `id` and `label`.
- `mermaid` preserves the exact original source.
- No normalized graph JSON, neighbors, groups, routes, viewport, layout, timestamps, UUIDs, cancellation events, history, or negotiation.

Potential future additions:

- selected edge/group context
- visible viewport bounds
- route/edge class summaries
- neighboring nodes and incident edges
- full graph text and/or normalized graph JSON

## Inline chat rendering

Inline rendering is the nearer-term non-interactive integration path:

- detect Mermaid code blocks in agent output
- pipe them through `diaview --inline`
- render ANSI diagrams directly in chat scrollback
- fall back to raw Mermaid if Diaview is unavailable

The CLI side is already implemented. The remaining work is Pi/chat integration and fallback behavior.

## Design constraints

- Herdr launcher mode requires a real file and `HERDR_PANE_ID`.
- Piped/stdin input remains inline-only for v0.
- Herdr subprocess calls use argument arrays; user prompt, labels, and paths are not shell-interpolated.
- The only shell quoting is the command string passed to `herdr pane run`.
- selected-node context should be sufficient for an agent to make localized edits.
- the full Mermaid file should remain available when broader edits are needed.
- inline ANSI rendering and Herdr sidecar action flow remain separate modes.
