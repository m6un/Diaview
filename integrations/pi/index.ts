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
 *
 * Wide-diagram behavior:
 *   Pi's Component contract requires every returned line to fit within the
 *   supplied `width` columns. When a diagram exceeds the available component
 *   width, each line is clipped to `width` and a deliberate, user-visible
 *   warning line is appended:
 *     ⚠ diagram clipped: N cols wide (terminal: W cols) — enlarge terminal for full diagram
 *   This ensures lines never overflow the terminal viewport while keeping the
 *   clipping explicit and informative to the user.
 */

import { spawn } from "node:child_process";
import type { ExtensionAPI } from "@earendil-works/pi-coding-agent";
import {
  type Component,
  Text,
  truncateToWidth,
  visibleWidth,
} from "@earendil-works/pi-tui";
import { Type } from "typebox";

// ---------------------------------------------------------------------------
// Parameter schema
// ---------------------------------------------------------------------------

export const DiaviewParams = Type.Object({
  mermaid: Type.String({
    description:
      "Mermaid flowchart source text to render as a terminal diagram. " +
      "Must start with a direction declaration such as `flowchart TD` or `graph LR`. " +
      "Supported node shapes: rectangles A[label], rounded B(label), diamonds C{label}, " +
      "circles D((label)), cylinders E[(label)]. " +
      "Supported edges: -->, ---, -.->, edge labels. Subgraphs are supported.",
  }),
});

// ---------------------------------------------------------------------------
// Details type stored with each tool result
// ---------------------------------------------------------------------------

export interface DiaviewDetails {
  /** true when diaview ran and produced ANSI output */
  rendered: boolean;
  /** diaview exit code, or null when binary not found */
  exitCode: number | null;
  /** stderr from diaview, if any */
  stderr: string;
  /** line count of the rendered ANSI output, 0 on fallback */
  lineCount: number;
  /** Set when a resource limit was exceeded (timeout or output overflow) */
  limitHit?: "timeout" | "stdout_overflow" | "stderr_overflow";
  /** Human-readable error message explaining failure / fallback */
  errorMessage?: string;
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

/**
 * AnsiDiagram renders the raw ANSI output from `diaview --inline --compact -`
 * into Pi's component system.
 *
 * Wide-diagram contract:
 *   The Component.render(width) contract strictly requires every returned line
 *   to fit within `width` visible terminal columns. When the diagram exceeds
 *   `width`, lines are clipped to `width` and a deliberate, user-visible
 *   warning line is appended so the clipping is immediately clear to the user.
 */
export class AnsiDiagram implements Component {
  private readonly ansi: string;

  constructor(ansi: string) {
    this.ansi = ansi;
  }

  render(width: number): string[] {
    const text = this.ansi.endsWith("\n") ? this.ansi.slice(0, -1) : this.ansi;
    const lines = text.split("\n").map((line) => `${line}\x1b[0m`);

    // Measure the widest line across the diagram.
    const maxCols = lines.reduce((max, l) => Math.max(max, visibleWidth(l)), 0);

    if (maxCols <= width) {
      // Diagram fits within available width. Apply truncateToWidth to guarantee contract.
      return lines.map((l) => truncateToWidth(l, width));
    }

    // Diagram is wider than the available component width.
    // The Component contract requires every returned line to fit within width.
    // Truncate each diagram line to width and append a deliberate, user-visible warning.
    const warning = truncateToWidth(
      `\x1b[33m⚠ diagram clipped: ${maxCols} cols wide (terminal: ${width} cols) — enlarge terminal for full diagram\x1b[0m`,
      width
    );
    const clippedLines = lines.map((l) => truncateToWidth(l, width));
    return [...clippedLines, warning];
  }

