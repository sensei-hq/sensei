import { readFileSync, existsSync } from "fs";
import { mkdir, writeFile } from "fs/promises";
import { execSync } from "child_process";
import { join } from "path";
import { intro, outro, spinner, note, log, confirm, isCancel } from "@clack/prompts";
import type { SymbolMap } from "@sensei/shared";
import { callClaude } from "../claude.js";
import {
  getCurrentBranch, isCleanWorkingTree, createAndCheckoutBranch,
  checkoutBranch, stageFiles, commitFiles, branchExists,
} from "../git.js";
import { generateRunName } from "../names.js";
import { senseiPath } from "@sensei/shared";

// ── Prompt builder ────────────────────────────────────────────────────────────

export function buildPopulatePrompt(repoPath: string, skillContent: string | null): string {
  // README — first 60 lines
  const readmePath = join(repoPath, "README.md");
  const readmeLines = existsSync(readmePath)
    ? readFileSync(readmePath, "utf-8").split("\n").slice(0, 60).join("\n")
    : "(no README found)";

  // Symbol-map — source files only (no .spec., no .test., no .md)
  const symbolMapPath = senseiPath(repoPath, "symbol-map.json");
  const symbolMap: SymbolMap = existsSync(symbolMapPath)
    ? JSON.parse(readFileSync(symbolMapPath, "utf-8"))
    : {};

  const sourceEntries = Object.entries(symbolMap)
    .filter(([p]) => !p.endsWith(".md") && !p.includes(".spec.") && !p.includes(".test."))
    .map(([p, entry]) => `${p}: ${entry.L0.slice(0, 4).join(", ") || "(no exports)"}`)
    .join("\n");

  // Doc files — exclude plans/ and templates/
  const docEntries = Object.entries(symbolMap)
    .filter(([p]) => p.endsWith(".md") && !p.includes("plans/") && !p.includes("templates/"))
    .map(([p, entry]) => `${p}: ${entry.L0.slice(0, 3).join(", ") || "(no headings)"}`)
    .join("\n");

  // Current llmspec skeleton — strip docs/plans/ and docs/templates/ entries
  const llmspecPath = senseiPath(repoPath, "llmspec.yaml");
  const llmspecRaw = existsSync(llmspecPath)
    ? readFileSync(llmspecPath, "utf-8")
    : "(no llmspec.yaml found)";
  // Remove YAML list items whose path is under docs/plans/ or docs/templates/
  // Each entry looks like:  "  - path: docs/plans/...\n    covers: []"
  const llmspecContent = (() => {
    const lines = llmspecRaw.split("\n");
    const filtered: string[] = [];
    let skip = false;
    for (const line of lines) {
      if (/[ \t]*- path:\s*(docs\/plans\/|docs\/templates\/)/.test(line)) {
        skip = true;
        continue;
      }
      if (skip && /^[ \t]+\S/.test(line)) continue;
      skip = false;
      filtered.push(line);
    }
    return filtered.join("\n");
  })();

  const skillSection = skillContent
    ? `${skillContent}\n\n---\n\n`
    : "";

  return `${skillSection}You are filling in the semantic fields of .sensei/llmspec.yaml.

## README (first 60 lines)
${readmeLines}

## Source files (path → key exports)
${sourceEntries}

## Documentation files (path → headings)
${docEntries}

## Current llmspec.yaml skeleton (fill the TODO fields)
${llmspecContent}

## Instructions

Fill all TODO fields with accurate values based on the repo context above.
For docs[].covers[]:
- Only include files the doc directly documents (explains its API, algorithm, or design)
- Exclude: test/spec files, config files, stub files
- Leave docs/plans/ and docs/templates/ entries with empty covers: []
- Use exact paths from the source files list above

Respond with the complete filled llmspec.yaml content only.
No preamble, no explanation, no markdown code fences.`;
}

// ── Output parser ─────────────────────────────────────────────────────────────

export function parseYamlOutput(text: string): string {
  // Strip markdown code fences (```yaml ... ``` or ``` ... ```)
  const fenceMatch = text.match(/```(?:yaml)?\n([\s\S]*?)\n```/);
  if (fenceMatch) return fenceMatch[1].trim();
  return text.trim();
}

// ── Score parser ──────────────────────────────────────────────────────────────

export function parsePopulateScore(output: string): number {
  const match = output.match(/llmspec coverage score:\s*(\d+)\/100/);
  return match ? parseInt(match[1], 10) : 0;
}

