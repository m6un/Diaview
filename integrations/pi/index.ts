/**
 * Diaview Pi Extension
 *
 * Registers a model-callable `diaview` tool that renders Mermaid flowcharts as
 * ANSI terminal diagrams directly in Pi's chat scrollback.
 *
 * The tool pipes Mermaid source text to `diaview --inline --compact -` on stdin and captures
 * the ANSI output. On parse/render errors it falls back cleanly, returning the
 * original Mermaid source wrapped in a fenced code block so the model still has
 * something useful to show.
 *
 * Loading this extension:
 *   pi --extension ./integrations/pi/index.ts
 *
 * See integrations/pi/README.md for full setup and usage details.
 */

import { spawnSync } from "node:child_process";
import type { ExtensionAPI } from "@earendil-works/pi-coding-agent";
import { type Component, Text, truncateToWidth } from "@earendil-works/pi-tui";
import { Type } from "typebox";

// ---------------------------------------------------------------------------
// Parameter schema
// ---------------------------------------------------------------------------

const DiaviewParams = Type.Object({
  mermaid: Type.String({
    description:
      "Mermaid flowchart source text to render as a terminal diagram. " +
      "Must start with a direction declaration such as `flowchart TD` or `graph LR`. " +
      "Supported node shapes: rectangles A[label], rounded B(label), diamonds C{label}, " +
      "circles D((label)), cylinders E[(label)]. " +
      "Supported edges: -->, ---, -.->. Subgraphs and edge labels are supported.",
  }),
});

// ---------------------------------------------------------------------------
// Details type stored with each tool result
// ---------------------------------------------------------------------------

interface DiaviewDetails {
  /** true when diaview ran and produced ANSI output */
  rendered: boolean;
  /** diaview exit code, or null when binary not found */
  exitCode: number | null;
  /** stderr from diaview, if any */
  stderr: string;
  /** line count of the rendered ANSI output, 0 on fallback */
  lineCount: number;
  /**
   * Raw ANSI string from diaview --inline --compact -, stored here so it is never passed
   * through model-facing content (which Pi may sanitize/escape before
   * rendering). renderResult reads this field directly to emit the diagram.
   * Undefined on fallback.
   */
  ansi?: string;
}

// ---------------------------------------------------------------------------
// Custom Component for fixed-layout ANSI diagrams
// ---------------------------------------------------------------------------

class AnsiDiagram implements Component {
  constructor(private readonly ansi: string) {}

  render(width: number): string[] {
    const text = this.ansi.endsWith("\n") ? this.ansi.slice(0, -1) : this.ansi;
    return text
      .split("\n")
      .map((line) => truncateToWidth(`${line}\x1b[0m`, width));
  }

