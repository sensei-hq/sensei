/**
 * sensei benchmark run --acp <id>
 *
 * Measures the real-world impact of sensei skills by running identical feature
 * implementation tasks through an ACP (e.g. Claude Code) on two git branches:
 *
 *   <acp>-without-sensei  — base repo, no skills/context
 *   <acp>-with-sensei     — same repo + sensei skills in .claude/skills/
 *
 * Token usage and test pass rates are captured per task and compared.
 * Requires the ACP CLI to be installed (e.g. `claude`).
 *
 * Supports --resume to continue from a previous interrupted run.
 */

import { mkdir, readdir, copyFile, readFile, writeFile, cp } from "fs/promises";
import { existsSync } from "fs";
import { join, dirname, basename } from "path";
import { tmpdir } from "os";
import { fileURLToPath } from "url";
import { getRunner, listRunners, type AcpRunner, type AcpSession } from "../lib/acp-runner.js";

// ── Types ────────────────────────────────────────────────────────────────────

export interface BenchmarkRunOptions {
  acp?: string;
  repo?: string;
  tasks?: string;
  skills?: string;
  output?: string;
  verbose?: boolean;
  resume?: string;  // path to existing workdir to resume
  workdir?: string; // custom workdir path (created if missing, reused if exists)
}

interface TaskFile {
  id: string;
  path: string;
}

interface TestResult {
  passed: number;
  failed: number;
  total: number;
}

interface TaskResult {
  task: TaskFile;
  session: AcpSession;
  tests: TestResult;
}

interface BranchReport {
  branch: string;
  tasks: TaskResult[];
}

interface BenchmarkReport {
  metadata: {
    timestamp: string;
    acp: string;
    repo: string;
    tasks: string[];
  };
  baseline: BranchReport;
  withSensei: BranchReport;
  comparison: {
    perTask: Array<{
      id: string;
      baseContext: number;
      senseiContext: number;
      baseCost: number;
      senseiCost: number;
      costSaved: number;
      toolCallsSaved: number;
      baseTurns: number;
      senseiTurns: number;
      baselineTests: number;
      senseiTests: number;
      testDelta: number;
      baseTotal: number;
    }>;
    summary: {
      totalBaseCost: number;
      totalSenseiCost: number;
      costSavedPct: number;
      totalToolCallsSaved: number;
      baselinePassTotal: number;
      senseiPassTotal: number;
      testDelta: number;
    };
  };
}

// ── Checkpoint state (for resume) ────────────────────────────────────────────

interface CheckpointState {
  acp: string;
  sampleDir: string;
  skillsDir: string;
  taskIds: string[];
  baseline: Record<string, TaskResult>;  // taskId → result
  withSensei: Record<string, TaskResult>;
}

function checkpointPath(workDir: string): string {
  return join(workDir, "benchmark-checkpoint.json");
}

async function loadCheckpoint(workDir: string): Promise<CheckpointState | null> {
  try {
    const raw = await readFile(checkpointPath(workDir), "utf-8");
    return JSON.parse(raw) as CheckpointState;
  } catch {
    return null;
  }
}

async function saveCheckpoint(workDir: string, state: CheckpointState): Promise<void> {
  await writeFile(checkpointPath(workDir), JSON.stringify(state, null, 2));
}

// ── Path resolution ───────────────────────────────────────────────────────────

const EXAMPLE_DIR = join(
  dirname(fileURLToPath(import.meta.url)),
  "..", "..", "..", "..", "examples",
);

function findExampleSample(): string {
  const candidates = [
    join(EXAMPLE_DIR, "sample"),
    join(dirname(fileURLToPath(import.meta.url)), "..", "..", "..", "examples", "sample"),
  ];
  for (const c of candidates) {
    if (existsSync(join(c, "package.json"))) return c;
  }
  throw new Error("Cannot locate examples/sample/ — pass --repo to specify a custom repo");
}

// ── Git helpers ───────────────────────────────────────────────────────────────

async function git(cwd: string, ...args: string[]): Promise<string> {
  const proc = Bun.spawn(["git", ...args], {
    cwd,
    stdout: "pipe",
    stderr: "pipe",
    env: { ...process.env, GIT_AUTHOR_NAME: "sensei-benchmark", GIT_AUTHOR_EMAIL: "benchmark@sensei.dev", GIT_COMMITTER_NAME: "sensei-benchmark", GIT_COMMITTER_EMAIL: "benchmark@sensei.dev" },
  });
  const out = await new Response(proc.stdout).text();
  await proc.exited;
  return out.trim();
}

