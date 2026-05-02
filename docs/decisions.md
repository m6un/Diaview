# Decisions

This file records durable project decisions so agents and contributors do not repeatedly revisit settled direction.

## Terminal-native rendering

Diaview renders diagrams directly in the terminal using text, Unicode, color, and Ratatui.

It should not rely on Kitty graphics, raster images, or browser rendering as the primary path.

## Own the layout pipeline for now

Diaview should not introduce Dagre, Graphviz, or other hidden layout engines as required runtime dependencies at this stage.

Rationale:

- keeps the CLI simple and portable
- keeps tests pure Rust
- allows terminal-specific layout/routing decisions
- avoids coupling to SVG/browser-oriented coordinate assumptions

This does not forbid future experimentation behind an explicit abstraction, but the default path should remain native and owned.

## Parse existing languages

Diaview should parse existing diagram languages instead of inventing a new DSL.

Current primary language: Mermaid flowcharts.

Future candidates:

- D2
- additional Mermaid diagram types

## Graph IR as the center

Parsers should target a shared Graph IR. Layout, rendering, interaction, and agent integration should consume that IR rather than parser-specific syntax trees.

## Testability over terminal magic

Parser, layout, renderer, and interaction state should be testable with `cargo test` without a real terminal.

Terminal event-loop code should stay thin.

## Agent-native UX

Diaview is designed for both humans and AI agents.

The long-term UX is a Visual REPL where selected diagram context and user intent can be sent back to an agent as structured JSON.
