# Agent Integration

Diaview's long-term goal is to become a Visual REPL for human + AI architecture planning.

Instead of only editing Mermaid text, a user should be able to select a node, type an instruction, and feed structured graph context back to an agent.

## Current integration surface


- `diaview --inline` renders Mermaid diagrams as ANSI terminal output without alternate screen/raw mode.
- The parser/layout/renderer pipeline can be called from the library for tests or future integrations.
- Route metadata, groups, and edge classes are present in the Graph IR after layout, so future tools can inspect more than just nodes and edges.

Not implemented yet:

- interactive selection
- action bar
- JSON IPC output
- Pi extension/tool wrapper

## Visual REPL loop

Planned loop:

1. Agent produces Mermaid.
2. Diaview renders the diagram.
3. User selects a node.
4. User enters an instruction such as "Add a Redis cache before this".
5. Diaview emits structured JSON.
6. Agent receives the selected-node context and updates the Mermaid.
7. Diaview reopens with the updated graph.

## JSON IPC shape

Protocol v1 is a single JSON document with the minimal useful payload:

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

Notes:

- `protocol` is fixed to `diaview.action`.
- `version` is `1`.
- `selected_node` only includes `id` and `label`.
- `mermaid` preserves the exact original source.
- No normalized graph JSON, neighbors, groups, routes, viewport, layout, timestamps, UUIDs, cancellation events, or negotiation.

Potential future additions:

- selected edge/group context
- visible viewport bounds
- route/edge class summaries
- neighboring nodes and incident edges
- full graph text and/or normalized graph JSON

## Pi integration plan

A future Pi extension can expose a tool such as:

```ts
open_diagram({ mermaid: string })
```

Expected behavior:

- suspend Pi's TUI
- launch `diaview --interactive`
- pass Mermaid input via stdin
- collect JSON output from stdout
- feed the JSON context and user prompt back to the agent
- relaunch Diaview with the updated Mermaid if the loop continues

## Inline chat rendering

Inline rendering is the nearer-term integration path:

- detect Mermaid code blocks in agent output
- pipe them through `diaview --inline`
- render ANSI diagrams directly in chat scrollback
- fall back to raw Mermaid if Diaview is unavailable

The CLI side is already implemented. The remaining work is Pi/chat integration and fallback behavior.

## Design constraints

- stdout JSON should be machine-readable and stable in interactive IPC mode.
- human UI logs should not corrupt JSON IPC output.
- selected-node context should be sufficient for an agent to make localized edits.
- the full current graph should be available when broader edits are needed.
- exiting without an action should be distinguishable from submitting an action.
- inline ANSI rendering and JSON IPC should remain separate modes so chat output cannot accidentally corrupt structured IPC.
