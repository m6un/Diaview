# Diaview Pi Extension

A Pi coding-agent extension that renders Mermaid flowcharts as 24-bit ANSI
terminal diagrams directly in the chat scrollback using
[Diaview](../../README.md).

## What it does

Registers a model-callable **`diaview`** tool. When the model produces a
Mermaid diagram it can call this tool with the diagram source; the extension
pipes it through `diaview --inline --compact -` and returns the ANSI-rendered output so
the user sees a visual layout in the transcript by default — no expand or toggle
required.

On any error (binary not found, parse failure, unsupported syntax) the tool
falls back cleanly: it returns the original Mermaid source in a fenced code
block along with a human-readable error note, and shows a ⚠ label in the
Pi TUI.

---

## Prerequisites

### 1 — Build and install Diaview

The extension shells out to the `diaview` binary. Build and install it from
the repository root:

```bash
cargo install --path .
```

This places `diaview` in `~/.cargo/bin/`. Make sure that directory is on your
`PATH` (Rust's `rustup` installer adds it automatically).

Verify:

```bash
diaview --version
printf 'flowchart TD\n  A --> B\n' | diaview --inline --compact -
```

### 2 — Nerd Fonts

Diaview renders engineering stencil icons from **Nerd Fonts v3**. Configure
your terminal emulator to use a patched Nerd Font (e.g. *JetBrainsMono Nerd
Font*), or install the symbols-only fallback:

```bash
# macOS (Homebrew)
brew install --cask font-symbols-only-nerd-font
```

---

## Loading the extension

Pi uses `jiti` for TypeScript; no compile step is needed for any method below.

### Option A — One-off: `--extension` flag

Load for a single Pi session from the command line:

```bash
pi --extension ./integrations/pi/index.ts
```

### Option B — User-global: copy or symlink into your agent extensions directory

Pi automatically discovers extensions under `~/.pi/agent/extensions/`
(source: Pi [configuration docs](~/.local/lib/node_modules/@earendil-works/pi-coding-agent/docs/configuration.md)).

```bash
# symlink — the extension stays in the repo; Pi picks it up on every start
ln -s "$(pwd)/integrations/pi" ~/.pi/agent/extensions/diaview
```

### Option C — Project-scoped: drop into `.pi/extensions/`

Pi automatically discovers extensions under `.pi/extensions/` for the current
project (after project trust is granted):

```bash
mkdir -p .pi/extensions
cp -r integrations/pi .pi/extensions/diaview
```

### Option D — `extensions` in `settings.json`

Add an explicit path to `~/.pi/agent/settings.json` (user-level) or
`.pi/settings.json` (project-level). Relative paths in project settings
resolve from the `.pi/` directory:

```json
{
  "extensions": ["../integrations/pi/index.ts"]
}
```

Run `/reload` after editing `settings.json` manually.

---

## Using the tool

Once loaded, the model can call the `diaview` tool directly. Example prompt:

> "Draw an auth flow diagram and render it."

The model produces a Mermaid flowchart, then calls `diaview`; the ANSI diagram
appears inline in the chat transcript without any expand/toggle step.

---

## Supported Mermaid subset

| Feature | Supported |
|---|---|
| `flowchart TD / LR` | ✓ |
| `graph TD / LR / TB` | ✓ |
| Rectangles `A[label]` | ✓ |
| Rounded `B(label)` | ✓ |
| Diamonds `C{label}` | ✓ |
| Circles `D((label))` | ✓ |
| Cylinders `E[(label)]` | ✓ |
| Solid arrows `-->` | ✓ |
| Dashed arrows `-.->` | ✓ |
| Edge labels `-->|text|` | ✓ |
| Subgraphs | ✓ |
| Sequence / Gantt / other | ✗ (clean fallback to fenced block) |

See [docs/parser.md](../../docs/parser.md) for the full parsing grammar.

---

## Wide-diagram behavior

Pi's component contract requires every rendered line to fit within the available terminal width (`Component.render(width)`).

When a diagram is wider than the available panel:
- Each line is clipped to `width` using ANSI-aware boundary truncation.
- A deliberate, user-visible warning line is appended:
  ```text
  ⚠ diagram clipped: 120 cols wide (terminal: 80 cols) — enlarge terminal for full diagram
  ```
- All output lines strictly honor the terminal width limit.

---

## Fallback behaviour

| Situation | What the tool returns |
|---|---|
| `diaview` renders successfully | ANSI diagram shown directly in transcript |
| Parse error or unsupported syntax | ⚠ label + fenced Mermaid block + stderr |
| Binary not found | ⚠ label + fenced Mermaid block + install hint |
| Execution timeout (> 15s) | ⚠ label (`rendering timed out`) + fenced Mermaid block |
| Output limit exceeded (> 10 MB stdout) | ⚠ label (`output limit exceeded`) + fenced Mermaid block |
| Error log limit exceeded (> 64 KB stderr) | ⚠ label (`error log limit exceeded`) + fenced Mermaid block |

The model always receives the tool result as text content, so it can still
refer to the diagram even when rendering fails.

---

## Development & testing

Pi loads TypeScript extensions directly via `jiti`; no build step is needed for runtime.

### Running extension tests

The extension includes automated unit tests covering binary discovery, subprocess bounding, timeout escalation, output limits, error fallback, and wide-diagram contract compliance.

```bash
cd integrations/pi
npm ci
npm run check   # Type-check with tsc --noEmit
npm test        # Run unit tests via node --test
```

### Manual smoke-testing

```bash
# Verify diaview produces output on stdin
printf 'flowchart TD\n  A[Client] --> B{Auth}\n  B -->|ok| C[API]\n' | diaview --inline --compact -

# Load extension for one Pi session
pi --extension ./integrations/pi/index.ts

# Run Diaview's Rust test suite (from repo root)
cargo test
cargo check
```

Pi extension API reference:
`~/.local/lib/node_modules/@earendil-works/pi-coding-agent/docs/extensions.md`
