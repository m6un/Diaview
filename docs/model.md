# Graph Model

Diaview's graph model is the shared IR between input parsers, layout/routing, rendering, interaction, and future agent integrations.

The model should stay language-normalized. Mermaid syntax is parsed into graph concepts instead of leaking parser-specific syntax through the pipeline.

## Core concepts

- `Graph` — collection of nodes, edges, direction, and group metadata.
- `Node` — diagram entity with id, label, shape, and optional layout geometry.
- `Edge` — connection between nodes with style, optional label, arrow behavior, and optional route metadata.
- `Group` — Mermaid `subgraph`/cluster metadata with member node ids and layout bounds.

## Graph

Current fields:

- `direction: Direction`
- `nodes: Vec<Node>`
- `edges: Vec<Edge>`
- `groups: Vec<Group>`

`Direction` supports:

- `TopDown`
- `LeftRight`

Mermaid `TB` is normalized to `TopDown`.

## Nodes

Current fields:

- `id: String`
- `label: String`
- `shape: NodeShape`
- `x/y/width/height: Option<f64>`

The parser keeps geometry empty. Layout fills `x`, `y`, `width`, and `height` later.

## Shapes

Current Mermaid flowchart shape support:

| Syntax | Model shape | Notes |
|--------|-------------|-------|
| `A[text]` | `Rectangle` | default box/card |
| `A(text)` | `RoundedRect` | rounded semantic card |
| `A{text}` | `Diamond` | decision styling with `◆` icon |
| `A((text))` | `Circle` | circular semantic styling with `●` icon |
| `A[(text)]` | `Rectangle` | Mermaid database/cylinder syntax, normalized to rectangle for now |

There is no dedicated database shape yet.

## Edges

Current fields:

- `source: String`
- `target: String`
- `label: Option<String>`
- `style: EdgeStyle`
- `arrowhead: Arrowhead`
- `route: Option<RoutePlan>`

Supported styles:

| Mermaid syntax | Style | Arrowhead |
|----------------|-------|-----------|
| `-->` | `Solid` | `Normal` |
| `---` | `Solid` | `None` |
| `-.->` | `Dashed` | `Normal` |
| `-.-` | `Dashed` | `None` |
| `==>` | `Solid` | `Normal` |

`Arrowhead::Open` exists in the model but is not currently parsed from a standard Mermaid operator.

## Route metadata

Layout writes route metadata onto edges so the renderer does not have to invent routes locally.

`RoutePlan` includes:

- ordered `points: Vec<RoutePoint>`
- `source_port: Port`
- `target_port: Port`
- `lane_id: Option<usize>`
- `class: EdgeClass`
- `label_anchor: Option<RoutePoint>`

`Port` contains an `(x, y)` position plus a `PortSide`:

- `Top`
- `Right`
- `Bottom`
- `Left`

`EdgeClass` currently supports:

- `Primary`
- `Telemetry`
- `Error`
- `BackEdge`
- `External`

Edge class affects routing priority, lane choice, port side, and rendering style.

## Groups

`Group` represents Mermaid `subgraph` metadata:

- `id`
- `label`
- `node_ids`
- `parent`
- `x/y/width/height`

Parser fills ids, labels, membership, and nesting parent. Layout computes bounds after member nodes are positioned. Renderer draws muted group boxes behind nodes.

## Future model additions

Likely future additions:

- group-level edges and group boundary ports
- richer shape taxonomy, e.g. dedicated database/cylinder shape
- collapsed/summary nodes for large diagrams
- explicit selection/hover metadata for interactive mode
- viewport or display-state structs kept separate from pure graph data where practical
