# Diaview Roadmap

## Vision

Diaview is a terminal-native diagram renderer that evolves into a **shared spatial workspace** where humans and AI agents co-design architecture visually — without ever leaving the terminal.

The north star: move from "text-in, text-out" chat interfaces to a **Visual REPL** where you point at nodes, give instructions, and watch the diagram transform in real time.

---

## Phase 1: Static Renderer (MVP) ✅

**Status: Complete**

The foundation — parse Mermaid, lay out nodes, render beautifully to the terminal.

### Delivered
- [x] **Model IR** — `Graph`, `Node`, `Edge` types with `x/y/width/height` as `Option<f64>`
- [x] **Mermaid Parser** — `graph TD/LR`, `flowchart TD/LR`, all node shapes (`[]`, `()`, `{}`, `(())`), edge styles (`-->`, `---`, `-.->`, `-.-`, `==>`), edge labels (`-->|text|`, `-- text -->`), comments, semicolons
- [x] **Layout Engine** — Kahn's topological sort, barycenter crossing reduction, aspect-ratio-aware sizing, dummy nodes for long-edge routing
- [x] **Canvas Renderer** — Ratatui Block widgets, rounded/sharp/double borders per shape, Z-shape orthogonal edge routing, proper corner glyphs (`┌┐└┘`), arrowheads (`▶▼◀▲`), edge labels, node-collision avoidance

### Node shapes
| Syntax | Shape | Border |
|--------|-------|--------|
| `A[text]` | Rectangle | `┌┐└┘` Plain |
| `A(text)` | Rounded Rect | `╭╮╰╯` Rounded |
| `A{text}` | Diamond | `╔╗╚╝` Double + `◇` prefix |
| `A((text))` | Circle | `╭╮╰╯` Rounded |

### Edge styles
| Syntax | Style | Arrow |
|--------|-------|-------|
| `-->` | Solid | `▼` Normal |
| `---` | Solid | None |
| `-.->` | Dashed | Normal |
| `-.-` | Dashed | None |
| `==>` | Solid (thick) | Normal |

---

## Phase 1.5: Complex Diagram Layout & Routing ✅

**Status: Complete baseline**

Focused implementation plan: [`phase-1.5-layout.md`](phase-1.5-layout.md).

The renderer is visually strong for tree-shaped, pipeline-shaped, and moderately branched DAG diagrams. However, stress-testing with real architecture-style Mermaid exposed that Diaview does **not yet handle dense complex diagrams well**.

### Context from stress testing

A large compatible Mermaid diagram with clients, edge services, API routing, service mesh, databases, observability, and external providers parsed successfully, but rendered poorly. The output became visual spaghetti: long edge walls, overlapping dashed monitoring paths, crowded labels, and excessive crossings.

The issue is not primarily styling. It is layout/routing intelligence. Current Diaview has a basic layered layout plus orthogonal edge routing, but complex architecture diagrams need a more complete graph drawing pipeline.

### Problem patterns

- [ ] **Shared sinks create routing walls** — many nodes pointing to `LOGS`, `METRICS`, `ALERTS`, or `DONE` produce huge fan-in congestion
- [ ] **Cycles and back-edges break the forward-flow assumption** — edges such as `Payment --> Billing` create reverse routes in LR/TD layouts
- [ ] **No global edge lane reservation** — edges are routed locally rather than assigned non-overlapping lanes globally
- [ ] **Dense fan-out/fan-in is untreated** — routers, queues, logs, and metrics naturally create bus-like structures that need special rendering
- [ ] **Edge labels collide in crowded branches** — labels need placement that accounts for nearby nodes and other labels
- [ ] **No subgraph/swimlane clustering** — architecture diagrams want sections like Client, Edge, API, Services, Data, Observability, External
- [ ] **Monitoring/telemetry edges overwhelm primary flow** — dotted side-channel edges should be bundled, dimmed, hidden, or routed separately

### Delivered implementation sequence

- [x] Added routing diagnostics and stress fixtures before changing behavior
- [x] Moved edge routing decisions into layout-owned route metadata; renderer keeps a fallback path
- [x] Assigned explicit node-side ports for fan-in, fan-out, long edges, and back-edges
- [x] Added deterministic orthogonal lane reservation for route bands
- [x] Bundled shared sinks/sources into bus-like trunks and short spokes, especially logs/metrics/alerts/events/queues
- [x] Classified telemetry/secondary edges, route them after primary flow, prefer perimeter/bundled lanes, and dim them in rendering
- [x] Parsed Mermaid `subgraph` blocks into Graph IR groups and rendered them as terminal clusters

### Baseline success criteria

- [x] Real architecture diagrams with 30–60 nodes have dedicated stress fixtures and route metadata for inspection
- [x] Shared observability sinks route through bundled trunks/spokes instead of independent edge walls
- [x] Back-edges are classified and routed through side/reverse-flow ports
- [x] Primary request flow gets priority over telemetry/error edges, which are classified and visually distinguished
- [x] Mermaid diagrams organized with `subgraph` sections render as terminal clusters

---

## Phase 2: Interactive Navigation

**Status: Next up**

Turn the static renderer into a navigable, interactive TUI application.

### 2.1 Selection Engine
- [ ] Add `selected_node: Option<String>` to application state
- [ ] Highlighted border (bright white / thick) on the selected node
- [ ] `Tab` / `Shift+Tab` to cycle selection through nodes
- [ ] `hjkl` / arrow keys for spatial navigation (jump to nearest node in that direction using X/Y coordinates)
- [ ] Mouse click to select a node directly

### 2.2 Persistent Event Loop
- [ ] Replace the current "render once, wait for `q`" with a proper Ratatui event loop
- [ ] Continuous rendering with state updates
- [ ] Status bar at the bottom showing selected node info