// ── Test runner ───────────────────────────────────────────────────────────────

async function runTests(cwd: string): Promise<TestResult> {
  const proc = Bun.spawn(["bun", "test", "--timeout", "10000"], {
    cwd,
    stdout: "pipe",
    stderr: "pipe",
  });
  const [out, err] = await Promise.all([
    new Response(proc.stdout).text(),
    new Response(proc.stderr).text(),
  ]);
  await proc.exited;

  const combined = out + err;
  const passMatch = combined.match(/(\d+) pass/);
  const failMatch = combined.match(/(\d+) fail/);
  const passed = passMatch ? parseInt(passMatch[1]) : 0;
  const failed = failMatch ? parseInt(failMatch[1]) : 0;
  return { passed, failed, total: passed + failed };
}

// ── Skill installer ──────────────────────────────────────────────────────────

async function installSkills(skillsDir: string, workDir: string): Promise<void> {
  const dest = join(workDir, ".claude", "skills");
  await mkdir(dest, { recursive: true });
  const files = (await readdir(skillsDir)).filter((f) => f.endsWith(".md"));
  for (const f of files) {
    await copyFile(join(skillsDir, f), join(dest, f));
  }
}

// ── Benchmark workspace setup ────────────────────────────────────────────────

/**
 * Write a minimal .claude/settings.json that disables global hooks and plugins
 * so each benchmark session is lean — no 27 plugins, no session-start hooks,
 * just the bare ACP + the task prompt.
 */
async function setupClaudeSettings(workDir: string): Promise<void> {
  const settingsDir = join(workDir, ".claude");
  await mkdir(settingsDir, { recursive: true });
  const settingsPath = join(settingsDir, "settings.json");

  const settings = {
    // Empty hooks — prevents global hooks from firing
    hooks: {},
    // Permission bypass for non-interactive benchmark
    permissions: {
      allow: ["Bash(*)", "Read(*)", "Write(*)", "Edit(*)"],
    },
  };

  await writeFile(settingsPath, JSON.stringify(settings, null, 2));

  // Also write a minimal CLAUDE.md to prevent global CLAUDE.md from loading
  await writeFile(join(workDir, "CLAUDE.md"), "# Benchmark workspace\nThis is a benchmark run. Implement the requested feature.\n");
}

// ── Setup a fresh copy of the repo in a temp dir ──────────────────────────────

async function setupWorkdir(sampleDir: string, targetDir?: string): Promise<string> {
  const workDir = targetDir ?? join(tmpdir(), `sensei-benchmark-${Date.now()}`);
  if (existsSync(workDir)) {
    // If target exists but has no checkpoint, wipe and start fresh
    try {
      const { $ } = await import("bun");
      await $`rm -rf ${workDir}`;
    } catch {}
  }
  await cp(sampleDir, workDir, { recursive: true });
  // Remove any existing .git to start clean
  try {
    const { $ } = await import("bun");
    await $`rm -rf ${join(workDir, ".git")}`;
  } catch {}
  // Install dependencies if package.json exists
  if (existsSync(join(workDir, "package.json"))) {
    const proc = Bun.spawn(["bun", "install"], { cwd: workDir, stdout: "pipe", stderr: "pipe" });
    await proc.exited;
  }
  await git(workDir, "init", "-b", "main");
  await git(workDir, "add", "-A");
  await git(workDir, "commit", "-m", "initial state");
  return workDir;
}

// ── Run a single task (with rate-limit wait) ─────────────────────────────────

function parseRateLimitReset(rawOutput: string): number | null {
  // Look for resetsAt timestamp in rate_limit_event
  const match = rawOutput.match(/"resetsAt"\s*:\s*(\d+)/);
  if (match) return parseInt(match[1], 10);
  return null;
}

