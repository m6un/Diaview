# Decisions

This file records durable project decisions so agents and contributors do not repeatedly revisit settled direction.

## Terminal-native rendering

Diaview renders diagrams directly in the terminal using text, Unicode, color, and Ratatui.

It should not rely on Kitty graphics, raster images, browser rendering, or screenshot generation as the primary path.

## Own the layout pipeline for now

Diaview should not introduce Dagre, Graphviz, or other hidden layout engines as required runtime dependencies at this stage.

Rationale:

- keeps the CLI simple and portable
- keeps tests pure Rust
- allows terminal-specific layout/routing decisions
- avoids coupling to SVG/browser-oriented coordinate assumptions

This does not forbid future experimentation behind the existing `LayoutEngine` abstraction, but the default path should remain native and owned.

## Layout owns routing policy

Routing decisions belong in layout, not renderer.

Layout writes `RoutePlan` metadata onto edges:

- route points
- source/target ports
- lane id
- edge class
- label anchor

Renderer consumes this metadata and paints glyphs. Renderer may keep a fallback route path for compatibility, but new routing intelligence should go into layout.

## Graph IR as the center

Parsers should target a shared Graph IR. Layout, rendering, interaction, and agent integration should consume that IR rather than parser-specific syntax trees.

The IR now includes group metadata and route metadata in addition to nodes and edges.

## Parse existing languages

Diaview should parse existing diagram languages instead of inventing a new DSL.

Current primary language: Mermaid flowcharts.

Future candidates:

- D2
- additional Mermaid diagram types

## Normalize unsupported syntax conservatively

When Mermaid syntax is useful but the model lacks a dedicated semantic type, normalize to the closest existing concept rather than failing unnecessarily.

Current example:

- `A[(Database)]` parses as a rectangle-shaped node until a dedicated database/cylinder shape exists.

## Testability over terminal magic

Parser, layout, renderer, and interaction state should be testable with `cargo test` without a real terminal.

Terminal event-loop code should stay thin.

## Docs are canonical project memory

`AGENTS.md` is the table of contents. Detailed project context belongs in `docs/`.

When changing architecture, model shape, parser support, layout/routing strategy, rendering behavior, interaction behavior, or agent IPC, update the relevant docs file.

## Agent-native UX

Diaview is designed for both humans and AI agents.

The long-term UX is a Visual REPL where selected diagram context and user intent can be sent back to an agent as structured JSON.
