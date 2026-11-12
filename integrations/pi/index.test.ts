/**
 * Diaview Pi Extension – unit tests
 *
 * Run with:   npm test
 * Requires:   Node >= 22 (node:test built-in with --experimental-strip-types)
 *
 * Coverage:
 *   1. cargo-path and PATH discovery
 *   2. Success: valid Mermaid renders ANSI output and returns expected details
 *   3. Parse failure: invalid Mermaid falls back with exit code and stderr
 *   4. Missing binary: ENOENT handled gracefully without unhandled rejection
 *   5. Timeout: bounded escalation kills process, distinguishes timeout from SIGTERM
 *   6. Output limits: stdout and stderr overflows terminate process and surface in fallback
 *   7. Wide output: AnsiDiagram strictly honors width contract and shows clipping warning
 */

import { after, before, describe, it } from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { visibleWidth } from "@earendil-works/pi-tui";
import {
  AnsiDiagram,
  describeError,
  executeDiaview,
  findDiaview,
  probeBinary,
  spawnBounded,
} from "./index.ts";

let testDir = "";

before(() => {
  testDir = fs.mkdtempSync(path.join(os.tmpdir(), "diaview-test-"));
});

after(() => {
  if (testDir) {
    fs.rmSync(testDir, { recursive: true, force: true });
  }
});

/** Create a small executable node script with shebang and 0o755 permissions. */
function createMockExecutable(code: string): string {
  const filePath = path.join(
    testDir,
    `mock-${Math.random().toString(36).slice(2)}.js`
  );
  fs.writeFileSync(filePath, `#!${process.execPath}\n${code}\n`, { mode: 0o755 });
  return filePath;
}

// ---------------------------------------------------------------------------
// 1. Cargo-path and PATH discovery
// ---------------------------------------------------------------------------

describe("Binary discovery", () => {
  it("probeBinary returns true for an existing executable", async () => {
    const okBin = createMockExecutable("process.exit(0);");
    const ok = await probeBinary(okBin);
    assert.equal(ok, true);
  });

  it("probeBinary returns false for a non-existent binary", async () => {
    const ok = await probeBinary("/no/such/binary/diaview-xyzzy-12345");
    assert.equal(ok, false);
  });

  it("probeBinary returns false when candidate exits non-zero", async () => {
    const failBin = createMockExecutable("process.exit(1);");
    const ok = await probeBinary(failBin);
    assert.equal(ok, false);
  });

  it("findDiaview selects first candidate when cargo path probes successfully", async () => {
    const goodBin = createMockExecutable("process.exit(0);");
    const chosen = await findDiaview([goodBin, "/no/such/binary"]);
    assert.equal(chosen, goodBin);
  });

  it("findDiaview falls through to second candidate when first candidate is missing", async () => {
    const goodBin = createMockExecutable("process.exit(0);");
    const chosen = await findDiaview([
      "/no/such/binary-1",
      goodBin,
      "/no/such/binary-2",
    ]);
    assert.equal(chosen, goodBin);
  });

  it("findDiaview falls back to candidate[0] when neither candidate exists", async () => {
    const chosen = await findDiaview([
      "/no/such/cargo/diaview",
      "/no/such/path/diaview",
    ]);
    assert.equal(chosen, "/no/such/cargo/diaview");
  });
});

// ---------------------------------------------------------------------------
// 2. spawnBounded: stream bounding, timeout, and bounded escalation
// ---------------------------------------------------------------------------

