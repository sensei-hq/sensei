/**
 * sensei benchmark run
 *
 * Measures the token-cost impact of sensei skills by running identical prompts
 * against two contexts:
 *
 *   naive   — prompt + raw source files concatenated (what developers paste manually)
 *   sensei  — prompt + skills markdown only (targeted knowledge)
 *
 * Uses the Anthropic API directly for reproducible, exact token counts.
 * Requires ANTHROPIC_API_KEY in environment.
 */

import Anthropic from "@anthropic-ai/sdk";
import { readFile, readdir, writeFile } from "fs/promises";
import { existsSync } from "fs";
import { join, dirname, relative } from "path";
import { fileURLToPath } from "url";

// ── Types ────────────────────────────────────────────────────────────────────

export interface BenchmarkRunOptions {
  model?: string;
  output?: string;   // write JSON results to this path
  repo?: string;     // custom repo path (default: bundled example)
  skillsDir?: string;
  verbose?: boolean;
}

interface Prompt {
  id: string;
  title: string;
  prompt: string;
}

interface RoundResult {
  contextType: "naive" | "sensei";
  contextChars: number;
  inputTokens: number;
  outputTokens: number;
  totalTokens: number;
  response: string;
}

interface PromptResult {
  id: string;
  title: string;
  prompt: string;
  naive: RoundResult;
  sensei: RoundResult;
  savings: {
    inputTokens: number;
    inputPct: number;
    totalTokens: number;
    totalPct: number;
  };
}

interface BenchmarkReport {
  metadata: {
    timestamp: string;
    model: string;
    repo: string;
    promptCount: number;
  };
  prompts: PromptResult[];
  summary: {
    naiveTotalInput: number;
    senseiTotalInput: number;
    inputSavings: number;
    inputSavingsPct: number;
    naiveTotalTokens: number;
    senseiTotalTokens: number;
    totalSavings: number;
    totalSavingsPct: number;
  };
}

// ── Helpers ──────────────────────────────────────────────────────────────────

const EXAMPLE_DIR = join(dirname(fileURLToPath(import.meta.url)), "..", "..", "..", "..", "examples", "benchmark");

async function findExampleDir(): Promise<string> {
  // production: relative to dist/
  const candidates = [
    EXAMPLE_DIR,
    join(dirname(fileURLToPath(import.meta.url)), "..", "..", "..", "examples", "benchmark"),
  ];
  for (const c of candidates) {
    if (existsSync(join(c, "prompts.json"))) return c;
  }
  throw new Error("Cannot locate examples/benchmark/ — run from the sensei monorepo or pass --repo");
}

async function collectSourceFiles(repoPath: string): Promise<string> {
  const parts: string[] = [];
  const walk = async (dir: string) => {
    const entries = await readdir(dir, { withFileTypes: true });
    for (const e of entries) {
      if (e.name.startsWith(".") || e.name === "node_modules" || e.name === "dist") continue;
      const full = join(dir, e.name);
      if (e.isDirectory()) {
        await walk(full);
      } else if (e.name.endsWith(".ts") || e.name.endsWith(".js")) {
        const rel = relative(repoPath, full);
        const content = await readFile(full, "utf-8");
        parts.push(`// === ${rel} ===\n${content}`);
      }
    }
  };
  await walk(repoPath);
  return parts.join("\n\n");
}

async function collectSkills(skillsDir: string): Promise<string> {
  if (!existsSync(skillsDir)) return "";
  const entries = await readdir(skillsDir);
  const parts: string[] = [];
  for (const e of entries.filter(f => f.endsWith(".md"))) {
    parts.push(await readFile(join(skillsDir, e), "utf-8"));
  }
  return parts.join("\n\n---\n\n");
}

function buildNaiveSystem(rawFiles: string): string {
  return `You are an expert TypeScript developer. Here is the full source of the codebase you are working on:\n\n${rawFiles}\n\nAnswer the developer's request with precise, idiomatic TypeScript. Return only the code changes needed, no commentary.`;
}

function buildSenseiSystem(skills: string): string {
  return `You are an expert TypeScript developer. The following knowledge base describes the codebase architecture, types, and patterns:\n\n${skills}\n\nAnswer the developer's request with precise, idiomatic TypeScript. Return only the code changes needed, no commentary.`;
}

function pct(a: number, b: number): number {
  if (b === 0) return 0;
  return Math.round(((b - a) / b) * 1000) / 10;
}

// ── Hook installer ───────────────────────────────────────────────────────────

