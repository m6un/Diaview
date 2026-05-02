# Graph Model

Diaview's graph model is the shared IR between input parsers, layout, rendering, interaction, and future agent integrations.

## Core concepts

- `Graph` — collection of nodes and edges plus global diagram metadata such as direction.
- `Node` — diagram entity with id, label, shape, and optional layout geometry.
- `Edge` — directed or undirected connection between nodes with style, optional label, and arrow behavior.

## Layout fields

Nodes carry layout fields such as `x`, `y`, `width`, and `height` as optional values.

This lets parsing remain geometry-free and allows layout engines to fill positions later.

## Shapes

Current Mermaid flowchart shape support:

| Syntax | Shape |
|--------|-------|
| `A[text]` | Rectangle |
| `A(text)` | Rounded rectangle |
| `A{text}` | Diamond / decision |
| `A((text))` | Circle |

## Edge styles

Current Mermaid flowchart edge style support:

| Syntax | Style | Arrow |
|--------|-------|-------|
| `-->` | Solid | yes |
| `---` | Solid | no |
| `-.->` | Dashed | yes |
| `-.-` | Dashed | no |
| `==>` | Thick/solid | yes |

## Future model additions

Likely additions:

- groups/clusters for Mermaid `subgraph`
- group-level edges
- edge priority classes, e.g. primary flow vs telemetry vs error path
- ports on node sides for better routing
- collapsed/summary nodes for large diagrams
- selection metadata for interactive mode

Keep the IR language-agnostic where possible. Mermaid syntax should be normalized into model concepts instead of leaking parser-specific details through the pipeline.
