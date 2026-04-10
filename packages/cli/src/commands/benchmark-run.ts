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
      inputTokensSaved: number;
      inputPctSaved: number;
      toolCallsSaved: number;
      baselineTests: number;
      senseiTests: number;
      testDelta: number;
    }>;
    summary: {
      totalInputSaved: number;
      totalInputPctSaved: number;
      totalToolCallsSaved: number;
      baselinePassTotal: number;
      senseiPassTotal: number;
      testDelta: number;
    };
  };
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

// ── Sensei skill installation ─────────────────────────────────────────────────

async function installSkills(skillsDir: string, targetRepoDir: string): Promise<void> {
  const dest = join(targetRepoDir, ".claude", "skills");
  await mkdir(dest, { recursive: true });
  const entries = await readdir(skillsDir);
  for (const f of entries.filter((e) => e.endsWith(".md"))) {
    await copyFile(join(skillsDir, f), join(dest, f));
  }
}

// ── Hook installation (real-world capture alongside automated run) ─────────────

async function installCaptureHook(repoDir: string): Promise<void> {
  const settingsPath = join(repoDir, ".claude", "settings.json");
  await mkdir(join(repoDir, ".claude"), { recursive: true });

  let cfg: Record<string, unknown> = {};
  if (existsSync(settingsPath)) {
    try { cfg = JSON.parse(await readFile(settingsPath, "utf-8")); } catch {}
  }

  const hooks = (cfg.hooks ?? {}) as Record<string, unknown[]>;
  const stopHooks = (hooks.Stop ?? []) as Array<{ matcher: string; hooks: unknown[] }>;

  const alreadyInstalled = stopHooks.some((h) =>
    JSON.stringify(h).includes("benchmark/sessions.jsonl"),
  );
  if (!alreadyInstalled) {
    stopHooks.push({
      matcher: "",
      hooks: [{
        type: "command",
        command: [
          "node", "-e",
          "const d=JSON.parse(require('fs').readFileSync('/dev/stdin','utf-8'));",
          "require('fs').mkdirSync('.sensei/benchmark',{recursive:true});",
          "require('fs').appendFileSync('.sensei/benchmark/sessions.jsonl',",
          "JSON.stringify({ts:new Date().toISOString(),...d})+'\\n');",
        ].join(" "),
      }],
    });
    hooks.Stop = stopHooks;
    cfg.hooks = hooks;
    await writeFile(settingsPath, JSON.stringify(cfg, null, 2));
  }
}

// ── Setup a fresh copy of the repo in a temp dir ──────────────────────────────

async function setupWorkdir(sampleDir: string): Promise<string> {
  const workDir = join(tmpdir(), `sensei-benchmark-${Date.now()}`);
  await cp(sampleDir, workDir, { recursive: true });
  // Remove any existing .git to start clean
  try {
    const { $ } = await import("bun");
    await $`rm -rf ${join(workDir, ".git")}`;
  } catch {}
  await git(workDir, "init", "-b", "main");
  await git(workDir, "add", "-A");
  await git(workDir, "commit", "-m", "initial state");
  return workDir;
}

// ── Branch runner ─────────────────────────────────────────────────────────────