async function installCaptureHook(repoPath: string): Promise<void> {
  const settingsPath = join(repoPath, ".claude", "settings.json");
  let cfg: Record<string, unknown> = {};
  if (existsSync(settingsPath)) {
    try {
      cfg = JSON.parse(await readFile(settingsPath, "utf-8"));
    } catch {}
  }

  // Install a Stop hook that appends session data to .sensei/benchmark/sessions.jsonl
  const hook = {
    type: "command",
    command: "node -e \"const d=JSON.parse(require('fs').readFileSync('/dev/stdin','utf-8')); require('fs').mkdirSync('.sensei/benchmark',{recursive:true}); require('fs').appendFileSync('.sensei/benchmark/sessions.jsonl', JSON.stringify({ts:new Date().toISOString(),...d})+'\\n');\""
  };

  const hooks = (cfg.hooks ?? {}) as Record<string, unknown>;
  const stopHooks = (hooks.Stop ?? []) as Array<{ matcher: string; hooks: unknown[] }>;

  const alreadyInstalled = stopHooks.some(
    h => JSON.stringify(h).includes("benchmark/sessions.jsonl")
  );

  if (!alreadyInstalled) {
    stopHooks.push({ matcher: "", hooks: [hook] });
    hooks.Stop = stopHooks;
    cfg.hooks = hooks;
    const { mkdir } = await import("fs/promises");
    await mkdir(join(repoPath, ".claude"), { recursive: true });
    await writeFile(settingsPath, JSON.stringify(cfg, null, 2));
  }
}

// ── Main ─────────────────────────────────────────────────────────────────────

