import { Type } from "typebox";
import { type ExtensionAPI } from "@mariozechner/pi-coding-agent";
import { spawn, spawnSync } from "node:child_process";
import * as path from "node:path";
import * as fs from "node:fs/promises";

export default function (pi: ExtensionAPI) {
  // Tool to spawn an agent in a new worktree
  pi.registerTool({
    name: "spawn_worktree_agent",
    label: "Spawn Worktree Agent",
    description: "Spawn a subagent in a new git worktree to work on a specific task. Use this to delegate parallel workstreams.",
    promptSnippet: "Spawn subagents in parallel worktrees to delegate tasks",
    promptGuidelines: [
      "Use spawn_worktree_agent when the user asks to spawn agents, parallel workstreams, or work on independent modules.",
      "Do not try to implement parallel sub-tasks yourself in the main thread; delegate them using this tool.",
      "You can call this tool multiple times in parallel to spawn multiple agents concurrently."
    ],
    parameters: Type.Object({
      branch: Type.String({ description: "Name of the new branch and worktree" }),
      model: Type.Optional(Type.String({ description: "Model to use for the subagent" })),
      task: Type.String({ description: "The task instruction for the agent" })
    }),
    async execute(toolCallId, params, signal, onUpdate, ctx) {
      const worktreeDir = path.resolve(ctx.cwd, '../Diaview-' + params.branch);
      
      onUpdate?.({ content: [{ type: "text", text: `Creating worktree at ${worktreeDir}...` }] });
      const wtResult = await pi.exec("git", ["worktree", "add", worktreeDir, "-b", params.branch], { signal });
      if (wtResult.code !== 0) {
        throw new Error(`Failed to create worktree: ${wtResult.stderr}`);
      }

      onUpdate?.({ content: [{ type: "text", text: `Spawning agent in ${worktreeDir}...` }] });
      
      const piArgs = ["-p", params.task];
      if (params.model) {
        piArgs.push("--model", params.model);
      }

      const result = await pi.exec("pi", piArgs, { cwd: worktreeDir, signal });
      
      return {
        content: [{ 
          type: "text", 
          text: `Agent finished task in branch ${params.branch}.\nWorktree created at: ${worktreeDir}\n\nReview the changes using \`/lazygit ${params.branch}\`.\n\nAgent Output:\n${result.stdout}\n${result.stderr}` 
        }],
        details: { worktreeDir, branch: params.branch, code: result.code }
      };
    }
  });

  // Command to review worktree changes with lazygit
  pi.registerCommand("lazygit", {
    description: "Open lazygit for a specific worktree branch",
    handler: async (args, ctx) => {
      const branch = args.trim();
      if (!branch) {
        ctx.ui.notify("Please specify a branch name (e.g. /lazygit parser)", "error");
        return;
      }

      const worktreeDir = path.resolve(ctx.cwd, '../Diaview-' + branch);
      try {
        await fs.access(worktreeDir);
      } catch {
        ctx.ui.notify(`Worktree not found at ${worktreeDir}`, "error");
        return;
      }

      if (!ctx.hasUI) {
        ctx.ui.notify("TUI not available", "error");
        return;
      }

      // Run lazygit in the worktree, suspending pi's TUI
      await ctx.ui.custom<number | null>((tui, _theme, _kb, done) => {
        tui.stop();
        process.stdout.write("\x1b[2J\x1b[H");

        const result = spawnSync("lazygit", [], {
          cwd: worktreeDir,
          stdio: "inherit",
          env: process.env,
        });

        tui.start();
        tui.requestRender(true);
        done(result.status);

        return { render: () => [], invalidate: () => {} };
      });
    }
  });
}