// ── Skill reader ──────────────────────────────────────────────────────────────

function readSkillContent(repoPath: string): string | null {
  const skillPath = join(repoPath, "skills/populate-llmspec/SKILL.md");
  if (!existsSync(skillPath)) return null;
  const raw = readFileSync(skillPath, "utf-8");
  // Strip YAML frontmatter (--- ... ---)
  return raw.replace(/^---[\s\S]*?---\n/, "").trim();
}

// ── Score script runner ───────────────────────────────────────────────────────

// Note: score-coverage.ts resolves .sensei/llmspec-expected.yaml relative to its own
// repo root (the directory containing tasks/). Passing repoPath as cwd does not change
// where the expected file is looked up — only the generated llmspecPath is an argument.
function runScoreScript(repoPath: string, llmspecPath: string): string {
  const scoreScript = join(repoPath, "tasks/score-coverage.ts");
  if (!existsSync(scoreScript)) return "";
  try {
    return execSync(`bun ${scoreScript} ${llmspecPath}`, {
      cwd: repoPath,
      encoding: "utf-8",
    });
  } catch (err: unknown) {
    const e = err as { stdout?: string };
    return e.stdout ?? "";
  }
}

// ── Main command ──────────────────────────────────────────────────────────────

interface StrategyRun {
  tokensIn: number;
  tokensOut: number;
  elapsedMs: number;
  coverageScore: number;
}