export async function benchmarkRun(
  _cwd: string,
  opts: BenchmarkRunOptions = {},
): Promise<void> {
  const model = opts.model ?? "claude-haiku-4-5-20251001"; // use Haiku for cost efficiency
  const verbose = opts.verbose ?? false;

  const apiKey = process.env.ANTHROPIC_API_KEY;
  if (!apiKey) {
    console.error("sensei benchmark run: ANTHROPIC_API_KEY not set");
    process.exit(1);
  }

  const client = new Anthropic({ apiKey });

  // ── Locate example repo and skills ────────────────────────────────────────
  const exampleDir = await findExampleDir();
  const repoPath = opts.repo ?? join(exampleDir, "repo");
  const skillsDir = opts.skillsDir ?? join(exampleDir, "skills");

  if (!existsSync(repoPath)) {
    console.error(`sensei benchmark run: repo not found at ${repoPath}`);
    process.exit(1);
  }

  console.log("sensei benchmark run");
  console.log(`  repo:   ${repoPath}`);
  console.log(`  model:  ${model}`);
  console.log();

  // ── Install capture hook (idempotent) ─────────────────────────────────────
  await installCaptureHook(repoPath);
  console.log("  ✓ capture hook installed → .claude/settings.json");

  // ── Load contexts ─────────────────────────────────────────────────────────
  const rawFiles = await collectSourceFiles(repoPath);
  const skills = await collectSkills(skillsDir);

  if (!skills) {
    console.warn("  ⚠ no skills found — run `sensei skills` to generate, or add to examples/benchmark/skills/");
  }

  const naiveSystem = buildNaiveSystem(rawFiles);
  const senseiSystem = buildSenseiSystem(skills || rawFiles);

  // ── Load prompts ──────────────────────────────────────────────────────────
  const promptsPath = opts.repo
    ? join(opts.repo, "..", "prompts.json")
    : join(exampleDir, "prompts.json");
  const prompts: Prompt[] = JSON.parse(await readFile(
    existsSync(promptsPath) ? promptsPath : join(exampleDir, "prompts.json"),
    "utf-8",
  ));

  console.log(`  Running ${prompts.length} prompts × 2 contexts...\n`);

  // ── Run benchmark ─────────────────────────────────────────────────────────
  const results: PromptResult[] = [];

  for (const p of prompts) {
    process.stdout.write(`  [${p.id}] naive... `);

    const naiveResp = await client.messages.create({
      model,
      max_tokens: 1024,
      system: naiveSystem,
      messages: [{ role: "user", content: p.prompt }],
    });
    const naiveText = naiveResp.content.filter(b => b.type === "text").map(b => (b as { type: "text"; text: string }).text).join("");

    process.stdout.write(`${naiveResp.usage.input_tokens}in/${naiveResp.usage.output_tokens}out  sensei... `);

    const senseiResp = await client.messages.create({
      model,
      max_tokens: 1024,
      system: senseiSystem,
      messages: [{ role: "user", content: p.prompt }],
    });
    const senseiText = senseiResp.content.filter(b => b.type === "text").map(b => (b as { type: "text"; text: string }).text).join("");

    process.stdout.write(`${senseiResp.usage.input_tokens}in/${senseiResp.usage.output_tokens}out\n`);

    if (verbose) {
      console.log(`\n    naive response:\n${naiveText.slice(0, 200)}...\n`);
      console.log(`    sensei response:\n${senseiText.slice(0, 200)}...\n`);
    }

    const naive: RoundResult = {
      contextType: "naive",
      contextChars: naiveSystem.length,
      inputTokens: naiveResp.usage.input_tokens,
      outputTokens: naiveResp.usage.output_tokens,
      totalTokens: naiveResp.usage.input_tokens + naiveResp.usage.output_tokens,
      response: naiveText,
    };

    const sensei: RoundResult = {
      contextType: "sensei",
      contextChars: senseiSystem.length,
      inputTokens: senseiResp.usage.input_tokens,
      outputTokens: senseiResp.usage.output_tokens,
      totalTokens: senseiResp.usage.input_tokens + senseiResp.usage.output_tokens,
      response: senseiText,
    };

    results.push({
      id: p.id,
      title: p.title,
      prompt: p.prompt,
      naive,
      sensei,
      savings: {
        inputTokens: naive.inputTokens - sensei.inputTokens,
        inputPct: pct(sensei.inputTokens, naive.inputTokens),
        totalTokens: naive.totalTokens - sensei.totalTokens,
        totalPct: pct(sensei.totalTokens, naive.totalTokens),
      },
    });
  }

  // ── Summary ───────────────────────────────────────────────────────────────
  const naiveTotalInput = results.reduce((s, r) => s + r.naive.inputTokens, 0);
  const senseiTotalInput = results.reduce((s, r) => s + r.sensei.inputTokens, 0);
  const naiveTotalTokens = results.reduce((s, r) => s + r.naive.totalTokens, 0);
  const senseiTotalTokens = results.reduce((s, r) => s + r.sensei.totalTokens, 0);

  const report: BenchmarkReport = {
    metadata: {
      timestamp: new Date().toISOString(),
      model,
      repo: repoPath,
      promptCount: prompts.length,
    },
    prompts: results,
    summary: {
      naiveTotalInput,
      senseiTotalInput,
      inputSavings: naiveTotalInput - senseiTotalInput,
      inputSavingsPct: pct(senseiTotalInput, naiveTotalInput),
      naiveTotalTokens,
      senseiTotalTokens,
      totalSavings: naiveTotalTokens - senseiTotalTokens,
      totalSavingsPct: pct(senseiTotalTokens, naiveTotalTokens),
    },
  };

  // ── Print summary ─────────────────────────────────────────────────────────
  console.log("\n── Results ────────────────────────────────────────────");
  console.log(`${"Prompt".padEnd(26)} ${"Naive".padStart(8)} ${"Sensei".padStart(8)} ${"Saved".padStart(8)}`);
  console.log("─".repeat(54));
  for (const r of results) {
    const label = r.title.slice(0, 25).padEnd(25);
    const naive = r.naive.inputTokens.toString().padStart(8);
    const sensei = r.sensei.inputTokens.toString().padStart(8);
    const saved = `${r.savings.inputPct}%`.padStart(8);
    console.log(`${label} ${naive} ${sensei} ${saved}`);
  }
  console.log("─".repeat(54));
  const totNaive = naiveTotalInput.toString().padStart(34);
  const totSensei = senseiTotalInput.toString().padStart(8);
  const totSaved = `${report.summary.inputSavingsPct}%`.padStart(8);
  console.log(`${"TOTAL (input tokens)".padEnd(26)} ${totNaive} ${totSensei} ${totSaved}`);
  console.log();
  console.log(`  input token savings:  ${report.summary.inputSavings.toLocaleString()} tokens (${report.summary.inputSavingsPct}%)`);
  console.log(`  total token savings:  ${report.summary.totalSavings.toLocaleString()} tokens (${report.summary.totalSavingsPct}%)`);

  // ── Write JSON ────────────────────────────────────────────────────────────
  const outPath = opts.output ?? "benchmark-results.json";
  await writeFile(outPath, JSON.stringify(report, null, 2));
  console.log(`\n  results written → ${outPath}`);
}