describe("spawnBounded", () => {
  it("resolves successfully with stdout and exitCode 0", async () => {
    const echoBin = createMockExecutable(
      'process.stdout.write("hello-stdout"); process.exit(0);'
    );
    const res = await spawnBounded(echoBin, [], {
      timeoutMs: 5_000,
      maxStdoutBytes: 1024,
      maxStderrBytes: 1024,
    });
    assert.equal(res.exitCode, 0);
    assert.equal(res.stdout, "hello-stdout");
    assert.equal(res.limitHit, undefined);
  });

  it("pipes stdin into the child process", async () => {
    const catBin = createMockExecutable(
      'let b=""; process.stdin.on("data", c => b += c); process.stdin.on("end", () => process.stdout.write(b));'
    );
    const res = await spawnBounded(catBin, [], {
      input: "piped-input-data",
      timeoutMs: 5_000,
      maxStdoutBytes: 1024,
      maxStderrBytes: 1024,
    });
    assert.equal(res.exitCode, 0);
    assert.equal(res.stdout, "piped-input-data");
  });

  it("kills process and sets limitHit=stdout_overflow when stdout exceeds limit", async () => {
    const overflowBin = createMockExecutable(
      'process.stdout.write("x".repeat(2048)); setTimeout(() => {}, 5000);'
    );
    const res = await spawnBounded(overflowBin, [], {
      timeoutMs: 5_000,
      maxStdoutBytes: 100,
      maxStderrBytes: 1024,
    });
    assert.equal(res.limitHit, "stdout_overflow");
  });

  it("kills process and sets limitHit=stderr_overflow when stderr exceeds limit", async () => {
    const overflowBin = createMockExecutable(
      'process.stderr.write("e".repeat(2048)); setTimeout(() => {}, 5000);'
    );
    const res = await spawnBounded(overflowBin, [], {
      timeoutMs: 5_000,
      maxStdoutBytes: 1024,
      maxStderrBytes: 100,
    });
    assert.equal(res.limitHit, "stderr_overflow");
  });

  it("kills process and sets limitHit=timeout when timeout expires", async () => {
    const sleepBin = createMockExecutable("setTimeout(() => {}, 10_000);");
    const res = await spawnBounded(sleepBin, [], {
      timeoutMs: 150,
      maxStdoutBytes: 1024,
      maxStderrBytes: 1024,
    });
    assert.equal(res.limitHit, "timeout");
  });

  it("escalates to SIGKILL if child ignores SIGTERM on timeout", async () => {
    const ignoreSigtermBin = createMockExecutable(
      'process.on("SIGTERM", () => {}); setInterval(() => {}, 1000);'
    );
    const res = await spawnBounded(ignoreSigtermBin, [], {
      timeoutMs: 150,
      killGracePeriodMs: 200,
      maxStdoutBytes: 1024,
      maxStderrBytes: 1024,
    });
    assert.equal(res.limitHit, "timeout");
  });

  it("distinguishes actual timeout from arbitrary external SIGTERM", async () => {
    const termBin = createMockExecutable("process.kill(process.pid, 'SIGTERM');");
    const res = await spawnBounded(termBin, [], {
      timeoutMs: 5_000,
      maxStdoutBytes: 1024,
      maxStderrBytes: 1024,
    });
    // Because timeoutTimer did not fire, limitHit must NOT be "timeout"
    assert.notEqual(res.limitHit, "timeout");
    assert.notEqual(res.exitCode, 0);
  });

  it("rejects when binary is missing (ENOENT)", async () => {
    await assert.rejects(
      () =>
        spawnBounded("/nonexistent/binary/path-xyzzy", [], {
          timeoutMs: 1000,
          maxStdoutBytes: 1024,
          maxStderrBytes: 1024,
        }),
      (err: unknown) => {
        assert.ok(err instanceof Error);
        assert.equal((err as NodeJS.ErrnoException).code, "ENOENT");
        return true;
      }
    );
  });
});

// ---------------------------------------------------------------------------
// 3. Error descriptions and fallback formatting
// ---------------------------------------------------------------------------

describe("describeError", () => {
  it("formats missing binary error with installation hint", () => {
    const { errorNote, errorLabel } = describeError({
      exitCode: null,
      stderr: "not found",
      notFound: true,
      binary: "/test/diaview",
    });
    assert.ok(errorNote.includes("Diaview binary not found at '/test/diaview'"));
    assert.ok(errorNote.includes("cargo install --path ."));
    assert.ok(errorLabel.includes("diaview binary not found"));
  });

  it("formats timeout error", () => {
    const { errorNote, errorLabel } = describeError({
      exitCode: 1,
      stderr: "",
      limitHit: "timeout",
    });
    assert.ok(errorNote.includes("timed out"));
    assert.ok(errorLabel.includes("timed out"));
  });

  it("formats stdout overflow error", () => {
    const { errorNote, errorLabel } = describeError({
      exitCode: 1,
      stderr: "",
      limitHit: "stdout_overflow",
    });
    assert.ok(errorNote.includes("output exceeded"));
    assert.ok(errorLabel.includes("output limit exceeded"));
  });

  it("formats stderr overflow error", () => {
    const { errorNote, errorLabel } = describeError({
      exitCode: 1,
      stderr: "",
      limitHit: "stderr_overflow",
    });
    assert.ok(errorNote.includes("error output exceeded"));
    assert.ok(errorLabel.includes("error log limit exceeded"));
  });

  it("formats parse failure with first line of stderr", () => {
    const { errorNote, errorLabel } = describeError({
      exitCode: 1,
      stderr: "Parse error: line 2: invalid syntax\nMore details...",
    });
    assert.ok(errorNote.includes("Parse error: line 2: invalid syntax"));
    assert.equal(errorLabel, "Parse error: line 2: invalid syntax");
  });
});

