# benchmark populate Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add `sensei benchmark populate` — a two-strategy A/B benchmark comparing Claude-without-skill vs Claude-with-populate-llmspec-skill, measuring tokensIn, tokensOut, elapsedMs, and coverage score (0-100) for each.

**Architecture:** Follows benchmark-doctor pattern exactly: two git branches (one per strategy), callClaude for each, write filled llmspec.yaml output to the branch, score with `bun tasks/score-coverage.ts`, commit results JSON on each branch, leave winner branch checked out. Strategy A sends repo context only. Strategy B prepends the populate-llmspec skill protocol. No LLM judge — score-coverage.ts provides an objective 0-100 score.

**Tech Stack:** TypeScript, @clack/prompts, callClaude, js-yaml, existing git.ts utilities, bun execSync for scoring

---

### Task 1: `buildPopulatePrompt` + `parseYamlOutput` + `parsePopulateScore` + tests

**Files:**
- Create: `packages/cli/src/commands/benchmark-populate.ts`
- Create: `packages/cli/src/commands/benchmark-populate.spec.ts`

**Step 1: Write the failing tests**

Create `packages/cli/src/commands/benchmark-populate.spec.ts`:

```typescript
import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { mkdirSync, writeFileSync, rmSync } from "fs";
import { join } from "path";
import { buildPopulatePrompt, parseYamlOutput, parsePopulateScore } from "./benchmark-populate.js";

const TMP = "/tmp/sensei-populate-test";

beforeEach(() => {
  mkdirSync(join(TMP, ".sensei"), { recursive: true });
  writeFileSync(join(TMP, "README.md"), "# Sensei\n\nA CLI toolchain for AI skills.\n\nMore details here.\n");
  writeFileSync(join(TMP, ".sensei/symbol-map.json"), JSON.stringify({
    "packages/cli/src/cli.ts": { L0: ["export async function main"], L1: ["CLI entry point"], L2: [] },
    "packages/cli/src/cli.spec.ts": { L0: ["describe"], L1: ["tests"], L2: [] },
    "docs/design/01-overview.md": { L0: ["# Overview", "## Architecture"], L1: [], L2: [] },
    "docs/plans/2026-plan.md": { L0: ["# Plan"], L1: [], L2: [] },
  }));
  writeFileSync(join(TMP, ".sensei/llmspec.yaml"), [
    "project: sensei",
    "description: TODO: one-sentence summary",
    "concepts: []",
    "patterns: []",
    "docs:",
    "  - path: docs/design/01-overview.md",
    "    covers: []",
    "  - path: docs/plans/2026-plan.md",
    "    covers: []",
  ].join("\n"));
});

afterEach(() => rmSync(TMP, { recursive: true, force: true }));

describe("buildPopulatePrompt", () => {
  it("includes README content", () => {
    const prompt = buildPopulatePrompt(TMP, null);
    expect(prompt).toContain("A CLI toolchain for AI skills");
  });

  it("includes source files but excludes .spec. files", () => {
    const prompt = buildPopulatePrompt(TMP, null);
    expect(prompt).toContain("packages/cli/src/cli.ts");
    expect(prompt).not.toContain("packages/cli/src/cli.spec.ts");
  });

  it("includes non-plan doc paths", () => {
    const prompt = buildPopulatePrompt(TMP, null);
    expect(prompt).toContain("docs/design/01-overview.md");
  });

  it("excludes docs/plans/ from doc headings section", () => {
    const prompt = buildPopulatePrompt(TMP, null);
    expect(prompt).not.toContain("docs/plans/2026-plan.md");
  });

  it("includes current llmspec skeleton", () => {
    const prompt = buildPopulatePrompt(TMP, null);
    expect(prompt).toContain("TODO: one-sentence summary");
  });

  it("does NOT include skill section when skillContent is null", () => {
    const prompt = buildPopulatePrompt(TMP, null);
    expect(prompt).not.toContain("populate-llmspec skill");
  });

  it("prepends skill content when skillContent is provided", () => {
    const prompt = buildPopulatePrompt(TMP, "## populate-llmspec skill protocol\nStep 1: do this");
    expect(prompt).toContain("populate-llmspec skill protocol");
    // Skill section should appear before the repo context
    const skillIdx = prompt.indexOf("populate-llmspec skill protocol");
    const readmeIdx = prompt.indexOf("A CLI toolchain");
    expect(skillIdx).toBeLessThan(readmeIdx);
  });
});

describe("parseYamlOutput", () => {
  it("returns raw YAML when no code fences", () => {
    const raw = "project: foo\ndescription: bar";
    expect(parseYamlOutput(raw)).toBe(raw);
  });

  it("strips ```yaml ... ``` code fences", () => {
    const raw = "```yaml\nproject: foo\ndescription: bar\n```";
    expect(parseYamlOutput(raw)).toBe("project: foo\ndescription: bar");
  });

  it("strips ``` ... ``` code fences without language tag", () => {
    const raw = "```\nproject: foo\n```";
    expect(parseYamlOutput(raw)).toBe("project: foo");
  });

  it("strips leading/trailing prose if yaml block is present", () => {
    const raw = "Here is the YAML:\n```yaml\nproject: foo\n```\nDone.";
    expect(parseYamlOutput(raw)).toBe("project: foo");
  });
});

describe("parsePopulateScore", () => {
  it("extracts score from score-coverage.ts output", () => {
    const output = "\nllmspec coverage score: 87/100\n  description:   ✓ (10pts)\n";
    expect(parsePopulateScore(output)).toBe(87);
  });

  it("returns 0 when pattern not found", () => {
    expect(parsePopulateScore("no score here")).toBe(0);
  });

  it("returns 100 for perfect score", () => {
    const output = "llmspec coverage score: 100/100";
    expect(parsePopulateScore(output)).toBe(100);
  });
});
```

