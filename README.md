# Diaview

Diaview is a terminal-native diagram renderer for Mermaid flowcharts. It parses a supported Mermaid flowchart subset, lays it out in Rust, and renders it with Ratatui.

## Current scope

Supported today:

- `graph TD`
- `graph TB` normalized to top-down
- `graph LR`
- `flowchart TD`
- `flowchart LR`
- rectangle nodes: `A[text]`
- rounded nodes: `A(text)`
- diamond nodes: `A{text}`
- circle nodes: `A((text))`
- database/cylinder nodes: `A[(text)]`
- bare node refs: `A`
- solid arrows: `-->`
- solid links without arrows: `---`
- dashed arrows: `-.->`
- dashed links without arrows: `-.-`
- thick arrows: `==>`
- edge labels: `-->|text|` and `-- text -->`
- `subgraph` blocks with ids, labels, and membership
- whole-line comments starting with `%%`
- semicolon-separated statements

Not supported yet:

- interactive editing or selection
- agent / Visual REPL features
- class/style declarations
- non-flowchart Mermaid diagram types
- Graphviz / dagre-backed layout
- general-purpose Mermaid syntax coverage

## Usage

Render fullscreen from a file:

```bash
cargo run -- fixtures/simple.mmd
```

Render inline from a file:

```bash
cargo run -- --inline fixtures/simple.mmd
```

Render inline from stdin:

```bash
cat fixtures/simple.mmd | cargo run -- --inline
```

## License

Diaview is dual-licensed under either [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE).

## Font requirement

Diaview requires Nerd Fonts v3 glyph support.

Use a patched Nerd Font, or install `Symbols Nerd Font Mono` as a terminal fallback.

On macOS with Homebrew:

```bash
brew install --cask font-symbols-only-nerd-font
```

## Development

```bash
cargo fmt --check
cargo check
cargo test
cargo run -- --inline fixtures/complex_architecture.mmd
```

## Current limitations

- terminal-native rendering only
- no fullscreen interaction yet
- no hidden layout engine dependency
- no raster / Kitty graphics output
- parser and layout support are intentionally narrow

## Docs

- `docs/README.md`
- `docs/architecture.md`
- `docs/development.md`
- `docs/parser.md`
- `docs/layout.md`
- `docs/rendering.md`
- `docs/testing.md`
- `docs/roadmap.md`