async function runTaskWithRetry(
  runner: AcpRunner,
  task: TaskFile,
  workDir: string,
  verbose: boolean,
): Promise<{ session: AcpSession; tests: TestResult }> {
  let attempt = 0;

  while (true) {
    attempt++;
    process.stdout.write(`  [${task.id}]${attempt > 1 ? ` (attempt ${attempt})` : ""}`);

    const session = await runner.runTask(task.path, workDir);

    // Detect rate limit / credit errors — wait and retry indefinitely
    if (session.exitCode !== 0 && session.rawOutput.includes("Credit balance is too low")) {
      // Try to find the reset timestamp from the output
      const resetsAt = parseRateLimitReset(session.rawOutput);
      let waitSec: number;

      if (resetsAt) {
        waitSec = Math.max(30, Math.ceil(resetsAt - Date.now() / 1000));
        const resetTime = new Date(resetsAt * 1000).toLocaleTimeString();
        console.log(` ⏳ rate limited — resets at ${resetTime} — waiting ${Math.ceil(waitSec / 60)}min...`);
      } else {
        waitSec = Math.min(attempt * 120, 600); // 2min, 4min, 6min... max 10min
        console.log(` ⏳ rate limited — waiting ${Math.ceil(waitSec / 60)}min before retry...`);
      }

      await new Promise(r => setTimeout(r, waitSec * 1000));
      continue;
    }

    const ctxK = Math.round(session.totalContextTokens / 1000);
    process.stdout.write(
      ` ${ctxK}k ctx ${session.toolCalls} tools ${session.numTurns} turns $${session.costUsd.toFixed(2)}`,
    );

    // Commit whatever the ACP wrote
    await git(workDir, "add", "-A");
    await git(workDir, "commit", "-m", `implement ${task.id}`, "--allow-empty");

    const tests = await runTests(workDir);
    process.stdout.write(` → ${tests.passed}/${tests.total} tests pass\n`);

    if (verbose && session.rawOutput) {
      const preview = session.rawOutput.slice(0, 300).replace(/\n/g, "\n    ");
      console.log(`    output preview:\n    ${preview}\n`);
    }

    return { session, tests };
  }
}

// ── Branch runner (with checkpoint support) ──────────────────────────────────

async function runBranch(
  runner: AcpRunner,
  workDir: string,
  branchName: string,
  taskFiles: TaskFile[],
  withSensei: boolean,
  skillsDir: string,
  verbose: boolean,
  completed: Record<string, TaskResult>,
): Promise<BranchReport> {
  // Check if branch already exists (resume case)
  const branches = await git(workDir, "branch", "--list", branchName);
  if (branches.includes(branchName)) {
    await git(workDir, "checkout", branchName);
  } else {
    await git(workDir, "checkout", "-b", branchName);

    if (withSensei) {
      await installSkills(skillsDir, workDir);
      await git(workDir, "add", "-A");
      await git(workDir, "commit", "-m", "sensei: install skills");
    }

    await setupClaudeSettings(workDir);
  }

  const results: TaskResult[] = [];

  for (const task of taskFiles) {
    // Skip if already completed in a previous run
    if (completed[task.id]) {
      const prev = completed[task.id];
      const ctxK = Math.round(prev.session.totalContextTokens / 1000);
      process.stdout.write(`  [${task.id}] (cached) ${ctxK}k ctx $${prev.session.costUsd.toFixed(2)} → ${prev.tests.passed}/${prev.tests.total} tests pass\n`);
      results.push(prev);
      continue;
    }

    const { session, tests } = await runTaskWithRetry(runner, task, workDir, verbose);
    const result: TaskResult = { task, session, tests };
    results.push(result);

    // Save checkpoint after each task
    completed[task.id] = result;
  }

  return { branch: branchName, tasks: results };
}

// ── Report generation ─────────────────────────────────────────────────────────

function buildReport(
  acp: string,
  repoPath: string,
  taskFiles: TaskFile[],
  baseline: BranchReport,
  withSensei: BranchReport,
): BenchmarkReport {
  const perTask = taskFiles.map((task, i) => {
    const b = baseline.tasks[i];
    const s = withSensei.tasks[i];
    return {
      id: task.id,
      baseContext: b.session.totalContextTokens,
      senseiContext: s.session.totalContextTokens,
      baseCost: b.session.costUsd,
      senseiCost: s.session.costUsd,
      costSaved: b.session.costUsd - s.session.costUsd,
      toolCallsSaved: b.session.toolCalls - s.session.toolCalls,
      baseTurns: b.session.numTurns,
      senseiTurns: s.session.numTurns,
      baselineTests: b.tests.passed,
      senseiTests: s.tests.passed,
      testDelta: s.tests.passed - b.tests.passed,
      baseTotal: b.tests.total,
    };
  });

  const totalBaseCost = baseline.tasks.reduce((s, r) => s + r.session.costUsd, 0);
  const totalSenseiCost = withSensei.tasks.reduce((s, r) => s + r.session.costUsd, 0);

  return {
    metadata: {
      timestamp: new Date().toISOString(),
      acp,
      repo: repoPath,
      tasks: taskFiles.map((t) => t.id),
    },
    baseline,
    withSensei,
    comparison: {
      perTask,
      summary: {
        totalBaseCost: totalBaseCost,
        totalSenseiCost: totalSenseiCost,
        costSavedPct: totalBaseCost > 0 ? Math.round(((totalBaseCost - totalSenseiCost) / totalBaseCost) * 100) : 0,
        totalToolCallsSaved:
          baseline.tasks.reduce((s, r) => s + r.session.toolCalls, 0)
          - withSensei.tasks.reduce((s, r) => s + r.session.toolCalls, 0),
        baselinePassTotal: baseline.tasks.reduce((s, r) => s + r.tests.passed, 0),
        senseiPassTotal: withSensei.tasks.reduce((s, r) => s + r.tests.passed, 0),
        testDelta: withSensei.tasks.reduce((s, r) => s + r.tests.passed, 0)
          - baseline.tasks.reduce((s, r) => s + r.tests.passed, 0),
      },
    },
  };
}

