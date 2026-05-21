# Parser

Diaview currently parses Mermaid flowcharts into the shared Graph IR.

## Supported Mermaid scope

Current parser focus:

- `graph TD`
- `graph LR`
- `flowchart TD`
- `flowchart LR`
- rectangle nodes: `A[text]`
- rounded nodes: `A(text)`
- diamond nodes: `A{text}`
- circle nodes: `A((text))`
- database/cylinder nodes: `A[(text)]` (currently normalized to rectangle rendering)
- solid arrows: `-->`
- solid links without arrows: `---`
- dashed arrows: `-.->`
- dashed links without arrows: `-.-`
- thick arrows: `==>`
- edge labels: `-->|text|` and `-- text -->`
- comments and semicolon-separated statements

## Parser responsibilities

The parser should:

- preserve stable node ids
- preserve human-readable labels
- normalize Mermaid syntax into graph model enums
- infer graph direction from the flowchart declaration
- avoid doing layout or rendering work

## Out of scope for the parser

The parser should not:

- assign coordinates
- compute node sizes
- route edges
- choose terminal glyphs
- perform visual simplification

Those belong to layout and rendering.

## Future Mermaid support

Important future parser work:

- `subgraph` blocks
- cluster/group metadata in the Graph IR
- class/style declarations if they can map cleanly to themes or semantic node types
- more robust multiline input handling
- better error reporting with line/column context

## Testing guidance

Parser tests should use small Mermaid strings and assert graph structure:

- node count and ids
- edge count and endpoints
- shape mapping
- edge style mapping
- labels
- graph direction

When fixing a parser bug, add the smallest Mermaid fixture that reproduces it.
