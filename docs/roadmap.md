# Diaview Roadmap

## Vision

Diaview is a terminal-native diagram renderer that evolves into a **shared spatial workspace** where humans and AI agents co-design architecture visually — without ever leaving the terminal.

The north star: move from "text-in, text-out" chat interfaces to a **Visual REPL** where you point at nodes, give instructions, and watch the diagram transform in real time.

---

## Phase 1: Static Renderer (MVP) ✅

**Status: Complete**

The foundation — parse Mermaid, lay out nodes, render beautifully to the terminal.

### Delivered

- [x] **Model IR** — `Graph`, `Node`, `Edge` types with layout fields as `Option<f64>`
- [x] **Mermaid Parser** — `graph TD/TB/LR`, `flowchart TD/LR`, common node shapes, edge styles, labels, comments, semicolons
- [x] **Layout Engine** — topological layering, barycenter ordering, aspect-ratio-aware sizing, dummy nodes for long-edge routing
- [x] **Terminal Renderer** — Ratatui rendering, 24-bit ANSI colors, filled cards, Nerd Font artifact icons, shadows, orthogonal edges, arrowheads, labels
- [x] **Inline mode** — `--inline` renders ANSI output to scrollback without alternate screen/raw mode

### Node shapes

| Syntax | Model shape | Current rendering |
|--------|-------------|-------------------|
| `A[text]` | Rectangle | filled card |
| `A(text)` | Rounded rect | filled card |
| `A{text}` | Diamond | decision card with `◆` icon |
| `A((text))` | Circle | semantic card with `●` icon |
| `A[(text)]` | Database | database card with a Nerd Font database icon |

### Edge styles

| Syntax | Style | Arrow |
|--------|-------|-------|
| `-->` | Solid | Normal |
| `---` | Solid | None |
| `-.->` | Dashed | Normal |
| `-.-` | Dashed | None |
| `==>` | Solid/thick normalized | Normal |

---

## Phase 1.5: Complex Diagram Layout & Routing ✅

**Status: Complete baseline**

Focused implementation details: [`phase-1.5-layout.md`](phase-1.5-layout.md).

Phase 1.5 addressed the first major stress-test failure: large architecture Mermaid diagrams parsed correctly but rendered as visual spaghetti because routing decisions were too local.

### Delivered implementation sequence

- [x] Added routing diagnostics and stress fixtures before changing behavior
- [x] Introduced layout-owned route metadata: route points, ports, lane ids, edge class, label anchor
- [x] Assigned explicit node-side ports for fan-in, fan-out, long edges, and back-edges
- [x] Added deterministic orthogonal lane reservation for route bands
- [x] Bundled shared sinks/sources into bus-like trunks/spokes, especially logs/metrics/alerts/events/queues
- [x] Classified primary, telemetry, error, back-edge, and external edges
- [x] Routed dense telemetry through muted/perimeter/bundled paths where useful
- [x] Parsed Mermaid `subgraph` blocks into Graph IR groups and rendered them as terminal clusters
- [x] Added routed endpoint stubs so arrows visibly connect to node ports
- [x] Fixed dashed/dotted routed bends
- [x] Improved cyclic payment/workflow routing, including Temporal/Stripe-style return edges

### Baseline success criteria

- [x] Real architecture diagrams with 30–60 nodes have dedicated fixtures and route metadata for inspection
- [x] Shared observability sinks route through bundled trunks/spokes instead of independent edge walls
- [x] Back-edges are classified and routed through outer perimeter lanes
- [x] Primary request flow gets priority over telemetry/error edges, which are classified and visually distinguished
- [x] Mermaid diagrams organized with `subgraph` sections render as terminal clusters

### Still heuristic

The baseline is not a full graph drawing engine. Dense labels, deeply nested cycles, group-aware ranking, and full obstacle routing remain future work.

---

## Phase 2: Interactive Navigation

**Status: Core complete; panning unfinished**

Turn the static renderer into a navigable, interactive TUI application.

### 2.1 Selection Engine

- [x] Add app state with `selected_node: Option<String>` (`AppState` in `src/app.rs`)
- [x] Highlight selected node strongly (blue fill via `theme.accent_primary`)
- [x] `Tab` / `Shift+Tab` to cycle selection through nodes following visual flow
- [x] Dummy routing nodes (`__dummy*`) are never selectable
- [ ] `hjkl` / arrow keys for spatial navigation using node coordinates
- [ ] Mouse click to select a node directly

### 2.2 Persistent Event Loop

- [x] Persistent Ratatui event loop replaces render-once/wait-for-quit
- [x] Continuous rendering with state updates
- [x] Status bar at the bottom showing selected node info and key hints

### 2.3 Pan & Zoom

- [ ] Viewport offset for panning large diagrams *(unfinished — fit-to-screen only)*
- [ ] `+` / `-` for spacing/zoom levels
- [ ] Mouse drag for panning
- [ ] Keep selected node visible while navigating

---

## Phase 3: The Action Bar & Herdr Agent Integration