**Step 2: Run the tests to confirm they fail**

```bash
cd /Users/Jerry/Developer/skills
bun vitest run packages/cli/src/commands/benchmark-populate.spec.ts 2>&1 | head -30
```

Expected: FAIL — `benchmark-populate.js` module not found.

**Step 3: Write the minimal implementation**

Create `packages/cli/src/commands/benchmark-populate.ts` with the pure functions only (no main command yet):

```typescript
import { readFileSync, existsSync } from "fs";
import { join } from "path";
import yaml from "js-yaml";
import type { SymbolMap } from "@sensei/shared";
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

  // Current llmspec skeleton
  const llmspecPath = senseiPath(repoPath, "llmspec.yaml");
  const llmspecContent = existsSync(llmspecPath)
    ? readFileSync(llmspecPath, "utf-8")
    : "(no llmspec.yaml found)";

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
```

**Step 4: Run the tests to confirm they pass**

```bash
bun vitest run packages/cli/src/commands/benchmark-populate.spec.ts
```

Expected: all tests PASS.

**Step 5: Commit**

```bash
git add packages/cli/src/commands/benchmark-populate.ts packages/cli/src/commands/benchmark-populate.spec.ts
git commit -m "feat(benchmark-populate): add buildPopulatePrompt, parseYamlOutput, parsePopulateScore"
```

---

### Task 2: `benchmarkPopulate` main command

**Files:**
- Modify: `packages/cli/src/commands/benchmark-populate.ts` (add main command)

**Step 1: Read the skill content utility**

The command needs to read `skills/populate-llmspec/SKILL.md` and strip the YAML frontmatter block (`--- ... ---`). Add a helper function.

**Step 2: Write the `benchmarkPopulate` function**

Add to `packages/cli/src/commands/benchmark-populate.ts`:

```typescript
import { readFile, mkdir, writeFile } from "fs/promises";
import { execSync } from "child_process";
import { intro, outro, spinner, note, log, confirm, isCancel } from "@clack/prompts";
import { callClaude } from "../claude.js";
import {
  getCurrentBranch, isCleanWorkingTree, createAndCheckoutBranch,
  checkoutBranch, stageFiles, commitFiles, branchExists,
} from "../git.js";
import { generateRunName } from "../names.js";
import { SENSEI_DIR } from "@sensei/shared";

interface StrategyRun {
  tokensIn: number;
  tokensOut: number;
  elapsedMs: number;
  coverageScore: number;
}

function readSkillContent(repoPath: string): string | null {
  const skillPath = join(repoPath, "skills/populate-llmspec/SKILL.md");
  if (!existsSync(skillPath)) return null;
  const raw = readFileSync(skillPath, "utf-8");
  // Strip YAML frontmatter (--- ... ---)
  return raw.replace(/^---[\s\S]*?---\n/, "").trim();
}

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

export async function benchmarkPopulate(repoPath: string): Promise<void> {
  intro("sensei benchmark populate");

  // ── Preconditions ──────────────────────────────────────────────────────────
  const llmspecPath = senseiPath(repoPath, "llmspec.yaml");
  const expectedPath = senseiPath(repoPath, "llmspec-expected.yaml");

  if (!existsSync(llmspecPath)) {
    log.error("sensei: .sensei/llmspec.yaml not found. Run 'sensei init' first.");
    outro("Aborted.");
    return;
  }
  if (!existsSync(expectedPath)) {
    log.error("sensei: .sensei/llmspec-expected.yaml not found (gold standard required for scoring).");
    outro("Aborted.");
    return;
  }
  if (!isCleanWorkingTree(repoPath)) {
    log.error("Working tree is not clean. Commit or stash your changes before running benchmark populate.");
    outro("Aborted.");
    return;
  }

  const baseBranch = getCurrentBranch(repoPath);
  if (baseBranch === "HEAD") {
    log.error("Detached HEAD state. Please checkout a branch first.");
    outro("Aborted.");
    return;
  }

  // ── Read skill content ─────────────────────────────────────────────────────
  const skillContent = readSkillContent(repoPath);
  if (!skillContent) {
    log.warn("populate-llmspec skill not found at skills/populate-llmspec/SKILL.md — Strategy B will run without skill protocol.");
  }

  // ── Build prompts ──────────────────────────────────────────────────────────
  const promptA = buildPopulatePrompt(repoPath, null);
  const promptB = buildPopulatePrompt(repoPath, skillContent ?? null);

  // ── Generate run name + check branches ────────────────────────────────────
  const runName = generateRunName();
  const branches = {
    a: `benchmark/${runName}-a`,
    b: `benchmark/${runName}-b`,
  } as const;

  for (const branch of Object.values(branches)) {
    if (branchExists(repoPath, branch)) {
      log.error(`Branch already exists: ${branch}. Re-run to get a new run name.`);
      outro("Aborted.");
      return;
    }
  }

  const senseiJsonPath = senseiPath(repoPath, `benchmark-populate-${runName}.json`);
  const strategyNames = { a: "Baseline (no skill)", b: "With populate-llmspec skill" } as const;

  // ── Permission prompt ──────────────────────────────────────────────────────
  log.info(`sensei will perform these git operations:
  git checkout -b ${branches.a} ${baseBranch}
    → Claude API call: "Baseline" strategy (${promptA.length.toLocaleString()} chars)
    git add .sensei/llmspec.yaml && git commit
  git checkout ${baseBranch}
  git checkout -b ${branches.b} ${baseBranch}
    → Claude API call: "With skill" strategy (${promptB.length.toLocaleString()} chars)
    git add .sensei/llmspec.yaml && git commit
  git checkout ${baseBranch}
  [score each branch against .sensei/llmspec-expected.yaml]
  [write .sensei/benchmark-populate-${runName}.json on each branch]
  git checkout benchmark/${runName}-<winner>`);

  const proceed = await confirm({ message: "Proceed?" });
  if (isCancel(proceed) || !proceed) {
    outro("Cancelled.");
    return;
  }

  // ── Interleaved: branch → run → write llmspec → score → commit → back ─────
  const sp = spinner();
  const strategyRuns = {} as Record<"a" | "b", StrategyRun>;
  const strategyPrompts = { a: promptA, b: promptB };
  const strategies: Array<"a" | "b"> = ["a", "b"];

  for (const key of strategies) {
    const branch = branches[key];
    const stratName = strategyNames[key];

    createAndCheckoutBranch(repoPath, branch, baseBranch);

    sp.start(`Strategy ${key.toUpperCase()}: ${stratName}...`);
    const t0 = Date.now();
    const result = await callClaude(strategyPrompts[key]);
    const elapsedMs = Date.now() - t0;
    sp.stop(`Strategy ${key.toUpperCase()} done (${result.usage.tokensIn}→${result.usage.tokensOut} tokens, ${(elapsedMs / 1000).toFixed(1)}s)`);

    // Parse and write llmspec.yaml output
    const yamlText = parseYamlOutput(result.text);
    await writeFile(llmspecPath, yamlText, "utf-8");

    // Score against gold standard
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
    commitFiles(repoPath, `chore: sensei benchmark populate using "${stratName}"`);

    checkoutBranch(repoPath, baseBranch);
  }

  // ── Determine winner ───────────────────────────────────────────────────────
  const winner: "a" | "b" = strategyRuns.b.coverageScore >= strategyRuns.a.coverageScore ? "b" : "a";

  // ── Build results data ─────────────────────────────────────────────────────
  const resultsData = {
    run: runName,
    baseBranch,
    branches,
    autoPromoted: winner,
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

  // ── Write JSON on each branch ──────────────────────────────────────────────
  for (const key of strategies) {
    checkoutBranch(repoPath, branches[key]);
    await mkdir(senseiPath(repoPath), { recursive: true });
    await writeFile(senseiJsonPath, JSON.stringify(resultsData, null, 2), "utf-8");
    stageFiles(repoPath, [senseiJsonPath]);
    commitFiles(repoPath, `chore: add benchmark-populate scores for ${runName}`);
    checkoutBranch(repoPath, baseBranch);
  }

  // ── Checkout winner ────────────────────────────────────────────────────────
  checkoutBranch(repoPath, branches[winner]);

  // ── Announce results ───────────────────────────────────────────────────────
  note(
    `Run: ${runName}\n` +
    `A (${strategyNames.a}): score=${strategyRuns.a.coverageScore}/100 ` +
    `tokens=${strategyRuns.a.tokensIn}→${strategyRuns.a.tokensOut} ` +
    `time=${(strategyRuns.a.elapsedMs / 1000).toFixed(1)}s\n` +
    `B (${strategyNames.b}): score=${strategyRuns.b.coverageScore}/100 ` +
    `tokens=${strategyRuns.b.tokensIn}→${strategyRuns.b.tokensOut} ` +
    `time=${(strategyRuns.b.elapsedMs / 1000).toFixed(1)}s\n` +
    `Winner: ${winner.toUpperCase()} → ${branches[winner]}`
  );
  outro("Done.");
}
```