async function runBranch(
  runner: AcpRunner,
  workDir: string,
  branchName: string,
  taskFiles: TaskFile[],
  withSensei: boolean,
  skillsDir: string,
  verbose: boolean,
): Promise<BranchReport> {
  await git(workDir, "checkout", "-b", branchName);

  if (withSensei) {
    await installSkills(skillsDir, workDir);
    await git(workDir, "add", "-A");
    await git(workDir, "commit", "-m", "sensei: install skills");
  }

  await installCaptureHook(workDir);

  const results: TaskResult[] = [];

  for (const task of taskFiles) {
    process.stdout.write(`  [${task.id}]`);

    const session = await runner.runTask(task.path, workDir);
    process.stdout.write(
      ` ${session.inputTokens}in/${session.outputTokens}out ${session.toolCalls} tools ${session.numTurns} turns`,
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

    results.push({ task, session, tests });
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
    const inputSaved = b.session.inputTokens - s.session.inputTokens;
    const inputPct = b.session.inputTokens
      ? Math.round((inputSaved / b.session.inputTokens) * 1000) / 10
      : 0;
    return {
      id: task.id,
      inputTokensSaved: inputSaved,
      inputPctSaved: inputPct,
      toolCallsSaved: b.session.toolCalls - s.session.toolCalls,
      baselineTests: b.tests.passed,
      senseiTests: s.tests.passed,
      testDelta: s.tests.passed - b.tests.passed,
    };
  });

  const totalBaseInput = baseline.tasks.reduce((s, r) => s + r.session.inputTokens, 0);
  const totalSenseiInput = withSensei.tasks.reduce((s, r) => s + r.session.inputTokens, 0);
  const totalInputSaved = totalBaseInput - totalSenseiInput;

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
        totalInputSaved,
        totalInputPctSaved: totalBaseInput
          ? Math.round((totalInputSaved / totalBaseInput) * 1000) / 10
          : 0,
        totalToolCallsSaved: baseline.tasks.reduce((s, r) => s + r.session.toolCalls, 0)
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
  const { summary } = report.comparison;
  console.log("\n── Results ────────────────────────────────────────────────────");
  console.log(`${"Task".padEnd(22)} ${"Base tokens".padStart(12)} ${"Sensei tokens".padStart(14)} ${"Saved".padStart(7)} ${"Tests +/-".padStart(10)}`);
  console.log("─".repeat(70));

  for (const r of report.comparison.perTask) {
    const b = report.baseline.tasks.find((t) => t.task.id === r.id)!;
    const s = report.withSensei.tasks.find((t) => t.task.id === r.id)!;
    console.log(
      `${r.id.padEnd(22)} ${b.session.inputTokens.toString().padStart(12)} ${s.session.inputTokens.toString().padStart(14)} ${`${r.inputPctSaved}%`.padStart(7)} ${`${r.testDelta >= 0 ? "+" : ""}${r.testDelta}`.padStart(10)}`,
    );
  }

  console.log("─".repeat(70));
  console.log(
    `${"TOTAL".padEnd(22)} ${report.baseline.tasks.reduce((s, r) => s + r.session.inputTokens, 0).toString().padStart(12)} ${report.withSensei.tasks.reduce((s, r) => s + r.session.inputTokens, 0).toString().padStart(14)} ${`${summary.totalInputPctSaved}%`.padStart(7)} ${`${summary.testDelta >= 0 ? "+" : ""}${summary.testDelta}`.padStart(10)}`,
  );

  console.log(`
  input tokens saved : ${summary.totalInputSaved.toLocaleString()} (${summary.totalInputPctSaved}%)
  tool calls saved   : ${summary.totalToolCallsSaved}
  extra tests passing: ${summary.testDelta >= 0 ? "+" : ""}${summary.testDelta} (baseline ${summary.baselinePassTotal} → sensei ${summary.senseiPassTotal})`);
}

// ── Main ─────────────────────────────────────────────────────────────────────

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
  console.log();

  // Set up isolated working directory
  const workDir = await setupWorkdir(sampleDir);
  console.log(`  workdir: ${workDir}\n`);

  // ── Branch 1: without sensei ──────────────────────────────────────────────
  console.log(`── Branch: ${acpId}-without-sensei ──────────────────────────`);
  const baseline = await runBranch(
    runner, workDir, `${acpId}-without-sensei`, taskFiles, false, skillsDir, verbose,
  );

  // Return to main for second branch
  await git(workDir, "checkout", "main");

  // ── Branch 2: with sensei ─────────────────────────────────────────────────
  console.log(`\n── Branch: ${acpId}-with-sensei ─────────────────────────────`);
  const withSensei = await runBranch(
    runner, workDir, `${acpId}-with-sensei`, taskFiles, true, skillsDir, verbose,
  );

  // ── Report ────────────────────────────────────────────────────────────────
  const report = buildReport(acpId, sampleDir, taskFiles, baseline, withSensei);
  printReport(report);

  const outPath = opts.output ?? "benchmark-results.json";
  await writeFile(outPath, JSON.stringify(report, null, 2));
  console.log(`\n  results written → ${outPath}`);
  console.log(`  repo branches available at: ${workDir}`);
}