  invalidate(): void {}
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** Bounded subprocess execution result. */
export interface SpawnResult {
  stdout: string;
  stderr: string;
  exitCode: number | null;
  /** Set when the process was killed due to timeout or output limits. */
  limitHit?: "timeout" | "stdout_overflow" | "stderr_overflow";
}

/**
 * Run a process asynchronously with a bounded timeout, bounded output
 * capture, and bounded escalation (SIGTERM followed by SIGKILL if needed).
 *
 * Distinguishes actual timeout from arbitrary external SIGTERM.
 * Rejects only on spawn errors (e.g. ENOENT).
 */
export function spawnBounded(
  bin: string,
  args: readonly string[],
  opts: {
    input?: string;
    timeoutMs: number;
    maxStdoutBytes: number;
    maxStderrBytes: number;
    killGracePeriodMs?: number;
  }
): Promise<SpawnResult> {
  return new Promise((resolve, reject) => {
    let child: ReturnType<typeof spawn>;
    try {
      child = spawn(bin, args, { stdio: ["pipe", "pipe", "pipe"] });
    } catch (err) {
      reject(err);
      return;
    }

    let spawnError: Error | null = null;
    child.on("error", (err) => {
      spawnError = err;
    });

    if (!child.stdout || !child.stderr || !child.stdin) {
      reject(new Error("Failed to initialize subprocess stdio pipes"));
      return;
    }

    const stdout = child.stdout;
    const stderr = child.stderr;
    const stdin = child.stdin;

    const stdoutChunks: Buffer[] = [];
    let stdoutBytes = 0;
    let stdoutOverflow = false;

    const stderrChunks: Buffer[] = [];
    let stderrBytes = 0;
    let stderrOverflow = false;

    let timedOut = false;
    let escalationTimer: NodeJS.Timeout | null = null;
    const graceMs = opts.killGracePeriodMs ?? 1000;

    function terminateChild(): void {
      try {
        child.kill("SIGTERM");
      } catch {
        // Child may have already exited
      }
      if (!escalationTimer) {
        escalationTimer = setTimeout(() => {
          try {
            child.kill("SIGKILL");
          } catch {
            // Child may have already exited
          }
        }, graceMs);
      }
    }

    stdout.on("data", (chunk: Buffer) => {
      if (stdoutOverflow) return;
      stdoutBytes += chunk.length;
      if (stdoutBytes > opts.maxStdoutBytes) {
        stdoutOverflow = true;
        stdout.destroy();
        terminateChild();
      } else {
        stdoutChunks.push(chunk);
      }
    });

    stderr.on("data", (chunk: Buffer) => {
      if (stderrOverflow) return;
      stderrBytes += chunk.length;
      if (stderrBytes > opts.maxStderrBytes) {
        stderrOverflow = true;
        stderr.destroy();
        terminateChild();
      } else {
        stderrChunks.push(chunk);
      }
    });

    const timeoutTimer = setTimeout(() => {
      timedOut = true;
      terminateChild();
    }, opts.timeoutMs);

    child.on("close", (code, signal) => {
      clearTimeout(timeoutTimer);
      if (escalationTimer) {
        clearTimeout(escalationTimer);
      }

      if (spawnError) {
        reject(spawnError);
        return;
      }

      const outStr = Buffer.concat(stdoutChunks).toString("utf8");
      const errStr = Buffer.concat(stderrChunks).toString("utf8");

      let limitHit: SpawnResult["limitHit"];
      if (stdoutOverflow) {
        limitHit = "stdout_overflow";
      } else if (stderrOverflow) {
        limitHit = "stderr_overflow";
      } else if (timedOut) {
        limitHit = "timeout";
      }

      resolve({
        stdout: outStr,
        stderr: errStr,
        exitCode: code ?? (signal ? 1 : 0),
        limitHit,
      });
    });

    // Write input then close stdin so diaview sees EOF.
    if (opts.input != null) {
      stdin.on("error", () => {
        // Ignore EPIPE or similar pipe errors if child exits early
      });
      stdin.write(opts.input, () => {
        stdin.end();
      });
    } else {
      stdin.end();
    }
  });
}

/**
 * Probe one binary candidate asynchronously.
 * Returns true if the probe exits with code 0.
 */
export async function probeBinary(candidate: string): Promise<boolean> {
  try {
    const result = await spawnBounded(candidate, ["--version"], {
      timeoutMs: 3_000,
      maxStdoutBytes: 4 * 1024,
      maxStderrBytes: 4 * 1024,
    });
    return result.exitCode === 0;
  } catch {
    // spawn error (ENOENT, etc.)
    return false;
  }
}

/**
 * Locate the diaview binary.
 * Probes ~/.cargo/bin/diaview first, then falls back to PATH lookup.
 * Returns the path of the first binary that launches successfully,
 * or the cargo path as a default when neither is found.
 */
export async function findDiaview(candidates?: string[]): Promise<string> {
  const cargoBin = `${process.env["HOME"] ?? ""}/.cargo/bin/diaview`;
  const candidateList = candidates ?? [cargoBin, "diaview"];

  for (const candidate of candidateList) {
    if (await probeBinary(candidate)) {
      return candidate;
    }
  }

  // Best-guess fallback; execute() will produce a human-readable error.
  return candidateList[0] ?? cargoBin;
}

let cachedBinary: string | null = null;

/** Get or discover the diaview binary path. Directly caches the findDiaview result. */
export async function getDiaviewBinary(): Promise<string> {
  if (cachedBinary != null) {
    return cachedBinary;
  }
  cachedBinary = await findDiaview();
  return cachedBinary;
}

/** Reset cached binary path (used in unit tests). */
export function resetDiaviewBinaryCache(): void {
  cachedBinary = null;
}

export interface RenderResult {
  ansi: string;
  exitCode: number | null;
  stderr: string;
  limitHit?: "timeout" | "stdout_overflow" | "stderr_overflow";
  notFound?: boolean;
}

/** Render Mermaid source via `diaview --inline --compact -` and return the ANSI string. */
export async function renderMermaid(
  binary: string,
  mermaid: string,
  opts?: {
    timeoutMs?: number;
    maxStdoutBytes?: number;
    maxStderrBytes?: number;
  }
): Promise<RenderResult> {
  try {
    const result = await spawnBounded(binary, ["--inline", "--compact", "-"], {
      input: mermaid,
      timeoutMs: opts?.timeoutMs ?? 15_000,
      maxStdoutBytes: opts?.maxStdoutBytes ?? 10 * 1024 * 1024, // 10 MB
      maxStderrBytes: opts?.maxStderrBytes ?? 64 * 1024, // 64 KB
    });

    return {
      ansi: result.stdout,
      exitCode: result.exitCode,
      stderr: result.stderr,
      limitHit: result.limitHit,
    };
  } catch (err: unknown) {
    const isEnoent =
      err instanceof Error && (err as NodeJS.ErrnoException).code === "ENOENT";
    return {
      ansi: "",
      exitCode: null,
      stderr: isEnoent ? `diaview binary not found at '${binary}'` : String(err),
      notFound: isEnoent,
    };
  }
}

/** Format human-readable error notes and short labels for fallback display. */
export function describeError(renderRes: {
  exitCode: number | null;
  stderr: string;
  limitHit?: "timeout" | "stdout_overflow" | "stderr_overflow";
  notFound?: boolean;
  binary?: string;
}): { errorNote: string; errorLabel: string } {
  if (renderRes.notFound) {
    return {
      errorNote: `Diaview binary not found at '${renderRes.binary ?? "diaview"}'. Ensure diaview is built and on PATH (cargo install --path . from the Diaview repo).`,
      errorLabel: "diaview binary not found — run: cargo install --path .",
    };
  }

  if (renderRes.limitHit === "timeout") {
    return {
      errorNote: "Diaview rendering timed out (exceeded 15s limit).",
      errorLabel: "rendering timed out (> 15s)",
    };
  }

  if (renderRes.limitHit === "stdout_overflow") {
    return {
      errorNote: "Diaview output exceeded maximum allowed limit (10 MB).",
      errorLabel: "output limit exceeded (> 10 MB)",
    };
  }

  if (renderRes.limitHit === "stderr_overflow") {
    return {
      errorNote: "Diaview error output exceeded maximum allowed limit (64 KB).",
      errorLabel: "error log limit exceeded (> 64 KB)",
    };
  }

  const stderrTrimmed = renderRes.stderr.trim();
  if (stderrTrimmed) {
    const firstLine = stderrTrimmed.split("\n")[0] ?? stderrTrimmed;
    return {
      errorNote: `Diaview rendering failed (exit ${renderRes.exitCode ?? 1}): ${stderrTrimmed}`,
      errorLabel: firstLine,
    };
  }

  return {
    errorNote: `Diaview rendering failed (exit ${renderRes.exitCode ?? 1}).`,
    errorLabel: `render failed (exit ${renderRes.exitCode ?? 1})`,
  };
}

/** Standalone execution handler for the diaview tool. */
export async function executeDiaview(
  params: { mermaid: string },
  opts?: {
    binary?: string;
    timeoutMs?: number;
    maxStdoutBytes?: number;
    maxStderrBytes?: number;
  }
): Promise<{
  content: Array<{ type: "text"; text: string }>;
  details: DiaviewDetails;
}> {
  const { mermaid } = params;
  const binary = opts?.binary ?? (await getDiaviewBinary());

  const renderRes = await renderMermaid(binary, mermaid, {
    timeoutMs: opts?.timeoutMs,
    maxStdoutBytes: opts?.maxStdoutBytes,
    maxStderrBytes: opts?.maxStderrBytes,
  });

  const rendered =
    renderRes.exitCode === 0 &&
    !renderRes.limitHit &&
    renderRes.ansi.trim().length > 0;

  const { errorNote, errorLabel } = rendered
    ? { errorNote: "", errorLabel: "" }
    : describeError({ ...renderRes, binary });

  const textForCount = renderRes.ansi.endsWith("\n")
    ? renderRes.ansi.slice(0, -1)
    : renderRes.ansi;

  const details: DiaviewDetails = {
    rendered,
    exitCode: renderRes.exitCode,
    stderr: renderRes.stderr.trim(),
    lineCount: rendered ? textForCount.split("\n").length : 0,
    limitHit: renderRes.limitHit,
    errorMessage: errorLabel || undefined,
    // Keep ANSI out of model-facing content to avoid Pi sanitization.
    // renderResult reads this field directly.
    ansi: rendered ? renderRes.ansi : undefined,
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
  const fallback =
    `${errorNote}\n\n` +
    "```mermaid\n" +
    mermaid +
    "\n```";

  return {
    content: [{ type: "text", text: fallback }],
    details,
  };
}

// ---------------------------------------------------------------------------
// Extension factory (synchronous to comply with Pi extension lifecycle)
// ---------------------------------------------------------------------------

export default function diaviewExtension(pi: ExtensionAPI): void {
  pi.registerTool({
    name: "diaview",
    label: "Diaview",
    description:
      "Render a Mermaid flowchart as a 24-bit ANSI terminal diagram in the chat. " +
      "Use this whenever you produce a Mermaid diagram so the user sees a visual " +
      "layout instead of raw Mermaid text. " +
      "Supported: flowchart TD/LR, graph TD/LR/TB, subgraphs, all standard node " +
      "shapes, and common edge types (-->, ---, -.->, edge labels). " +
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
      const binary = await getDiaviewBinary();
      return executeDiaview(params, { binary });
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
        // the diagram source, plus an informative error label.
        const errorLabel =
          details?.errorMessage ??
          (details?.stderr
            ? details.stderr.split("\n")[0]
            : "diaview rendering failed");
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
      //
      // AnsiDiagram.render() ensures every returned line fits within width,
      // clipping if necessary and displaying a visible warning when clipped.
      return new AnsiDiagram(details.ansi ?? "");
    },
  });
}
