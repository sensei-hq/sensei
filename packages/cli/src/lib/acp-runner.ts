/**
 * ACP runner abstraction for benchmark execution.
 *
 * Each runner knows how to:
 *  - detect whether the ACP is installed
 *  - invoke the ACP non-interactively with a task prompt
 *  - parse the output for token usage
 */

import { readFile } from "fs/promises";
import { join } from "path";

// ── Types ────────────────────────────────────────────────────────────────────

export interface AcpSession {
  /** Direct input tokens (non-cached). */
  inputTokens: number;
  /** Cache creation tokens (first-time context). */
  cacheCreationTokens: number;
  /** Cache read tokens (reused context). */
  cacheReadTokens: number;
  /** Total context = input + cacheCreation + cacheRead. */
  totalContextTokens: number;
  /** Total output tokens for the session. */
  outputTokens: number;
  /** Cost in USD. */
  costUsd: number;
  /** Number of agentic turns taken. */
  numTurns: number;
  /** Number of tool calls made. */
  toolCalls: number;
  /** Files read via Read/Bash(cat) tool calls. */
  filesRead: string[];
  /** Files written/edited via Write/Edit tool calls. */
  filesWritten: string[];
  /** MCP tool calls (sensei search, get_symbol, etc.). */
  mcpCalls: string[];
  /** Exit code of the ACP process. */
  exitCode: number;
  /** Raw stdout (stream-json or text). */
  rawOutput: string;
}

export interface AcpRunner {
  readonly id: string;
  readonly name: string;
  detect(): Promise<boolean>;
  /** Run a task file and return session metrics. */
  runTask(taskPath: string, cwd: string): Promise<AcpSession>;
}

// ── Helpers ───────────────────────────────────────────────────────────────────

function whichExists(name: string): boolean {
  const PATH = process.env.PATH ?? "";
  return PATH.split(":").some((dir) => {
    try {
      return Bun.file(`${dir}/${name}`).size > 0;
    } catch {
      return false;
    }
  });
}

async function spawnCapture(
  cmd: string[],
  opts: { cwd: string; stdin?: string },
): Promise<{ stdout: string; stderr: string; exitCode: number }> {
  const proc = Bun.spawn(cmd, {
    cwd: opts.cwd,
    stdin: opts.stdin ? new TextEncoder().encode(opts.stdin) : "ignore",
    stdout: "pipe",
    stderr: "pipe",
  });
  const [stdout, stderr] = await Promise.all([
    new Response(proc.stdout).text(),
    new Response(proc.stderr).text(),
  ]);
  const exitCode = await proc.exited;
  return { stdout, stderr, exitCode };
}

// ── Claude Code runner ────────────────────────────────────────────────────────

/**
 * Parses claude's `--output-format stream-json` output.
 * Each line is a JSON event; the final `result` event contains usage stats.
 */
function parseClaudeStreamJson(output: string): Omit<AcpSession, "exitCode" | "rawOutput"> {
  let inputTokens = 0;
  let cacheCreationTokens = 0;
  let cacheReadTokens = 0;
  let outputTokens = 0;
  let costUsd = 0;
  let numTurns = 0;
  let toolCalls = 0;
  const filesRead: string[] = [];
  const filesWritten: string[] = [];
  const mcpCalls: string[] = [];

  for (const line of output.split("\n")) {
    const trimmed = line.trim();
    if (!trimmed) continue;
    try {
      const event = JSON.parse(trimmed) as Record<string, unknown>;

      if (event.type === "result") {
        const usage = event.usage as Record<string, number> | undefined;
        inputTokens = usage?.input_tokens ?? 0;
        cacheCreationTokens = usage?.cache_creation_input_tokens ?? 0;
        cacheReadTokens = usage?.cache_read_input_tokens ?? 0;
        outputTokens = usage?.output_tokens ?? 0;
        costUsd = (event.total_cost_usd as number) ?? 0;
        numTurns = (event.num_turns as number) ?? 0;
      }

      // Parse tool_use blocks from assistant messages
      if (event.type === "assistant") {
        const content = (event.message as { content?: unknown[] })?.content ?? [];
        for (const block of content) {
          const b = block as { type?: string; name?: string; input?: Record<string, unknown> };
          if (b.type !== "tool_use") continue;
          toolCalls++;

          const name = b.name ?? "";
          const input = b.input ?? {};

          // File reads: Read tool, or Bash with cat/head/tail
          if (name === "Read" && input.file_path) {
            filesRead.push(String(input.file_path));
          } else if (name === "Bash" && typeof input.command === "string") {
            const cmd = input.command;
            if (/\b(cat|head|tail|less)\s/.test(cmd)) {
              const pathMatch = cmd.match(/(?:cat|head|tail|less)\s+["']?([^\s"'|>]+)/);
              if (pathMatch) filesRead.push(pathMatch[1]);
            }
          }

          // File writes: Write, Edit tools
          if ((name === "Write" || name === "Edit") && input.file_path) {
            filesWritten.push(String(input.file_path));
          }

          // MCP tool calls (sensei tools start with mcp__sensei__ or are named search, get_symbol, etc.)
          if (name.startsWith("mcp__") || name === "search" || name === "get_symbol" || name === "context_pack" || name === "get_session_context" || name === "load_context") {
            mcpCalls.push(name);
          }
        }
      }
    } catch {
      // Non-JSON lines — skip
    }
  }

  const totalContextTokens = inputTokens + cacheCreationTokens + cacheReadTokens;
  return {
    inputTokens, cacheCreationTokens, cacheReadTokens, totalContextTokens,
    outputTokens, costUsd, numTurns, toolCalls,
    filesRead: [...new Set(filesRead)],
    filesWritten: [...new Set(filesWritten)],
    mcpCalls,
  };
}

export class ClaudeRunner implements AcpRunner {
  readonly id = "claude";
  readonly name = "Claude Code";

  async detect(): Promise<boolean> {
    return whichExists("claude");
  }

  async runTask(taskPath: string, cwd: string): Promise<AcpSession> {
    const prompt = await readFile(taskPath, "utf-8");

    // --verbose: required for stream-json to include usage data.
    // --no-session-persistence: avoid polluting user's session history.
    const { stdout, exitCode } = await spawnCapture(
      [
        "claude", "-p", prompt,
        "--output-format", "stream-json",
        "--verbose",
        "--no-session-persistence",
        "--plugin-dir", join(cwd, ".claude", "no-plugins"),
      ],
      { cwd },
    );

    const metrics = parseClaudeStreamJson(stdout);
    return { ...metrics, exitCode, rawOutput: stdout };
  }
}

// ── Registry ──────────────────────────────────────────────────────────────────

const RUNNERS: AcpRunner[] = [
  new ClaudeRunner(),
  // Future: new CursorRunner(), new OpenCodeRunner(), ...
];

export function getRunner(id: string): AcpRunner {
  const runner = RUNNERS.find((r) => r.id === id);
  if (!runner) {
    const available = RUNNERS.map((r) => r.id).join(", ");
    throw new Error(`Unknown ACP '${id}'. Available: ${available}`);
  }
  return runner;
}

export function listRunners(): AcpRunner[] {
  return RUNNERS;
}