export async function benchmarkPopulate(repoPath: string): Promise<void> {
  intro("sensei benchmark populate");

  // ── Preconditions ────────────────────────────────────────────────────────────
  const llmspecPath = senseiPath(repoPath, "llmspec.yaml");
  if (!existsSync(llmspecPath)) {
    log.error(".sensei/llmspec.yaml not found. Run: sensei init");
    outro("Aborted.");
    return;
  }

  // score-coverage.ts resolves llmspec-expected.yaml relative to its own repo root.
  // This benchmark only produces valid scores when run inside the skills repo.
  const scoreScriptPath = join(repoPath, "tasks/score-coverage.ts");
  if (!existsSync(scoreScriptPath)) {
    log.error("sensei: tasks/score-coverage.ts not found. This benchmark must be run from the skills repo.");
    outro("Aborted.");
    return;
  }

  if (!isCleanWorkingTree(repoPath)) {
    log.error("Working tree is not clean. Please commit or stash your changes before running benchmark populate.");
    outro("Aborted.");
    return;
  }

  const baseBranch = getCurrentBranch(repoPath);
  if (baseBranch === "HEAD") {
    log.error("Detached HEAD state detected. Please checkout a branch before running benchmark populate.");
    outro("Aborted.");
    return;
  }

  // ── Read skill content ────────────────────────────────────────────────────────
  let skillContent: string | null = null;
  try {
    skillContent = readSkillContent(repoPath);
    if (skillContent === null) {
      log.warn("skills/populate-llmspec/SKILL.md not found — Strategy B will run without skill content.");
    }
  } catch {
    log.warn("Could not read skills/populate-llmspec/SKILL.md — Strategy B will run without skill content.");
  }

  // ── Build prompts ─────────────────────────────────────────────────────────────
  const promptA = buildPopulatePrompt(repoPath, null);
  const promptB = buildPopulatePrompt(repoPath, skillContent);

  // ── Generate run name and check branches ─────────────────────────────────────
  const runName = generateRunName();
  const branches = { a: `benchmark/${runName}-a`, b: `benchmark/${runName}-b` } as const;

  for (const branch of Object.values(branches)) {
    if (branchExists(repoPath, branch)) {
      log.error(`Branch already exists: ${branch}. Re-run to get a new run name.`);
      outro("Aborted.");
      return;
    }
  }

  const strategyNames = {
    a: "Baseline (no skill)",
    b: "With populate-llmspec skill",
  } as const;

  // ── Permission prompt (before any API calls or git ops) ───────────────────────
  const gitOpsLines = [
    `git checkout -b ${branches.a} ${baseBranch}`,
    `  → Claude API call: "${strategyNames.a}" strategy (${promptA.length.toLocaleString()} chars)`,
    `  git add .sensei/llmspec.yaml && git commit`,
    `git checkout ${baseBranch}`,
    `git checkout -b ${branches.b} ${baseBranch}`,
    `  → Claude API call: "${strategyNames.b}" strategy (${promptB.length.toLocaleString()} chars)`,
    `  git add .sensei/llmspec.yaml && git commit`,
    `git checkout ${baseBranch}`,
    `[score each branch against .sensei/llmspec-expected.yaml]`,
    `[write .sensei/benchmark-populate-${runName}.json on each branch]`,
    `git checkout benchmark/${runName}-<winner>`,
  ];

  log.info(`sensei will perform these git operations:\n  ${gitOpsLines.join("\n  ")}`);
  const proceed = await confirm({ message: "Proceed?" });
  if (isCancel(proceed) || !proceed) {
    outro("Cancelled.");
    return;
  }

  // ── Interleaved: create branch → run strategy → score → commit → back ─────────
  const sp = spinner();
  const strategyRuns = {} as Record<"a" | "b", StrategyRun>;
  const strategyPrompts = { a: promptA, b: promptB };
  const strategies: Array<"a" | "b"> = ["a", "b"];

  for (const key of strategies) {
    createAndCheckoutBranch(repoPath, branches[key], baseBranch);

    sp.start(`Strategy ${key.toUpperCase()}: ${strategyNames[key]}...`);
    const t0 = Date.now();
    const result = await callClaude(strategyPrompts[key]);
    const elapsedMs = Date.now() - t0;
    sp.stop(
      `Strategy ${key.toUpperCase()} done ` +
      `(${result.usage.tokensIn}→${result.usage.tokensOut} tokens, ` +
      `${(elapsedMs / 1000).toFixed(1)}s)`
    );

    const yamlText = parseYamlOutput(result.text);
    await writeFile(llmspecPath, yamlText, "utf-8");

    const scoreOutput = runScoreScript(repoPath, llmspecPath);
    const coverageScore = parsePopulateScore(scoreOutput);
    log.info(`  Score: ${coverageScore}/100`);

    strategyRuns[key] = {
      tokensIn: result.usage.tokensIn,
      tokensOut: result.usage.tokensOut,
      elapsedMs,
      coverageScore,
    };

    stageFiles(repoPath, [llmspecPath]);
    commitFiles(repoPath, `chore: sensei benchmark populate using "${strategyNames[key]}": .sensei/llmspec.yaml`);

    checkoutBranch(repoPath, baseBranch);
  }

  // ── Determine winner ──────────────────────────────────────────────────────────
  const winner: "a" | "b" = strategyRuns.b.coverageScore >= strategyRuns.a.coverageScore ? "b" : "a";

  // ── Build results JSON ────────────────────────────────────────────────────────
  const senseiJsonPath = senseiPath(repoPath, `benchmark-populate-${runName}.json`);
  const resultsData = {
    run: runName,
    baseBranch,
    branches,
    autoPromoted: winner,
    userFeedback: null,
    promoted: null,
    report: {
      id: crypto.randomUUID(),
      timestamp: new Date().toISOString(),
      strategies: {
        a: { name: strategyNames.a, promptChars: promptA.length },
        b: { name: strategyNames.b, promptChars: promptB.length },
      },
      results: {
        a: { ...strategyRuns.a },
        b: { ...strategyRuns.b },
      },
    },
  };

  // ── Write JSON on each branch ─────────────────────────────────────────────────
  for (const key of strategies) {
    checkoutBranch(repoPath, branches[key]);
    await mkdir(senseiPath(repoPath), { recursive: true });
    await writeFile(senseiJsonPath, JSON.stringify(resultsData, null, 2), "utf-8");
    stageFiles(repoPath, [senseiJsonPath]);
    commitFiles(repoPath, `chore: add benchmark-populate scores for ${runName}`);
    checkoutBranch(repoPath, baseBranch);
  }

  // ── Checkout winner ───────────────────────────────────────────────────────────
  checkoutBranch(repoPath, branches[winner]);

  // ── Announce results ──────────────────────────────────────────────────────────
  note(
    `Run: ${runName}\n` +
    `A (${strategyNames.a}): score=${strategyRuns.a.coverageScore}/100 tokens=${strategyRuns.a.tokensIn}→${strategyRuns.a.tokensOut} time=${(strategyRuns.a.elapsedMs / 1000).toFixed(1)}s\n` +
    `B (${strategyNames.b}): score=${strategyRuns.b.coverageScore}/100 tokens=${strategyRuns.b.tokensIn}→${strategyRuns.b.tokensOut} time=${(strategyRuns.b.elapsedMs / 1000).toFixed(1)}s\n` +
    `Winner: ${winner.toUpperCase()} → ${branches[winner]}`
  );
  outro("Done.");
}