**Step 3: Run the full test suite**

```bash
bun vitest run packages/cli/src/commands/benchmark-populate.spec.ts
```

Expected: all tests still PASS (pure functions unchanged).

**Step 4: Build check**

```bash
bun run build 2>&1 | tail -20
```

Expected: no TypeScript errors.

**Step 5: Commit**

```bash
git add packages/cli/src/commands/benchmark-populate.ts
git commit -m "feat(benchmark-populate): add benchmarkPopulate command"
```

---

### Task 3: Wire into CLI

**Files:**
- Modify: `packages/cli/src/cli.ts`

**Step 1: Add `benchmark populate` subcommand**

In `cli.ts`, inside the `case "benchmark":` block, after `} else if (subCmd === "coverage") {`, add:

```typescript
} else if (subCmd === "populate") {
  const { benchmarkPopulate } = await import("./commands/benchmark-populate.js");
  await benchmarkPopulate(repoRoot);
```

**Step 2: Add help text**

In the `HELP` string, after the `benchmark coverage:` block, add:

```
benchmark populate:
  Compares Claude-without-skill vs Claude-with-populate-llmspec-skill.
  Scores each strategy with score-coverage.ts and reports tokens, time, score.
  Requires: .sensei/llmspec.yaml and .sensei/llmspec-expected.yaml

```

**Step 3: Build and verify**

```bash
bun run build 2>&1 | tail -20
```

Expected: no TypeScript errors.

**Step 4: Run full test suite**

```bash
bun vitest run 2>&1 | tail -20
```

Expected: all existing tests pass (≥125 tests).

**Step 5: Commit**

```bash
git add packages/cli/src/cli.ts
git commit -m "feat(cli): wire benchmark populate subcommand"
```

---

### Task 4: Verify end-to-end (smoke test)

**Step 1: Build**

```bash
bun run build
```

**Step 2: Check help output includes benchmark populate**

```bash
node packages/cli/dist/cli.js benchmark populate --help 2>&1 | head -5
# OR: check help text appears
node packages/cli/dist/cli.js --help | grep -A3 "benchmark populate"
```

Expected: help text mentions benchmark populate.

**Step 3: Commit plan as artifact**

```bash
git add docs/plans/2026-03-07-benchmark-populate.md
git commit -m "docs: add benchmark-populate implementation plan"
```