// ---------------------------------------------------------------------------
// 4. Execution integration: success, parse failure, missing binary, limits
// ---------------------------------------------------------------------------

describe("executeDiaview", () => {
  it("handles missing binary gracefully without throwing unhandled rejection", async () => {
    const res = await executeDiaview(
      { mermaid: "flowchart TD\n  A --> B" },
      { binary: "/no/such/diaview/binary-12345" }
    );
    assert.equal(res.details.rendered, false);
    assert.equal(res.details.exitCode, null);
    assert.equal(res.details.limitHit, undefined);
    assert.equal(
      res.details.errorMessage,
      "diaview binary not found — run: cargo install --path ."
    );
    assert.ok(
      res.content[0]?.text.includes(
        "Diaview binary not found at '/no/such/diaview/binary-12345'."
      )
    );
    assert.ok(res.content[0]?.text.includes("cargo install --path ."));
    assert.ok(
      res.content[0]?.text.includes("```mermaid\nflowchart TD\n  A --> B\n```")
    );
  });

  it("renders valid Mermaid when diaview binary succeeds", async () => {
    const ansiSample = "\x1b[38;2;100;150;255m[Client] -> [Server]\x1b[0m\n";
    const successBin = createMockExecutable(
      `process.stdout.write(${JSON.stringify(ansiSample)});\nprocess.exit(0);`
    );
    const res = await executeDiaview(
      { mermaid: "flowchart TD\n  Client --> Server" },
      { binary: successBin }
    );
    assert.equal(res.details.rendered, true);
    assert.equal(res.details.exitCode, 0);
    assert.equal(res.details.limitHit, undefined);
    assert.equal(res.details.lineCount, 1);
    assert.equal(res.details.ansi, ansiSample);
    assert.equal(res.details.errorMessage, undefined);
    assert.equal(res.content[0]?.text, "Diagram rendered successfully.");
  });

  it("handles parse failure by returning fenced code block and stderr error details", async () => {
    const stderrMsg = "Parse error: line 2: unexpected token '??'";
    const failBin = createMockExecutable(
      `process.stderr.write(${JSON.stringify(stderrMsg + "\n")});\nprocess.exit(1);`
    );
    const res = await executeDiaview(
      { mermaid: "flowchart TD\n  A ?? B" },
      { binary: failBin }
    );
    assert.equal(res.details.rendered, false);
    assert.equal(res.details.exitCode, 1);
    assert.equal(res.details.limitHit, undefined);
    assert.equal(res.details.stderr, stderrMsg);
    assert.equal(res.details.errorMessage, stderrMsg);
    assert.ok(
      res.content[0]?.text.includes(
        `Diaview rendering failed (exit 1): ${stderrMsg}`
      )
    );
    assert.ok(
      res.content[0]?.text.includes("```mermaid\nflowchart TD\n  A ?? B\n```")
    );
  });

  it("surfaces timeout in fallback and details", async () => {
    const timeoutBin = createMockExecutable("setTimeout(() => {}, 60_000);");
    const res = await executeDiaview(
      { mermaid: "flowchart TD\n  A --> B" },
      { binary: timeoutBin, timeoutMs: 100 }
    );
    assert.equal(res.details.rendered, false);
    assert.equal(res.details.limitHit, "timeout");
    assert.equal(res.details.errorMessage, "rendering timed out (> 15s)");
    assert.ok(
      res.content[0]?.text.includes(
        "Diaview rendering timed out (exceeded 15s limit)."
      )
    );
    assert.ok(
      res.content[0]?.text.includes("```mermaid\nflowchart TD\n  A --> B\n```")
    );
  });

  it("surfaces stdout overflow in fallback and details", async () => {
    const stdoutBin = createMockExecutable(
      'process.stdout.write("x".repeat(2048)); setTimeout(() => {}, 10_000);'
    );
    const res = await executeDiaview(
      { mermaid: "flowchart TD\n  A --> B" },
      { binary: stdoutBin, maxStdoutBytes: 50 }
    );
    assert.equal(res.details.rendered, false);
    assert.equal(res.details.limitHit, "stdout_overflow");
    assert.equal(res.details.errorMessage, "output limit exceeded (> 10 MB)");
    assert.ok(
      res.content[0]?.text.includes(
        "Diaview output exceeded maximum allowed limit (10 MB)."
      )
    );
    assert.ok(
      res.content[0]?.text.includes("```mermaid\nflowchart TD\n  A --> B\n```")
    );
  });

  it("surfaces stderr overflow in fallback and details", async () => {
    const stderrBin = createMockExecutable(
      'process.stderr.write("e".repeat(2048)); setTimeout(() => {}, 10_000);'
    );
    const res = await executeDiaview(
      { mermaid: "flowchart TD\n  A --> B" },
      { binary: stderrBin, maxStderrBytes: 50 }
    );
    assert.equal(res.details.rendered, false);
    assert.equal(res.details.limitHit, "stderr_overflow");
    assert.equal(
      res.details.errorMessage,
      "error log limit exceeded (> 64 KB)"
    );
    assert.ok(
      res.content[0]?.text.includes(
        "Diaview error output exceeded maximum allowed limit (64 KB)."
      )
    );
    assert.ok(
      res.content[0]?.text.includes("```mermaid\nflowchart TD\n  A --> B\n```")
    );
  });
});

