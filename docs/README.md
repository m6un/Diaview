# Diaview Docs

This folder is the canonical project guide for agents and contributors.

## Current status

Diaview has completed the static renderer MVP and a Phase 1.5 complex-layout/routing baseline.

Current capabilities include:

- Mermaid flowchart parsing
- Mermaid `subgraph` parsing into Graph IR groups
- database/cylinder syntax `A[(label)]`, normalized to rectangle rendering
- layered layout with dummy nodes for long edges
- layout-owned route metadata with ports, lanes, edge classes, and label anchors
- shared sink/source bundling
- telemetry/error/back-edge/external edge classification
- perimeter routing for cyclic/back-edge workflows
- Ayu Dark-inspired terminal rendering
- inline ANSI rendering via `--inline`
- integration tests for parser, layout, renderer, and fixtures

Next major product phase: interactive navigation and selection.

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
- `interaction.md` — planned interactive TUI state, navigation, viewport, and input model.
- `agent-integration.md` — Visual REPL, JSON IPC, and Pi integration plan.

## Quality

- `testing.md` — test commands, renderer inspection, and expectations for non-interactive tests.
- `decisions.md` — durable architectural/product decisions.
