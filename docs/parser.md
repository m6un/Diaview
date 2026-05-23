# Parser

Diaview currently parses Mermaid flowcharts into the shared Graph IR.

## Supported Mermaid scope

Current parser support:

- `graph TD`
- `graph TB` normalized to top-down
- `graph LR`
- `flowchart TD`
- `flowchart LR`
- rectangle nodes: `A[text]`
- rounded nodes: `A(text)`
- diamond nodes: `A{text}`
- circle nodes: `A((text))`
- database/cylinder nodes: `A[(text)]` normalized to rectangle rendering for now
- bare node refs: `A`, normalized to a rectangle with label `A`
- solid arrows: `-->`
- solid links without arrows: `---`
- dashed arrows: `-.->`
- dashed links without arrows: `-.-`
- thick arrows: `==>`, normalized to solid rendering for now
- edge labels: `-->|text|` and `-- text -->`
- Mermaid `subgraph` blocks with group ids, labels, membership, and parent group metadata
- whole-line comments starting with `%%`
- semicolon-separated statements

## Parser responsibilities

The parser should:

- preserve stable node ids
- preserve human-readable labels
- normalize Mermaid syntax into graph model enums
- infer graph direction from the flowchart declaration
- populate `Graph.groups` for supported `subgraph` blocks
- leave geometry and route metadata empty
- avoid doing layout or rendering work

## Subgraph behavior

Supported forms include:

```mermaid
subgraph API[API Layer]
    A[Gateway] --> B[Router]
end
```

and simple labels/ids:

```mermaid
subgraph Services
    S[Service]
end
```

Nodes referenced inside a subgraph are added to that group's `node_ids`. Nested subgraphs preserve the parent group id.

## Out of scope for the parser

The parser should not:

- assign coordinates
- compute node sizes
- route edges
- classify edge semantics
- choose terminal glyphs
- perform visual simplification

Those belong to layout and rendering.

## Future Mermaid support

Important future parser work:

- class/style declarations if they can map cleanly to themes or semantic node/edge types
- more robust multiline input handling
- inline comments if needed
- richer Mermaid shape mapping, including a dedicated database/cylinder shape if the model grows one
- better error reporting with line/column context

## Testing guidance

Parser tests live in `tests/parser_mermaid.rs` and should use small Mermaid strings that assert graph structure:

- node count and ids
- edge count and endpoints
- group count and membership
- shape mapping
- edge style mapping
- labels
- graph direction

When fixing a parser bug, add the smallest Mermaid fixture that reproduces it.