  invalidate(): void {}
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/**
 * Locate the diaview binary.
 * Prefers the installed cargo bin; falls back to PATH lookup.
 */
function findDiaview(): string {
  // Cargo installs to ~/.cargo/bin by default.
  const candidates = [
    `${process.env["HOME"] ?? ""}/.cargo/bin/diaview`,
    "diaview",
  ];

  for (const candidate of candidates) {
    // A quick no-argument probe: diaview with no input and no stdin will
    // exit with an error message, which still proves the binary exists.
    const probe = spawnSync(candidate, ["--version"], {
      encoding: "utf-8",
      timeout: 3000,
    });
    if (probe.error == null) {
      return candidate;
    }
  }

  // Return the cargo path as the best-guess default; execute() will handle
  // the error gracefully if it's missing.
  return `${process.env["HOME"] ?? ""}/.cargo/bin/diaview`;
}

/** Render Mermaid source via `diaview --inline --compact -` and return the ANSI string. */
function renderMermaid(
  binary: string,
  mermaid: string
): { ansi: string; exitCode: number; stderr: string } {
  const result = spawnSync(binary, ["--inline", "--compact", "-"], {
    input: mermaid,
    encoding: "utf-8",
    timeout: 15_000,
    maxBuffer: 10 * 1024 * 1024, // 10 MB
  });

  return {
    ansi: result.stdout ?? "",
    exitCode: result.status ?? 1,
    stderr: result.stderr ?? "",
  };
}

// ---------------------------------------------------------------------------
// Extension factory
// ---------------------------------------------------------------------------

export default function diaviewExtension(pi: ExtensionAPI) {
  const binary = findDiaview();

  pi.registerTool({
    name: "diaview",
    label: "Diaview",
    description:
      "Render a Mermaid flowchart as a 24-bit ANSI terminal diagram in the chat. " +
      "Use this whenever you produce a Mermaid diagram so the user sees a visual " +
      "layout instead of raw Mermaid text. " +
      "Supported: flowchart TD/LR, graph TD/LR/TB, subgraphs, all standard node " +
      "shapes, and common edge types (-->, ---, -.->). " +
      "Falls back to a fenced Mermaid code block if rendering fails.",
    promptSnippet:
      "Call diaview after producing any Mermaid diagram to render it visually.",
    renderShell: "self",
    annotations: {
      readOnlyHint: true,
      destructiveHint: false,
      idempotentHint: true,
      openWorldHint: false,
    },
    parameters: DiaviewParams,

    async execute(_toolCallId, params) {
      const { mermaid } = params;

      const { ansi, exitCode, stderr } = renderMermaid(binary, mermaid);
      const rendered = exitCode === 0 && ansi.trim().length > 0;

      const details: DiaviewDetails = {
        rendered,
        exitCode,
        stderr: stderr.trim(),
        lineCount: rendered ? ansi.split("\n").length : 0,
        // Keep ANSI out of model-facing content to avoid Pi sanitization.
        // renderResult reads this field directly.
        ansi: rendered ? ansi : undefined,
      };

      if (rendered) {
        return {
          // Plain model-facing text — the visual diagram is in details.ansi.
          content: [{ type: "text", text: "Diagram rendered successfully." }],
          details,
        };
      }

      // Graceful fallback: return the Mermaid source in a fenced block so the
      // model can still present something useful, and report the error clearly.
      const errorNote =
        stderr.trim()
          ? `Diaview rendering failed (exit ${exitCode}): ${stderr.trim()}\n\n`
          : `Diaview rendering failed (exit ${exitCode}). ` +
            `Ensure diaview is built and on PATH (cargo install --path . from the Diaview repo).\n\n`;

      const fallback =
        `${errorNote}` +
        "```mermaid\n" +
        mermaid +
        "\n```";

      return {
        content: [{ type: "text", text: fallback }],
        details,
      };
    },

    // -------------------------------------------------------------------------
    // TUI renderers
    // -------------------------------------------------------------------------

    renderCall(_args, theme, context) {
      if (!context.isPartial) {
        return new Text("", 0, 0);
      }
      const text =
        theme.fg("toolTitle", theme.bold("diaview ")) +
        theme.fg("dim", "rendering diagram…");
      return new Text(text, 0, 0);
    },

    renderResult(result, { isPartial }, theme, _context) {
      if (isPartial) {
        return new Text(theme.fg("warning", "Rendering…"), 0, 0);
      }

      const details = result.details as DiaviewDetails | undefined;

      if (!details?.rendered) {
        // Fallback path: show the fenced Mermaid block so the user still sees
        // the diagram source, plus a one-line error label.
        const errorLabel = details?.stderr
          ? details.stderr.split("\n")[0]
          : "diaview binary not found — run: cargo install --path .";
        const content = result.content[0];
        const fallbackText = content?.type === "text" ? content.text : "";
        return new Text(
          theme.fg("warning", "⚠ diaview fallback") +
            theme.fg("dim", ` — ${errorLabel}`) +
            "\n" +
            fallbackText,
          0,
          0
        );
      }

      // Always show the ANSI diagram in the collapsed transcript — this is the
      // core user requirement. Read from details.ansi (not result.content) so
      // the raw escape sequences reach the renderer unmodified; Pi may
      // sanitize/escape model-facing content before rendering it.
      return new AnsiDiagram(details.ansi ?? "");
    },
  });
}
