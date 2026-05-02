# Agent Integration

Diaview's long-term goal is to become a Visual REPL for human + AI architecture planning.

Instead of only editing Mermaid text, a user should be able to select a node, type an instruction, and feed structured graph context back to an agent.

## Visual REPL loop

1. Agent produces Mermaid.
2. Diaview renders the diagram.
3. User selects a node.
4. User enters an instruction such as "Add a Redis cache before this".
5. Diaview emits structured JSON.
6. Agent receives the selected-node context and updates the Mermaid.
7. Diaview reopens with the updated graph.

## JSON IPC shape

Planned output shape:

```json
{
  "action": "modify",
  "selected_node": {
    "id": "B",
    "label": "JWT Validator",
    "shape": "diamond"
  },
  "prompt": "Add a Redis cache layer before this for token revocation",
  "current_graph": "graph TD\n    A[Request] --> B{JWT Validator}\n    ..."
}
```

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

A separate integration path is inline rendering:

- detect Mermaid code blocks in agent output
- pipe them through `diaview --inline`
- render ANSI diagrams directly in chat scrollback
- fall back to raw Mermaid if Diaview is unavailable

## Design constraints

- stdout JSON should be machine-readable and stable.
- human UI logs should not corrupt JSON IPC output.
- selected-node context should be sufficient for an agent to make localized edits.
- the full current graph should be available when broader edits are needed.
- exiting without an action should be distinguishable from submitting an action.
