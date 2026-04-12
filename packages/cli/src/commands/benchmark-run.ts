/**
 * sensei benchmark run --acp <id>
 *
 * Measures the real-world impact of sensei skills and indexed context by running
 * identical feature tasks through an ACP on three git branches:
 *
 *   <acp>-without-sensei  — base repo, no skills, no index
 *   <acp>-with-skills     — same repo + sensei skills in .claude/skills/
 *   <acp>-with-indexed    — same repo + skills + senseid MCP server + indexed code
 *
 * Supports --resume/--workdir for resumable runs across rate limit windows.
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
  resume?: string;
  workdir?: string;
}

interface TaskFile { id: string; path: string; }
interface TestResult { passed: number; failed: number; total: number; }
interface TaskResult { task: TaskFile; session: AcpSession; tests: TestResult; }
interface BranchReport { branch: string; tasks: TaskResult[]; }

type BranchMode = "bare" | "skills" | "indexed";

interface BenchmarkReport {
  metadata: { timestamp: string; acp: string; repo: string; tasks: string[] };
  branches: Record<string, BranchReport>;
  comparison: {
    perTask: Array<{
      id: string;
      bare: { cost: number; turns: number; tools: number; tests: number; total: number };
      skills: { cost: number; turns: number; tools: number; tests: number; total: number };
      indexed: { cost: number; turns: number; tools: number; tests: number; total: number };
    }>;
    summary: {
      bareCost: number; skillsCost: number; indexedCost: number;
      bareTests: number; skillsTests: number; indexedTests: number;
    };
  };
}

// ── Checkpoint ───────────────────────────────────────────────────────────────

interface CheckpointState {
  acp: string;
  sampleDir: string;
  skillsDir: string;
  taskIds: string[];
  bare: Record<string, TaskResult>;
  skills: Record<string, TaskResult>;
  indexed: Record<string, TaskResult>;
}

function checkpointPath(workDir: string): string {
  return join(workDir, "benchmark-checkpoint.json");
}

async function loadCheckpoint(workDir: string): Promise<CheckpointState | null> {
  try { return JSON.parse(await readFile(checkpointPath(workDir), "utf-8")) as CheckpointState; }
  catch { return null; }
}

async function saveCheckpoint(workDir: string, state: CheckpointState): Promise<void> {
  await writeFile(checkpointPath(workDir), JSON.stringify(state, null, 2));
}

// ── Path resolution ──────────────────────────────────────────────────────────

const EXAMPLE_DIR = join(dirname(fileURLToPath(import.meta.url)), "..", "..", "..", "..", "examples");

function findExampleSample(): string {
  for (const c of [join(EXAMPLE_DIR, "sample"), join(dirname(fileURLToPath(import.meta.url)), "..", "..", "..", "examples", "sample")]) {
    if (existsSync(join(c, "package.json"))) return c;
  }
  throw new Error("Cannot locate examples/sample/ — pass --repo to specify a custom repo");
}

// ── Git ──────────────────────────────────────────────────────────────────────

async function git(cwd: string, ...args: string[]): Promise<string> {
  const proc = Bun.spawn(["git", ...args], {
    cwd, stdout: "pipe", stderr: "pipe",
    env: { ...process.env, GIT_AUTHOR_NAME: "sensei-benchmark", GIT_AUTHOR_EMAIL: "benchmark@sensei.dev", GIT_COMMITTER_NAME: "sensei-benchmark", GIT_COMMITTER_EMAIL: "benchmark@sensei.dev" },
  });
  const out = await new Response(proc.stdout).text();
  await proc.exited;
  return out.trim();
}

// ── Tests ────────────────────────────────────────────────────────────────────

async function runTests(cwd: string): Promise<TestResult> {
  const proc = Bun.spawn(["bun", "test", "--timeout", "10000"], { cwd, stdout: "pipe", stderr: "pipe" });
  const [out, err] = await Promise.all([new Response(proc.stdout).text(), new Response(proc.stderr).text()]);
  await proc.exited;
  const combined = out + err;
  const passed = parseInt(combined.match(/(\d+) pass/)?.[1] ?? "0");
  const failed = parseInt(combined.match(/(\d+) fail/)?.[1] ?? "0");
  return { passed, failed, total: passed + failed };
}

// ── Skills ───────────────────────────────────────────────────────────────────

async function installSkills(skillsDir: string, workDir: string): Promise<void> {
  const dest = join(workDir, ".claude", "skills");
  await mkdir(dest, { recursive: true });
  for (const f of (await readdir(skillsDir)).filter(f => f.endsWith(".md"))) {
    await copyFile(join(skillsDir, f), join(dest, f));
  }
}

// ── Claude settings ──────────────────────────────────────────────────────────

async function setupClaudeSettings(workDir: string, mode: BranchMode): Promise<void> {
  const settingsDir = join(workDir, ".claude");
  await mkdir(settingsDir, { recursive: true });

  const settings: Record<string, unknown> = {
    hooks: {},
    permissions: { allow: ["Bash(*)", "Read(*)", "Write(*)", "Edit(*)", "mcp__sensei__*"] },
  };

  if (mode === "indexed") {
    // Configure MCP server pointing at this workdir
    const mcpEntry = join(dirname(fileURLToPath(import.meta.url)), "..", "..", "..", "server", "src", "mcp-entry.ts");
    settings.mcpServers = {
      sensei: {
        command: "bun",
        args: [mcpEntry],
        env: { SENSEI_REPO_PATH: workDir },
      },
    };

    // Create .sensei/config.yaml so the MCP server recognizes this as a sensei repo
    const senseiDir = join(workDir, ".sensei");
    await mkdir(senseiDir, { recursive: true });
    await writeFile(join(senseiDir, "config.yaml"), `repo_id: benchmark-${basename(workDir)}\n`);
  }

  await writeFile(join(settingsDir, "settings.json"), JSON.stringify(settings, null, 2));

  const claudeMd = mode === "indexed"
    ? "# Benchmark workspace\nImplement the requested feature.\nUse sensei MCP tools (search, get_symbol, context_pack) to understand the codebase before coding.\n"
    : "# Benchmark workspace\nImplement the requested feature.\n";
  await writeFile(join(workDir, "CLAUDE.md"), claudeMd);
}

// ── Workdir setup ────────────────────────────────────────────────────────────

async function setupWorkdir(sampleDir: string, targetDir?: string): Promise<string> {
  const workDir = targetDir ?? join(tmpdir(), `sensei-benchmark-${Date.now()}`);
  if (existsSync(workDir)) {
    try { const { $ } = await import("bun"); await $`rm -rf ${workDir}`; } catch {}
  }
  await cp(sampleDir, workDir, { recursive: true });
  try { const { $ } = await import("bun"); await $`rm -rf ${join(workDir, ".git")}`; } catch {}
  if (existsSync(join(workDir, "package.json"))) {
    const proc = Bun.spawn(["bun", "install"], { cwd: workDir, stdout: "pipe", stderr: "pipe" });
    await proc.exited;
  }
  await git(workDir, "init", "-b", "main");
  await git(workDir, "add", "-A");
  await git(workDir, "commit", "-m", "initial state");
  return workDir;
}

// ── Rate-limit aware task runner ─────────────────────────────────────────────

function parseRateLimitReset(rawOutput: string): number | null {
  const match = rawOutput.match(/"resetsAt"\s*:\s*(\d+)/);
  return match ? parseInt(match[1], 10) : null;
}

async function runTaskWithRetry(
  runner: AcpRunner, task: TaskFile, workDir: string, verbose: boolean,
): Promise<{ session: AcpSession; tests: TestResult }> {
  let attempt = 0;
  while (true) {
    attempt++;
    process.stdout.write(`  [${task.id}]${attempt > 1 ? ` (attempt ${attempt})` : ""}`);

    const session = await runner.runTask(task.path, workDir);

    if (session.exitCode !== 0 && session.rawOutput.includes("Credit balance is too low")) {
      const resetsAt = parseRateLimitReset(session.rawOutput);
      let waitSec: number;
      if (resetsAt) {
        waitSec = Math.max(30, Math.ceil(resetsAt - Date.now() / 1000));
        console.log(` ⏳ rate limited — resets at ${new Date(resetsAt * 1000).toLocaleTimeString()} — waiting ${Math.ceil(waitSec / 60)}min...`);
      } else {
        waitSec = Math.min(attempt * 120, 600);
        console.log(` ⏳ rate limited — waiting ${Math.ceil(waitSec / 60)}min...`);
      }
      await new Promise(r => setTimeout(r, waitSec * 1000));
      continue;
    }

    const ctxK = Math.round(session.totalContextTokens / 1000);
    process.stdout.write(` ${ctxK}k ctx ${session.toolCalls} tools ${session.numTurns} turns $${session.costUsd.toFixed(2)}`);

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

// ── Branch runner ────────────────────────────────────────────────────────────

async function runBranch(
  runner: AcpRunner,
  workDir: string,
  branchName: string,
  taskFiles: TaskFile[],
  mode: BranchMode,
  skillsDir: string,
  verbose: boolean,
  completed: Record<string, TaskResult>,
): Promise<BranchReport> {
  const branches = await git(workDir, "branch", "--list", branchName);
  if (branches.includes(branchName)) {
    await git(workDir, "checkout", branchName);
  } else {
    await git(workDir, "checkout", "-b", branchName);

    if (mode === "skills" || mode === "indexed") {
      await installSkills(skillsDir, workDir);
      await git(workDir, "add", "-A");
      await git(workDir, "commit", "-m", "sensei: install skills");
    }

    await setupClaudeSettings(workDir, mode);
  }

  // Index the repo for indexed mode (populates Kuzu DB before MCP server starts)
  if (mode === "indexed") {
    try {
      process.stdout.write("  indexing codebase into graph...");
      const { indexRepo: indexRepoFn } = await import("@sensei/graph-indexer");
      const repoId = `benchmark-${basename(workDir)}`;
      await indexRepoFn({ repoPath: workDir, repoId, project: repoId });
      console.log(" done");
    } catch (err) {
      console.log(` ⚠ ${err instanceof Error ? err.message : String(err)}`);
    }
  }

  const results: TaskResult[] = [];

  for (const task of taskFiles) {
    if (completed[task.id]) {
      const prev = completed[task.id];
      const ctxK = Math.round(prev.session.totalContextTokens / 1000);
      process.stdout.write(`  [${task.id}] (cached) ${ctxK}k ctx $${prev.session.costUsd.toFixed(2)} → ${prev.tests.passed}/${prev.tests.total} tests pass\n`);
      results.push(prev);
      continue;
    }

    const { session, tests } = await runTaskWithRetry(runner, task, workDir, verbose);
    results.push({ task, session, tests });
    completed[task.id] = { task, session, tests };
  }

  return { branch: branchName, tasks: results };
}

// ── Report ───────────────────────────────────────────────────────────────────

function buildReport(
  acp: string, repoPath: string, taskFiles: TaskFile[],
  bare: BranchReport, skills: BranchReport, indexed: BranchReport,
): BenchmarkReport {
  const perTask = taskFiles.map((task, i) => ({
    id: task.id,
    bare:    { cost: bare.tasks[i].session.costUsd,    turns: bare.tasks[i].session.numTurns,    tools: bare.tasks[i].session.toolCalls,    tests: bare.tasks[i].tests.passed,    total: bare.tasks[i].tests.total },
    skills:  { cost: skills.tasks[i].session.costUsd,  turns: skills.tasks[i].session.numTurns,  tools: skills.tasks[i].session.toolCalls,  tests: skills.tasks[i].tests.passed,  total: skills.tasks[i].tests.total },
    indexed: { cost: indexed.tasks[i].session.costUsd, turns: indexed.tasks[i].session.numTurns, tools: indexed.tasks[i].session.toolCalls, tests: indexed.tasks[i].tests.passed, total: indexed.tasks[i].tests.total },
  }));

  return {
    metadata: { timestamp: new Date().toISOString(), acp, repo: repoPath, tasks: taskFiles.map(t => t.id) },
    branches: { bare, skills, indexed },
    comparison: {
      perTask,
      summary: {
        bareCost:    bare.tasks.reduce((s, r) => s + r.session.costUsd, 0),
        skillsCost:  skills.tasks.reduce((s, r) => s + r.session.costUsd, 0),
        indexedCost: indexed.tasks.reduce((s, r) => s + r.session.costUsd, 0),
        bareTests:    bare.tasks.reduce((s, r) => s + r.tests.passed, 0),
        skillsTests:  skills.tasks.reduce((s, r) => s + r.tests.passed, 0),
        indexedTests: indexed.tasks.reduce((s, r) => s + r.tests.passed, 0),
      },
    },
  };
}

function printReport(report: BenchmarkReport): void {
  const { perTask, summary } = report.comparison;

  console.log(`\n── Results ──────────────────────────────────────────────────────────────────────`);
  console.log(`${"Task".padEnd(12)} │ ${"Bare".padStart(14)} │ ${"+ Skills".padStart(14)} │ ${"+ Skills+Index".padStart(14)} │ ${"Tests".padStart(16)}`);
  console.log("─".repeat(82));

  for (const t of perTask) {
    const bc = `$${t.bare.cost.toFixed(2)} ${t.bare.turns}t`;
    const sc = `$${t.skills.cost.toFixed(2)} ${t.skills.turns}t`;
    const ic = `$${t.indexed.cost.toFixed(2)} ${t.indexed.turns}t`;
    const tests = `${t.bare.tests}→${t.skills.tests}→${t.indexed.tests}/${t.bare.total}`;
    console.log(`${t.id.padEnd(12)} │ ${bc.padStart(14)} │ ${sc.padStart(14)} │ ${ic.padStart(14)} │ ${tests.padStart(16)}`);
  }

  console.log("─".repeat(82));
  const totals = `${"TOTAL".padEnd(12)} │ ${"$" + summary.bareCost.toFixed(2)}${" ".padStart(14 - ("$" + summary.bareCost.toFixed(2)).length)} │ ${"$" + summary.skillsCost.toFixed(2)}${" ".padStart(14 - ("$" + summary.skillsCost.toFixed(2)).length)} │ ${"$" + summary.indexedCost.toFixed(2)}${" ".padStart(14 - ("$" + summary.indexedCost.toFixed(2)).length)} │ ${(summary.bareTests + "→" + summary.skillsTests + "→" + summary.indexedTests).padStart(16)}`;
  console.log(totals);

  const skillsSaved = summary.bareCost > 0 ? Math.round(((summary.bareCost - summary.skillsCost) / summary.bareCost) * 100) : 0;
  const indexedSaved = summary.bareCost > 0 ? Math.round(((summary.bareCost - summary.indexedCost) / summary.bareCost) * 100) : 0;

  console.log(`
  Skills vs bare:    ${skillsSaved >= 0 ? "" : "+"}${-skillsSaved}% cost, +${summary.skillsTests - summary.bareTests} tests
  Indexed vs bare:   ${indexedSaved >= 0 ? "" : "+"}${-indexedSaved}% cost, +${summary.indexedTests - summary.bareTests} tests
  Indexed vs skills: ${summary.skillsCost > 0 ? Math.round(((summary.skillsCost - summary.indexedCost) / summary.skillsCost) * 100) : 0}% cost, +${summary.indexedTests - summary.skillsTests} tests`);

  // ── Tool usage breakdown per branch ──────────────────────────────────────
  console.log(`\n── Tool Usage ──────────────────────────────────────────────────────────────────`);
  for (const branchKey of ["bare", "skills", "indexed"] as const) {
    const branch = report.branches[branchKey];
    if (!branch) continue;

    let totalReads = 0, totalWrites = 0, totalMcp = 0;
    const allReads = new Set<string>();
    const allWrites = new Set<string>();
    const mcpBreakdown = new Map<string, number>();

    for (const t of branch.tasks) {
      for (const f of t.session.filesRead) { allReads.add(f); totalReads++; }
      for (const f of t.session.filesWritten) { allWrites.add(f); totalWrites++; }
      for (const m of t.session.mcpCalls) {
        totalMcp++;
        mcpBreakdown.set(m, (mcpBreakdown.get(m) ?? 0) + 1);
      }
    }

    const label = branchKey === "bare" ? "Bare" : branchKey === "skills" ? "Skills" : "Indexed";
    console.log(`\n  ${label}:`);
    console.log(`    files read:    ${allReads.size} unique (${totalReads} total)`);
    if (allReads.size > 0 && allReads.size <= 15) {
      for (const f of [...allReads].sort()) console.log(`      ${f}`);
    }
    console.log(`    files written: ${allWrites.size} unique`);
    if (totalMcp > 0) {
      console.log(`    MCP calls:     ${totalMcp}`);
      for (const [name, count] of [...mcpBreakdown.entries()].sort((a, b) => b[1] - a[1])) {
        console.log(`      ${name}: ${count}`);
      }
    }
  }
}

// ── Main ─────────────────────────────────────────────────────────────────────

export async function benchmarkRun(_cwd: string, opts: BenchmarkRunOptions = {}): Promise<void> {
  const acpId = opts.acp ?? "claude";
  const verbose = opts.verbose ?? false;

  let runner: AcpRunner;
  try { runner = getRunner(acpId); }
  catch (e) {
    console.error(`${e instanceof Error ? e.message : String(e)}`);
    console.error(`Available ACPs: ${listRunners().map(r => `${r.id} (${r.name})`).join(", ")}`);
    process.exit(1);
  }

  if (!await runner.detect()) {
    console.error(`${runner.name} CLI not found.`);
    process.exit(1);
  }

  const sampleDir = opts.repo ?? findExampleSample();
  const tasksDir = opts.tasks ?? join(sampleDir, "tasks");
  const skillsDir = opts.skills ?? join(sampleDir, "skills");

  if (!existsSync(tasksDir)) { console.error(`Tasks directory not found: ${tasksDir}`); process.exit(1); }

  const taskFiles: TaskFile[] = (await readdir(tasksDir))
    .filter(f => f.endsWith(".md"))
    .sort()
    .map(f => ({ id: basename(f, ".md"), path: join(tasksDir, f) }));

  if (taskFiles.length === 0) { console.error(`No .md task files in ${tasksDir}`); process.exit(1); }

  console.log(`sensei benchmark run`);
  console.log(`  acp:    ${runner.name}`);
  console.log(`  repo:   ${sampleDir}`);
  console.log(`  tasks:  ${taskFiles.map(t => t.id).join(", ")}`);
  console.log(`  branches: bare → skills → skills+indexed\n`);

  // Resume or fresh
  let workDir: string;
  let checkpoint: CheckpointState | null = null;

  const resumeDir = opts.resume ?? opts.workdir;
  if (resumeDir && existsSync(resumeDir) && existsSync(checkpointPath(resumeDir))) {
    workDir = resumeDir;
    checkpoint = await loadCheckpoint(workDir);
    const done = Object.keys(checkpoint!.bare).length + Object.keys(checkpoint!.skills).length + Object.keys(checkpoint!.indexed).length;
    console.log(`  resume: ${workDir} (${done}/${taskFiles.length * 3} tasks cached)`);
  } else if (opts.workdir) {
    workDir = await setupWorkdir(sampleDir, opts.workdir);
  } else {
    workDir = await setupWorkdir(sampleDir);
  }

  console.log(`  workdir: ${workDir}\n`);

  if (!checkpoint) {
    checkpoint = { acp: acpId, sampleDir, skillsDir, taskIds: taskFiles.map(t => t.id), bare: {}, skills: {}, indexed: {} };
  }

  // ── Branch 1: bare ────────────────────────────────────────────────────────
  console.log(`── Branch: ${acpId}-bare ───────────────────────────────────────`);
  const bare = await runBranch(runner, workDir, `${acpId}-bare`, taskFiles, "bare", skillsDir, verbose, checkpoint.bare);
  checkpoint.bare = {}; for (const r of bare.tasks) checkpoint.bare[r.task.id] = r;
  await saveCheckpoint(workDir, checkpoint);
  await git(workDir, "checkout", "main");

  // ── Branch 2: skills ──────────────────────────────────────────────────────
  console.log(`\n── Branch: ${acpId}-with-skills ────────────────────────────────`);
  const skills = await runBranch(runner, workDir, `${acpId}-with-skills`, taskFiles, "skills", skillsDir, verbose, checkpoint.skills);
  checkpoint.skills = {}; for (const r of skills.tasks) checkpoint.skills[r.task.id] = r;
  await saveCheckpoint(workDir, checkpoint);
  await git(workDir, "checkout", "main");

  // ── Branch 3: skills + indexed ────────────────────────────────────────────
  console.log(`\n── Branch: ${acpId}-with-indexed ───────────────────────────────`);
  const indexed = await runBranch(runner, workDir, `${acpId}-with-indexed`, taskFiles, "indexed", skillsDir, verbose, checkpoint.indexed);
  checkpoint.indexed = {}; for (const r of indexed.tasks) checkpoint.indexed[r.task.id] = r;
  await saveCheckpoint(workDir, checkpoint);

  // ── Report ────────────────────────────────────────────────────────────────
  const report = buildReport(acpId, sampleDir, taskFiles, bare, skills, indexed);
  printReport(report);

  const outPath = opts.output ?? join(workDir, "benchmark-results.json");
  await writeFile(outPath, JSON.stringify(report, null, 2));
  console.log(`\n  results written → ${outPath}`);
  console.log(`  workdir: ${workDir}`);
}