// ---------------------------------------------------------------------------
// 5. Component.render(width): contract compliance and wide-diagram behavior
// ---------------------------------------------------------------------------

describe("AnsiDiagram component", () => {
  it("renders diagram lines fitting within width without warning line", () => {
    const diagram = new AnsiDiagram("line1\nline2\n");
    const lines = diagram.render(20);

    // 2 diagram lines, each <= 20 cols
    assert.equal(lines.length, 2);
    for (const line of lines) {
      assert.ok(
        visibleWidth(line) <= 20,
        `line exceeds width: ${visibleWidth(line)}`
      );
      assert.ok(
        !line.includes("clipped"),
        "unexpected clipping warning for fitting diagram"
      );
    }
  });

  it("honors width contract when diagram is wider than width (no line wider than width)", () => {
    // 60-column line rendered with width=40
    const wideContent = "X".repeat(60);
    const diagram = new AnsiDiagram(wideContent);
    const lines = diagram.render(40);

    // Every single line returned must be <= width (40)
    for (const line of lines) {
      const w = visibleWidth(line);
      assert.ok(
        w <= 40,
        `Contract violation: line visibleWidth ${w} exceeds available width 40: ${JSON.stringify(
          line
        )}`
      );
    }

    // Must include the deliberate user-visible warning line
    const lastLine = lines[lines.length - 1]!;
    assert.ok(
      lastLine.includes("clipped"),
      `expected clipping warning, got: ${JSON.stringify(lastLine)}`
    );
    assert.ok(
      lastLine.includes("60 cols"),
      `expected original width in warning, got: ${JSON.stringify(lastLine)}`
    );
  });

  it("strictly bounds all lines even in very narrow terminal widths", () => {
    const wideContent = "X".repeat(80);
    const diagram = new AnsiDiagram(wideContent);
    const lines = diagram.render(15);
    for (const line of lines) {
      assert.ok(visibleWidth(line) <= 15);
    }
  });

  it("strips trailing newline before splitting lines", () => {
    const diagram = new AnsiDiagram("single-line\n");
    const lines = diagram.render(80);
    assert.equal(lines.length, 1);
  });

  it("preserves ANSI color sequences while strictly truncating within width", () => {
    const colored = "\x1b[32m" + "A".repeat(40) + "\x1b[0m";
    const diagram = new AnsiDiagram(colored);
    const lines = diagram.render(20);

    for (const line of lines) {
      assert.ok(visibleWidth(line) <= 20);
    }
    const lastLine = lines[lines.length - 1]!;
    assert.ok(lastLine.includes("clipped"));
  });
});