function printReport(report: BenchmarkReport): void {
  const { perTask, summary } = report.comparison;

  console.log(`\n── Results ──────────────────────────────────────────────────────────────────────`);
  console.log(`${"Task".padEnd(12)} ${"Base cost".padStart(10)} ${"Sensei cost".padStart(12)} ${"Saved".padStart(7)} ${"Base".padStart(6)} ${"Sensei".padStart(7)} ${"Tools".padStart(6)} ${"Tests".padStart(12)}`);
  console.log("─".repeat(80));
  for (const t of perTask) {
    const costPct = t.baseCost > 0 ? Math.round(((t.baseCost - t.senseiCost) / t.baseCost) * 100) : 0;
    const costDir = costPct >= 0 ? `${costPct}%` : `${costPct}%`;
    const toolDelta = t.toolCallsSaved >= 0 ? `-${t.toolCallsSaved}` : `+${-t.toolCallsSaved}`;
    const testDelta = t.testDelta >= 0 ? `+${t.testDelta}` : `${t.testDelta}`;
    console.log(
      `${t.id.padEnd(12)} ${"$" + t.baseCost.toFixed(2)}${(" ").padStart(10 - ("$" + t.baseCost.toFixed(2)).length)} ${"$" + t.senseiCost.toFixed(2)}${(" ").padStart(12 - ("$" + t.senseiCost.toFixed(2)).length)} ${costDir.padStart(7)} ${(t.baseTurns + "t").padStart(6)} ${(t.senseiTurns + "t").padStart(7)} ${toolDelta.padStart(6)} ${(t.baselineTests + "→" + t.senseiTests + "/" + t.baseTotal).padStart(12)}`,
    );
  }
  console.log("─".repeat(80));

  const costDir = summary.costSavedPct >= 0 ? `${summary.costSavedPct}%` : `${summary.costSavedPct}%`;
  console.log(
    `${"TOTAL".padEnd(12)} ${"$" + summary.totalBaseCost.toFixed(2)}${(" ").padStart(10 - ("$" + summary.totalBaseCost.toFixed(2)).length)} ${"$" + summary.totalSenseiCost.toFixed(2)}${(" ").padStart(12 - ("$" + summary.totalSenseiCost.toFixed(2)).length)} ${costDir.padStart(7)}                    ${summary.baselinePassTotal}→${summary.senseiPassTotal}`,
  );

  console.log(`
  cost saved  : $${(summary.totalBaseCost - summary.totalSenseiCost).toFixed(2)} (${summary.costSavedPct}%)
  tool calls  : ${summary.totalToolCallsSaved >= 0 ? "-" : "+"}${Math.abs(summary.totalToolCallsSaved)} tool calls
  test quality: ${summary.testDelta >= 0 ? "+" : ""}${summary.testDelta} tests passing (baseline ${summary.baselinePassTotal} → sensei ${summary.senseiPassTotal})`);
}

// ── Main entry point ─────────────────────────────────────────────────────────

