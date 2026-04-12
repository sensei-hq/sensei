/**
 * ACP runner abstraction for benchmark execution.
 *
 * Each runner knows how to:
 *  - detect whether the ACP is installed
 *  - invoke the ACP non-interactively with a task prompt
 *  - parse the output for token usage
 */

import { readFile } from "fs/promises";

// ── Types ────────────────────────────────────────────────────────────────────

export interface AcpSession {
  /** Total input tokens for the session (all turns). */
  inputTokens: number;
  /** Total output tokens for the session (all turns). */
  outputTokens: number;
  /** Number of agentic turns taken. */
  numTurns: number;
  /** Number of tool calls made (read_file, write_file, bash, etc.). */
  toolCalls: number;
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
  let outputTokens = 0;
  let numTurns = 0;
  let toolCalls = 0;

  for (const line of output.split("\n")) {
    const trimmed = line.trim();
    if (!trimmed) continue;
    try {
      const event = JSON.parse(trimmed) as Record<string, unknown>;
      if (event.type === "result") {
        const usage = event.usage as Record<string, number> | undefined;
        inputTokens = usage?.input_tokens ?? 0;
        outputTokens = usage?.output_tokens ?? 0;
        numTurns = (event.num_turns as number) ?? 0;
      }
      // Count tool_use blocks inside assistant messages
      if (event.type === "assistant") {
        const content = (event.message as { content?: unknown[] })?.content ?? [];
        for (const block of content) {
          if ((block as { type?: string }).type === "tool_use") toolCalls++;
        }
      }
    } catch {
      // Non-JSON lines (e.g. spinner output written to stdout) — skip
    }
  }

  return { inputTokens, outputTokens, numTurns, toolCalls };
}

export class ClaudeRunner implements AcpRunner {
  readonly id = "claude";
  readonly name = "Claude Code";

  async detect(): Promise<boolean> {
    return whichExists("claude");
  }

  async runTask(taskPath: string, cwd: string): Promise<AcpSession> {
    const prompt = await readFile(taskPath, "utf-8");

    const { stdout, exitCode } = await spawnCapture(
      ["claude", "--print", "--output-format", "stream-json", "--verbose"],
      { cwd, stdin: prompt },
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