### 2.3 Pan & Zoom
- [ ] Viewport offset for panning large diagrams
- [ ] `+` / `-` for zoom levels (adjust spacing constants)
- [ ] Mouse drag for panning

---

## Phase 3: The Action Bar & Agent Integration

**Status: Planned**

The core innovation — a command bar that lets you issue natural language instructions targeted at specific nodes, creating a visual feedback loop with an AI agent.

### 3.1 The Action Bar
- [ ] `i` or `Enter` on a selected node opens a text input widget at the bottom of the screen
- [ ] User types a natural language instruction (e.g., "Add a Redis cache before this")
- [ ] `Esc` cancels, `Enter` submits

### 3.2 JSON IPC Protocol
When the user submits a prompt, Diaview outputs structured JSON to stdout:

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

### 3.3 Pi Extension (`diaview.ts`)
- [ ] Register a custom `pi` tool: `open_diagram({ mermaid: string })`
- [ ] Suspends `pi`'s TUI, launches `diaview --interactive` with the Mermaid input via stdin
- [ ] On exit, reads the JSON payload from stdout
- [ ] Feeds the context + prompt back to the agent as a hidden user message
- [ ] Agent generates updated Mermaid, tool auto-relaunches `diaview` with the new graph
- [ ] Loop continues until the user presses `q` to exit the visual workspace

### 3.4 The Visual REPL UX

```
┌─────────────────────────────────────────────────────┐
│                                                     │
│        ╭──────────╮                                 │
│        │ Request  │                                 │
│        ╰────┬─────╯                                 │
│             │                                       │
│             ▼                                       │
│   ╔═══════════════════╗                             │
│   ║ ◇ JWT Validator   ║  ← SELECTED (white border) │
│   ╚═════════╤═════════╝                             │
│        ┌────┴────┐                                  │
│        │         │                                  │
│        ▼         ▼                                  │
│   ╭─────────╮ ╭─────────╮                          │
│   │ Allow   │ │  Deny   │                          │
│   ╰─────────╯ ╰─────────╯                          │
│                                                     │
├─────────────────────────────────────────────────────┤
│ > Add a Redis cache layer before this node          │
└─────────────────────────────────────────────────────┘
```

---

## Phase 4: Inline Rendering Mode

**Status: Planned**

Render diagrams directly inside chat output — no full-screen takeover.

### 4.1 `--inline` CLI Flag
- [ ] Skip alternate screen and raw mode
- [ ] Iterate over the Ratatui buffer and emit ANSI escape codes to stdout
- [ ] Output fits naturally in a terminal scroll buffer
- [ ] Respects terminal width, auto-scales

### 4.2 Pi Chat Integration
- [ ] Intercept ` ```mermaid ``` ` code blocks in agent output
- [ ] Pipe through `diaview --inline` automatically
- [ ] Render beautiful ANSI diagrams directly in the chat flow
- [ ] Falls back to raw Mermaid text if `diaview` binary not found

---

## Phase 5: Advanced Features

**Status: Future**

### 5.1 Theming
- [ ] Ayu, Catppuccin, Dracula, Nord built-in themes
- [ ] Theme config file (`~/.config/diaview/theme.toml`)
- [ ] Semantic coloring (error nodes = red, success = green, etc.)

### 5.2 D2 Language Support
- [ ] Parse D2 syntax as an alternative to Mermaid
- [ ] Same `Graph` IR, same layout, same renderer
- [ ] Auto-detect input format

### 5.3 Diagram Types Beyond Flowcharts
- [ ] Sequence diagrams
- [ ] Entity-relationship diagrams
- [ ] State machines
- [ ] Mind maps

### 5.4 Export
- [ ] `--export svg` — render to SVG file
- [ ] `--export png` — render to PNG via resvg
- [ ] `--export ascii` — plain ASCII art (no Unicode)

### 5.5 Live Watch Mode
- [ ] `diaview --watch diagram.mmd` — re-render on file change
- [ ] Pairs with editor workflows (edit Mermaid in neovim, see live preview in adjacent tmux pane)

---

## Architecture

```
                    ┌─────────────┐
                    │  Mermaid /  │
                    │  D2 Input   │
                    └──────┬──────┘
                           │
                           ▼
                    ┌─────────────┐
                    │   Parser    │
                    └──────┬──────┘
                           │
                           ▼
                    ┌─────────────┐
                    │  Graph IR   │  ← model.rs
                    └──────┬──────┘
                           │
                           ▼
                    ┌─────────────┐
                    │   Layout    │  ← layout.rs
                    └──────┬──────┘
                           │
                           ▼
                    ┌─────────────┐
              ┌─────┤  Renderer   ├─────┐
              │     └─────────────┘     │
              ▼                         ▼
      ┌──────────────┐         ┌──────────────┐
      │  Full-screen │         │   Inline     │
      │  Interactive │         │   ANSI out   │
      └──────┬───────┘         └──────────────┘
             │
             ▼
      ┌──────────────┐
      │  JSON IPC    │ ←→ Pi Extension ←→ AI Agent
      └──────────────┘
```

---

## Guiding Principles

1. **Terminal is first-class.** We're not degrading a GUI experience — we're building natively for the terminal. The output should be *beautiful*, not a fallback.
2. **No external layout engines.** We own the full pipeline. No dagre, no graphviz subprocess. Pure Rust.
3. **No new DSL.** We parse existing languages (Mermaid, D2). We compete on rendering quality and interactivity, not language adoption.
4. **Agent-native.** Diaview is designed from the ground up to be wielded by AI agents, not just humans. The JSON IPC protocol and tool integration make it a first-class agent capability.
5. **Keyboard-first, mouse-welcome.** Everything works with `hjkl` and keybindings. Mouse is supported but never required.
