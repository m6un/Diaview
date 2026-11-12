# Diaview Docs

This folder is the canonical project guide for agents and contributors.

## Current status

Diaview has completed the static renderer MVP and a Phase 1.5 complex-layout/routing baseline.

Current capabilities include:

- Mermaid flowchart parsing
- Mermaid `subgraph` parsing into Graph IR groups
- database/cylinder syntax `A[(label)]` as a dedicated database node
- Nerd Font artifact icons inferred from node ids and labels
- layered layout with dummy nodes for long edges
- layout-owned route metadata with ports, lanes, edge classes, and label anchors
- shared sink/source bundling
- telemetry/error/back-edge/external edge classification
- perimeter routing for cyclic/back-edge workflows
- neutral dark terminal theme with blue semantic accents
- inline ANSI rendering via `--inline`
- persistent Ratatui event loop with `AppState`
- `Tab` / `Shift+Tab` visual-flow node selection (skips dummy nodes)
- selected-node blue highlight
- status bar with mode-specific hints (standalone, Herdr browsing, prompt, waiting)
- Herdr sidecar: action bar (`i`/`Enter` prompt), `herdr agent prompt` dispatch, file-poll live reload
- Pi extension (`integrations/pi/index.ts`): `diaview --inline --compact` tool with ANSI inline rendering and fenced-block fallback
- integration tests for parser, layout, renderer, app state, and fixtures

Next phase: bounded panning for diagrams larger than the terminal.

## Start here

- `roadmap.md` — product roadmap, current status, and long-term vision.
- `architecture.md` — end-to-end pipeline and module responsibilities.
- `development.md` — local setup, workflow, commands, and contribution expectations.

## Subsystems

- `model.md` — shared graph IR: nodes, edges, groups, route metadata, shapes, styles, and layout fields.
- `parser.md` — Mermaid flowchart parsing scope and expected behavior.
- `layout.md` — current layout/routing strategy, known limits, and future layout direction.
- `phase-1.5-layout.md` — implementation record for complex layout/routing baseline.
- `rendering.md` — terminal rendering approach, aesthetic direction, and Ratatui testing.
- `interaction.md` — interactive TUI state, visual-flow selection, Herdr sidecar, and status bar.
- `agent-integration.md` — implemented Visual REPL modes, dormant JSON IPC schema, and Pi inline rendering.

## Quality

- `testing.md` — test commands, renderer inspection, and expectations for non-interactive tests.
- `decisions.md` — durable architectural/product decisions.