export async function benchmarkRun(
  _cwd: string,
  opts: BenchmarkRunOptions = {},
): Promise<void> {
  const acpId = opts.acp ?? "claude";
  const verbose = opts.verbose ?? false;

  // Resolve ACP runner
  let runner: AcpRunner;
  try {
    runner = getRunner(acpId);
  } catch (e) {
    const available = listRunners().map((r) => `${r.id} (${r.name})`).join(", ");
    console.error(`${e instanceof Error ? e.message : String(e)}`);
    console.error(`Available ACPs: ${available}`);
    process.exit(1);
  }

  if (!await runner.detect()) {
    console.error(`${runner.name} CLI not found. Install it first.`);
    console.error(`  Claude Code: npm install -g @anthropic-ai/claude-code`);
    process.exit(1);
  }

  // Resolve paths
  const sampleDir = opts.repo ?? findExampleSample();
  const tasksDir = opts.tasks ?? join(sampleDir, "tasks");
  const skillsDir = opts.skills ?? join(sampleDir, "skills");

  if (!existsSync(tasksDir)) {
    console.error(`Tasks directory not found: ${tasksDir}`);
    process.exit(1);
  }

  // Load task files (alphabetical order)
  const taskEntries = (await readdir(tasksDir))
    .filter((f) => f.endsWith(".md"))
    .sort();
  const taskFiles: TaskFile[] = taskEntries.map((f) => ({
    id: basename(f, ".md"),
    path: join(tasksDir, f),
  }));

  if (taskFiles.length === 0) {
    console.error(`No .md task files found in ${tasksDir}`);
    process.exit(1);
  }

  console.log(`sensei benchmark run`);
  console.log(`  acp:    ${runner.name}`);
  console.log(`  repo:   ${sampleDir}`);
  console.log(`  tasks:  ${taskFiles.map((t) => t.id).join(", ")}`);

  // Resume or fresh start
  let workDir: string;
  let checkpoint: CheckpointState | null = null;

  // --resume takes priority (explicit resume of a previous run)
  // --workdir creates/reuses a named directory (auto-resumes if checkpoint exists)
  // default: auto-generated temp dir
  const resumeDir = opts.resume ?? opts.workdir;
  if (resumeDir && existsSync(resumeDir) && existsSync(checkpointPath(resumeDir))) {
    workDir = resumeDir;
    checkpoint = await loadCheckpoint(workDir);
    const baselineDone = Object.keys(checkpoint!.baseline).length;
    const senseiDone = Object.keys(checkpoint!.withSensei).length;
    console.log(`  resume:  ${workDir}`);
    console.log(`  cached:  ${baselineDone} baseline + ${senseiDone} sensei tasks`);
  } else if (opts.workdir) {
    workDir = await setupWorkdir(sampleDir, opts.workdir);
  } else {
    workDir = await setupWorkdir(sampleDir);
  }

  console.log(`  workdir: ${workDir}\n`);

  // Initialize checkpoint
  if (!checkpoint) {
    checkpoint = {
      acp: acpId,
      sampleDir,
      skillsDir,
      taskIds: taskFiles.map(t => t.id),
      baseline: {},
      withSensei: {},
    };
  }

  // ── Branch 1: without sensei ──────────────────────────────────────────────
  console.log(`── Branch: ${acpId}-without-sensei ──────────────────────────`);
  const baseline = await runBranch(
    runner, workDir, `${acpId}-without-sensei`, taskFiles, false, skillsDir, verbose,
    checkpoint.baseline,
  );
  // Save checkpoint after baseline branch
  checkpoint.baseline = {};
  for (const r of baseline.tasks) checkpoint.baseline[r.task.id] = r;
  await saveCheckpoint(workDir, checkpoint);

  // Return to main for second branch
  await git(workDir, "checkout", "main");

  // ── Branch 2: with sensei ─────────────────────────────────────────────────
  console.log(`\n── Branch: ${acpId}-with-sensei ─────────────────────────────`);
  const withSensei = await runBranch(
    runner, workDir, `${acpId}-with-sensei`, taskFiles, true, skillsDir, verbose,
    checkpoint.withSensei,
  );
  // Save final checkpoint
  checkpoint.withSensei = {};
  for (const r of withSensei.tasks) checkpoint.withSensei[r.task.id] = r;
  await saveCheckpoint(workDir, checkpoint);

  // ── Report ────────────────────────────────────────────────────────────────
  const report = buildReport(acpId, sampleDir, taskFiles, baseline, withSensei);
  printReport(report);

  const outPath = opts.output ?? join(workDir, "benchmark-results.json");
  await writeFile(outPath, JSON.stringify(report, null, 2));

  console.log(`\n  results written → ${outPath}`);
  console.log(`  workdir: ${workDir}`);
}