**Status: Herdr sidecar implemented; JSON IPC schema dormant**

A command bar lets you issue natural language instructions targeted at specific nodes, creating a visual feedback loop with a Herdr-managed AI agent.

### 3.1 The Action Bar (Herdr sidecar only)

- [x] `i` or `Enter` on a selected node opens a one-line action prompt
- [x] Character keys append text; `Backspace` removes one character
- [x] `Esc` cancels the prompt and returns to browsing
- [x] `Enter` submits only non-whitespace prompt text; sidecar enters Waiting state
- [x] `herdr agent prompt` is dispatched with selected node id/label, file path, and instruction
- [x] File polling detects agent-written Mermaid changes and live-reloads the diagram
- [x] Invalid intermediate Mermaid keeps last valid graph on screen with an error status
- [x] `Esc` while waiting stops local polling; `q` while browsing exits

### 3.2 JSON IPC Protocol

The `ActionDocument` v1 schema exists in the codebase for future use but is **not** the active runtime path. The Herdr sidecar uses `herdr agent prompt` and file polling instead. No JSON is written to stdout on prompt submission.

```json
{
  "protocol": "diaview.action",
  "version": 1,
  "selected_node": { "id": "B", "label": "JWT Validator" },
  "prompt": "Add a Redis cache layer before this for token revocation",
  "mermaid": "graph TD\n    A[Request] --> B{JWT Validator}\n    ..."
}
```

### 3.3 Pi Extension — interactive selected-node actions

- [ ] Register a Pi tool that suspends Pi's TUI and launches `diaview --interactive`
- [ ] On exit, read ActionDocument JSON from stdout
- [ ] Feed selected-node context + prompt back to the agent
- [ ] Agent generates updated Mermaid and the tool can relaunch Diaview

This is separate from the **inline rendering** Pi tool (see Phase 4), which is already implemented.

---

## Phase 4: Inline Rendering Integration

**Status: Complete** — CLI and Pi extension both implemented.

```bash
cargo run -- --inline
cargo run -- --inline fixtures/simple.mmd
cargo run -- --inline fixtures/complex_architecture.mmd
```

CLI (`diaview --inline`):

- [x] Skip alternate screen and raw mode
- [x] Render through Ratatui `TestBackend`
- [x] Emit ANSI color output to stdout
- [x] Work naturally in terminal scrollback
- [x] Test via `render_to_string`

Pi extension (`integrations/pi/index.ts`):

- [x] Model-callable `diaview` tool registered via Pi extension API
- [x] Pipes Mermaid source to `diaview --inline --compact -` on stdin
- [x] ANSI diagram rendered inline in Pi chat transcript via `AnsiDiagram` component
- [x] Falls back to fenced Mermaid block + error note if binary not found or parse fails
- [x] `⚠ diaview fallback` label shown in Pi TUI on error

---

## Phase 5: Advanced Features

**Status: Future**

### 5.1 Theming

- [x] Static neutral dark default theme with blue semantic accents
- [ ] Catppuccin, Dracula, Nord built-in themes
- [ ] Theme config file, e.g. `~/.config/diaview/theme.toml`
- [ ] Semantic coloring controls beyond the current edge-class styling

### 5.2 D2 Language Support

- [ ] Parse D2 syntax as an alternative to Mermaid
- [ ] Same Graph IR, same layout, same renderer
- [ ] Auto-detect input format

### 5.3 Diagram Types Beyond Flowcharts

- [ ] Sequence diagrams
- [ ] Entity-relationship diagrams
- [ ] State machines
- [ ] Mind maps

### 5.4 Export

- [ ] `--export svg` — render to SVG file
- [ ] `--export png` — render to PNG via resvg
- [ ] `--export ascii` — plain ASCII art, no Unicode/ANSI

### 5.5 Live Watch Mode

- [ ] `diaview --watch diagram.mmd` — re-render on file change
- [ ] Pairs with editor workflows, e.g. edit Mermaid in neovim and preview in tmux

---

## Architecture

```text
Mermaid input
  ↓
Parser
  ↓
Graph IR
  ↓
Layout + route planning
  ↓
Ratatui renderer
  ├─ fullscreen TUI (event loop, selection, status bar)
  │   ├─ standalone browsing (Tab/Shift+Tab, q/Esc)
  │   └─ Herdr sidecar (action bar, herdr agent prompt, file-poll reload)
  └─ inline ANSI output (--inline, Pi extension tool)

Future interactive path (dormant):
fullscreen TUI → selected node/action bar → ActionDocument JSON → Pi/AI agent
```

## Guiding Principles

1. **Terminal is first-class.** We're not degrading a GUI experience — we're building natively for the terminal.
2. **No hidden external layout engines.** The default path stays pure Rust and terminal-aware.
3. **No new DSL.** Parse existing languages first, currently Mermaid.
4. **Agent-native.** Diaview is designed to be useful in human + AI architecture workflows.
5. **Keyboard-first, mouse-welcome.** Everything should work from the keyboard; mouse support is additive.
