# Diaview

A terminal-native diagram renderer for developers. It parses diagrams into a graph model, lays them out in Rust, and renders polished Ratatui output inline or fullscreen.

## What this repo is

Diaview is an experiment in making architecture diagrams feel native to the terminal. The current renderer supports Mermaid flowcharts, grouped subgraphs, layout-owned routes, bundled shared sinks, and Ayu Dark-inspired terminal styling.

Long term, it is meant to become a visual REPL for agent-assisted architecture work: select a node, give an instruction, and send structured graph context back to an AI agent.

## Architecture

```text
+---------------+
| Mermaid input |
+-------+-------+
        |
        v
+---------------+     +----------+     +------------------+
|    Parser     | --> | Graph IR | --> | Layout + routing |
+---------------+     +----------+     +--------+---------+
                                               |
                                               v
                                      +------------------+
                                      | Ratatui renderer |
                                      +--------+---------+
                                               |
                         +---------------------+---------------------+
                         |                                           |
                         v                                           v
                fullscreen terminal                         inline ANSI output
```

## Run

```bash
cargo run -- --inline fixtures/simple.mmd
cargo run -- --inline fixtures/complex_architecture.mmd
```

## Docs

Start with `docs/README.md`, then `docs/architecture.md` and `docs/roadmap.md`.
