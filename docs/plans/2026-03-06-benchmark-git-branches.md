# Benchmark Git-Branch Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace folder-based benchmark output with git branches — one branch per strategy — committed and annotated with semantic messages, with `inspect` and `promote` commands for branch navigation and merge.

**Architecture:** `benchmarkDoctor` creates 3 branches from HEAD, commits strategy outputs to each, then stays on the winner branch. `benchmarkInspect` checks out a branch. `benchmarkPromote` merges the chosen branch and offers to delete the others. A shared `git.ts` utility wraps all `execSync` calls. `serve.ts` and `index.ts` are unchanged.

**Tech Stack:** TypeScript, Bun, `@clack/prompts`, `child_process.execSync`, `vitest`

**Design doc:** `docs/plans/2026-03-06-benchmark-git-branches-design.md`

---

## Task 1: Git utilities

**Files:**
- Create: `packages/sensei/src/git.ts`
- Create: `packages/sensei/src/git.spec.ts`

**Step 1: Write the failing tests**

```typescript
// packages/sensei/src/git.spec.ts
import { describe, it, expect, vi, beforeEach } from "vitest";

vi.mock("child_process", () => ({ execSync: vi.fn() }));
import { execSync } from "child_process";

import {
  getCurrentBranch,
  isCleanWorkingTree,
  branchExists,
  readFileFromBranch,
} from "./git.js";

const REPO = "/fake/repo";
const mockExec = execSync as ReturnType<typeof vi.fn>;

beforeEach(() => vi.clearAllMocks());

describe("getCurrentBranch", () => {
  it("returns trimmed branch name", () => {
    mockExec.mockReturnValueOnce("main\n");
    expect(getCurrentBranch(REPO)).toBe("main");
    expect(mockExec).toHaveBeenCalledWith("git rev-parse --abbrev-ref HEAD", { cwd: REPO, encoding: "utf-8" });
  });
});

describe("isCleanWorkingTree", () => {
  it("returns true when output is empty", () => {
    mockExec.mockReturnValueOnce("");
    expect(isCleanWorkingTree(REPO)).toBe(true);
  });
  it("returns false when files are modified", () => {
    mockExec.mockReturnValueOnce(" M src/foo.ts\n");
    expect(isCleanWorkingTree(REPO)).toBe(false);
  });
});

describe("branchExists", () => {
  it("returns true when branch is found", () => {
    mockExec.mockReturnValueOnce("benchmark/wild-cat-a\n");
    expect(branchExists(REPO, "benchmark/wild-cat-a")).toBe(true);
  });
  it("returns false when branch is not found", () => {
    mockExec.mockReturnValueOnce("");
    expect(branchExists(REPO, "benchmark/wild-cat-a")).toBe(false);
  });
});

describe("readFileFromBranch", () => {
  it("returns file content from branch", () => {
    mockExec.mockReturnValueOnce('{"run":"wild-cat"}');
    expect(readFileFromBranch(REPO, "benchmark/wild-cat-a", ".sensei/benchmark-wild-cat.json"))
      .toBe('{"run":"wild-cat"}');
    expect(mockExec).toHaveBeenCalledWith(
      'git show benchmark/wild-cat-a:.sensei/benchmark-wild-cat.json',
      { cwd: REPO, encoding: "utf-8" },
    );
  });
});
```

**Step 2: Run tests to verify they fail**

```bash
cd /Users/Jerry/Developer/skills
bun test packages/sensei/src/git.spec.ts
```

Expected: FAIL — `Cannot find module './git.js'`

**Step 3: Implement `git.ts`**

```typescript
// packages/sensei/src/git.ts
import { execSync } from "child_process";

function exec(cmd: string, cwd: string): string {
  return execSync(cmd, { cwd, encoding: "utf-8" }) as string;
}

export function getCurrentBranch(repoPath: string): string {
  return exec("git rev-parse --abbrev-ref HEAD", repoPath).trim();
}

export function isCleanWorkingTree(repoPath: string): boolean {
  return exec("git status --porcelain", repoPath).trim() === "";
}

export function branchExists(repoPath: string, branch: string): boolean {
  return exec(`git branch --list ${branch}`, repoPath).trim() !== "";
}

export function createAndCheckoutBranch(repoPath: string, branch: string, from: string): void {
  exec(`git checkout -b ${branch} ${from}`, repoPath);
}

export function checkoutBranch(repoPath: string, branch: string): void {
  exec(`git checkout ${branch}`, repoPath);
}

export function stageFiles(repoPath: string, paths: string[]): void {
  exec(`git add ${paths.map(p => `"${p}"`).join(" ")}`, repoPath);
}

export function commitFiles(repoPath: string, message: string): void {
  exec(`git commit -m ${JSON.stringify(message)}`, repoPath);
}

export function mergeBranch(repoPath: string, branch: string): void {
  exec(`git merge ${branch}`, repoPath);
}

export function deleteBranch(repoPath: string, branch: string): void {
  exec(`git branch -d ${branch}`, repoPath);
}

export function readFileFromBranch(repoPath: string, branch: string, filePath: string): string {
  return exec(`git show ${branch}:${filePath}`, repoPath);
}
```

**Step 4: Run tests to verify they pass**

```bash
bun test packages/sensei/src/git.spec.ts
```

Expected: PASS (4 tests)

**Step 5: Commit**

```bash
cd /Users/Jerry/Developer/skills
git add packages/sensei/src/git.ts packages/sensei/src/git.spec.ts
git commit -m "feat(sensei): git utilities for benchmark branch operations"
```

---

## Task 2: Run name generation

**Files:**
- Create: `packages/sensei/src/names.ts`
- Create: `packages/sensei/src/names.spec.ts`

**Step 1: Write the failing tests**

```typescript
// packages/sensei/src/names.spec.ts
import { describe, it, expect } from "vitest";
import { generateRunName } from "./names.js";

describe("generateRunName", () => {
  it("returns adjective-noun format", () => {
    const name = generateRunName();
    expect(name).toMatch(/^[a-z]+-[a-z]+$/);
  });

  it("returns different names on repeated calls (probabilistic)", () => {
    const names = new Set(Array.from({ length: 20 }, () => generateRunName()));
    expect(names.size).toBeGreaterThan(1);
  });
});
```

**Step 2: Run to verify failure**

```bash
bun test packages/sensei/src/names.spec.ts
```

Expected: FAIL — `Cannot find module './names.js'`

**Step 3: Implement `names.ts`**

```typescript
// packages/sensei/src/names.ts
const ADJECTIVES = [
  "wild", "blue", "red", "new", "old", "swift", "dark", "bright",
  "cool", "warm", "soft", "bold", "calm", "clear", "deep", "fair",
  "free", "gold", "grey", "high", "jade", "keen", "lean", "lone",
  "mild", "neat", "open", "pale", "pure", "rich", "sage", "tall",
  "true", "vast", "wide", "wise", "young", "zeal",
];

const NOUNS = [
  "cat", "moon", "bird", "river", "ridge", "oak", "pine", "fox",
  "ash", "bay", "cape", "dale", "dune", "fern", "glen", "hawk",
  "isle", "lake", "lark", "leaf", "mead", "mist", "path", "peak",
  "pool", "reed", "rock", "rose", "rune", "sand", "seed", "star",
  "tide", "vale", "wave", "well", "wind", "wren",
];

export function generateRunName(): string {
  const adj = ADJECTIVES[Math.floor(Math.random() * ADJECTIVES.length)];
  const noun = NOUNS[Math.floor(Math.random() * NOUNS.length)];
  return `${adj}-${noun}`;
}
```

**Step 4: Run to verify pass**

```bash
bun test packages/sensei/src/names.spec.ts
```

Expected: PASS (2 tests)

**Step 5: Commit**

```bash
git add packages/sensei/src/names.ts packages/sensei/src/names.spec.ts
git commit -m "feat(sensei): random run name generation for benchmark branches"
```

---

## Task 3: Rewrite `benchmark-doctor.ts`

Replace folder-based output with git branch creation. The pure helper functions (`buildTargetedIndexPrompt`, `buildRawContentPrompt`, `buildFullRepoIndexPrompt`, `parseOutputFolder`, `structuralScore`, `llmJudge`, `pickWinner`) are **unchanged** — only `benchmarkDoctor()` changes.

**Files:**
- Modify: `packages/sensei/src/commands/benchmark-doctor.ts`
- Modify: `packages/sensei/src/commands/benchmark-doctor.spec.ts` (remove folder-output tests, the pure function tests still pass)

**Step 1: Verify pure-function tests still exist and pass before touching anything**

```bash
bun test packages/sensei/src/commands/benchmark-doctor.spec.ts
```

Expected: all pure-function tests PASS (they don't touch `benchmarkDoctor()`)

**Step 2: Replace `benchmarkDoctor()` in `benchmark-doctor.ts`**

Remove everything from line 258 onward (the `BenchmarkDoctorOptions` interface + `benchmarkDoctor` function) and replace with:

```typescript
// ── Main command ──────────────────────────────────────────────────────────────

import { writeFile, mkdir } from "fs/promises";
import { join, relative, extname, dirname } from "path";
import { intro, outro, spinner, note, log, confirm, isCancel } from "@clack/prompts";
import {
  getCurrentBranch, isCleanWorkingTree,
  createAndCheckoutBranch, checkoutBranch,
  stageFiles, commitFiles,
} from "../git.js";
import { generateRunName } from "../names.js";

export interface BenchmarkDoctorOptions {
  template?: string;
  examples?: string;
  sample?: number;
  out?: string;
}

export async function benchmarkDoctor(
  inputDir: string,
  outputName: string,
  repoPath: string,
  opts: BenchmarkDoctorOptions,
): Promise<void> {
  intro("sensei benchmark doctor");

  const fullInputDir = join(repoPath, inputDir);
  if (!existsSync(fullInputDir)) {
    log.error(`Input dir not found: ${fullInputDir}`);
    outro("Aborted.");
    return;
  }

  const templatePath = opts.template
    ? join(repoPath, opts.template)
    : join(repoPath, "docs/templates/feature.md");
  if (!existsSync(templatePath)) {
    log.error(`Template not found: ${templatePath}`);
    outro("Aborted.");
    return;
  }

  // ── Preconditions ────────────────────────────────────────────────────────────
  if (!isCleanWorkingTree(repoPath)) {
    log.error("Working tree has uncommitted changes. Commit or stash before running benchmark.");
    outro("Aborted.");
    return;
  }

  const baseBranch = getCurrentBranch(repoPath);
  if (baseBranch === "HEAD") {
    log.error("Detached HEAD state. Checkout a branch before running benchmark.");
    outro("Aborted.");
    return;
  }

  // ── Setup ────────────────────────────────────────────────────────────────────
  const examplesDir = opts.examples ? join(repoPath, opts.examples) : null;
  const templateContent = await readFile(templatePath, "utf-8");

  const allMd = readdirSync(fullInputDir).filter(f => extname(f) === ".md");
  const sample = opts.sample ? allMd.slice(0, opts.sample) : allMd;
  const original: Record<string, string> = {};
  for (const f of sample) {
    original[f] = await readFile(join(fullInputDir, f), "utf-8");
  }

  // ── Run strategies ───────────────────────────────────────────────────────────
  const sp = spinner();

  const promptA = buildTargetedIndexPrompt({ inputDir: fullInputDir, repoPath, templateContent, outputName });
  const promptB = buildRawContentPrompt({ inputDir: fullInputDir, examplesDir, outputName });
  const promptC = buildFullRepoIndexPrompt({ repoPath, templateContent, outputName });

  sp.start("Strategy A: targeted index...");
  const resultA = await callClaude(promptA);
  const outputA = parseOutputFolder(resultA.text);
  sp.stop(`Strategy A done (${resultA.usage.tokensIn}→${resultA.usage.tokensOut} tokens)`);

  sp.start("Strategy B: raw content...");
  const resultB = await callClaude(promptB);
  const outputB = parseOutputFolder(resultB.text);
  sp.stop(`Strategy B done (${resultB.usage.tokensIn}→${resultB.usage.tokensOut} tokens)`);

  sp.start("Strategy C: full repo index...");
  const resultC = await callClaude(promptC);
  const outputC = parseOutputFolder(resultC.text);
  sp.stop(`Strategy C done (${resultC.usage.tokensIn}→${resultC.usage.tokensOut} tokens)`);

  // ── Score ────────────────────────────────────────────────────────────────────
  sp.start("Scoring...");
  const structA = structuralScore({ template: templateContent, output: outputA, original });
  const structB = structuralScore({ template: templateContent, output: outputB, original });
  const structC = structuralScore({ template: templateContent, output: outputC, original });
  const judge = await llmJudge(original, outputA, outputB, outputC);
  sp.stop("Scoring done");

  const winner = pickWinner(structA, judge.scoreA, structB, judge.scoreB, structC, judge.scoreC);

  // ── Build results JSON ───────────────────────────────────────────────────────
  const runName = generateRunName();
  const branches = {
    a: `benchmark/${runName}-a`,
    b: `benchmark/${runName}-b`,
    c: `benchmark/${runName}-c`,
  };
  const resultsFile = `.sensei/benchmark-${runName}.json`;

  function cleanPaths(s: string): string {
    return s.replaceAll(repoPath + "/", "").replaceAll(repoPath, "");
  }

  const exFiles = examplesDir && existsSync(examplesDir)
    ? readdirSync(examplesDir).filter(f => extname(f) === ".md")
    : [];

  const resultsData = {
    run: runName,
    baseBranch,
    branches,
    autoPromoted: winner,
    userFeedback: null as null | { preferred: string; systemAgreed: boolean; note?: string },
    promoted: null as null | string,
    report: {
      id: crypto.randomUUID(),
      timestamp: new Date().toISOString(),
      scenario: {
        inputFileCount: Object.keys(original).length,
        inputTotalChars: Object.values(original).join("").length,
        inputTotalLines: Object.values(original).join("\n").split("\n").length,
        outputName,
        templateName: relative(repoPath, templatePath),
        examplesProvided: examplesDir !== null,
        examplesFileCount: exFiles.length,
      },
      strategies: {
        a: { name: "Targeted index",  prompt: cleanPaths(promptA), promptChars: promptA.length, promptLines: promptA.split("\n").length },
        b: { name: "Raw content",     prompt: cleanPaths(promptB), promptChars: promptB.length, promptLines: promptB.split("\n").length },
        c: { name: "Full repo index", prompt: cleanPaths(promptC), promptChars: promptC.length, promptLines: promptC.split("\n").length },
      },
      results: {
        a: { tokensIn: resultA.usage.tokensIn, tokensOut: resultA.usage.tokensOut, filesGenerated: Object.keys(outputA).length, structuralScore: structA, judgeScore: judge.scoreA },
        b: { tokensIn: resultB.usage.tokensIn, tokensOut: resultB.usage.tokensOut, filesGenerated: Object.keys(outputB).length, structuralScore: structB, judgeScore: judge.scoreB },
        c: { tokensIn: resultC.usage.tokensIn, tokensOut: resultC.usage.tokensOut, filesGenerated: Object.keys(outputC).length, structuralScore: structC, judgeScore: judge.scoreC },
      },
      userFeedback: null as null | { preferred: string; systemAgreed: boolean; note?: string },
      promoted: null as null | string,
    },
  };

  // ── Permission prompt ────────────────────────────────────────────────────────
  const strategyNames = { a: "Targeted index", b: "Raw content", c: "Full repo index" };
  const gitPlan = (["a", "b", "c"] as const).map(letter => {
    const branch = branches[letter];
    const nFiles = letter === "a" ? Object.keys(outputA).length : letter === "b" ? Object.keys(outputB).length : Object.keys(outputC).length;
    return [
      `  git checkout -b ${branch} ${baseBranch}`,
      `  git add docs/${outputName}/ ${resultsFile}`,
      `  git commit -m "chore: sensei benchmark doctor using \\"${strategyNames[letter]}\\": docs/${outputName} (${nFiles} files)"`,
    ].join("\n");
  }).join("\n") + `\n  git checkout ${branches[winner]}  ← winner`;

  log.info(`sensei will perform these git operations:\n${gitPlan}`);

  const ok = await confirm({ message: "Proceed?" });
  if (isCancel(ok) || !ok) { outro("Cancelled."); return; }

  // ── Create branches and commit ───────────────────────────────────────────────
  const targetDir = join(repoPath, dirname(relative(repoPath, fullInputDir)), outputName);

  async function writeBranch(
    letter: "a" | "b" | "c",
    output: Record<string, string>,
  ): Promise<void> {
    const branch = branches[letter];
    createAndCheckoutBranch(repoPath, branch, baseBranch);

    await mkdir(targetDir, { recursive: true });
    for (const [name, content] of Object.entries(output)) {
      await writeFile(join(targetDir, name), content, "utf-8");
    }

    await mkdir(join(repoPath, ".sensei"), { recursive: true });
    await writeFile(join(repoPath, resultsFile), JSON.stringify(resultsData, null, 2), "utf-8");

    const nFiles = Object.keys(output).length;
    stageFiles(repoPath, [join(repoPath, dirname(resultsFile)), targetDir]);
    commitFiles(repoPath, `chore: sensei benchmark doctor using "${strategyNames[letter]}": docs/${outputName} (${nFiles} files)`);
  }

  sp.start(`Creating branch ${branches.a}...`);
  await writeBranch("a", outputA);
  sp.stop(`Branch ${branches.a} committed`);

  sp.start(`Creating branch ${branches.b}...`);
  await writeBranch("b", outputB);
  sp.stop(`Branch ${branches.b} committed`);

  sp.start(`Creating branch ${branches.c}...`);
  await writeBranch("c", outputC);
  sp.stop(`Branch ${branches.c} committed`);

  // ── Switch to winner ─────────────────────────────────────────────────────────
  checkoutBranch(repoPath, branches[winner]);

  note(
    `Run: ${runName}\n` +
    `A (${strategyNames.a}): struct=${structA} judge=${judge.scoreA}\n` +
    `B (${strategyNames.b}): struct=${structB} judge=${judge.scoreB}\n` +
    `C (${strategyNames.c}): struct=${structC} judge=${judge.scoreC}\n` +
    `Winner: ${winner.toUpperCase()} → ${branches[winner]}\n` +
    `\nTo inspect: sensei benchmark inspect ${runName}-<a|b|c>\n` +
    `To promote: sensei benchmark promote ${runName}`,
    "Benchmark complete"
  );
  outro("Done.");
}
```

**Important:** The imports at the top of `benchmark-doctor.ts` need these added (they may already be present, add only what's missing):
```typescript
import { confirm, isCancel } from "@clack/prompts";
import { getCurrentBranch, isCleanWorkingTree, createAndCheckoutBranch, checkoutBranch, stageFiles, commitFiles } from "../git.js";
import { generateRunName } from "../names.js";
```

And remove these no-longer-needed imports from the top:
- `writeFile`, `mkdir` from `"fs/promises"` — keep these, still needed
- Remove: anything related to `outDir`, `summary.md`, auto-promote file copying

**Step 3: Run all tests to verify pure functions still pass**

```bash
bun test packages/sensei/src/commands/benchmark-doctor.spec.ts
```

Expected: all pure-function tests PASS

**Step 4: Run full test suite**

```bash
bun test
```

Expected: all existing tests pass (the `benchmarkDoctor` integration path isn't unit-tested)

**Step 5: Commit**

```bash
git add packages/sensei/src/commands/benchmark-doctor.ts
git commit -m "feat(sensei): benchmark doctor writes to git branches instead of folders"
```

---

## Task 4: Rewrite `benchmark-promote.ts`

Replace file-copying logic with git merge. Keep `buildFeedback` and `submitReport` (pure helpers, tests pass). Replace `pickPreferred` with a simpler local lookup. Replace `benchmarkPromote()` entirely.

**Files:**
- Modify: `packages/sensei/src/commands/benchmark-promote.ts`
- Modify: `packages/sensei/src/commands/benchmark-promote.spec.ts`

**Step 1: Update the spec — remove `pickPreferred`, keep `buildFeedback` + `submitReport`**

```typescript
// packages/sensei/src/commands/benchmark-promote.spec.ts
import { describe, it, expect, vi, beforeEach } from "vitest";
import { buildFeedback, submitReport } from "./benchmark-promote.js";

describe("buildFeedback", () => {
  it("sets systemAgreed true when user matches auto", () => {
    const fb = buildFeedback("c", "c", "great output");
    expect(fb).toEqual({ preferred: "c", systemAgreed: true, note: "great output" });
  });

  it("sets systemAgreed false when user overrides", () => {
    const fb = buildFeedback("b", "c");
    expect(fb).toEqual({ preferred: "b", systemAgreed: false });
  });
});

describe("submitReport", () => {
  beforeEach(() => {
    global.fetch = vi.fn().mockResolvedValue({ json: async () => ({ ok: true }) });
  });

  it("POSTs to telemetry URL", async () => {
    await submitReport({ id: "test-id" }, "http://localhost:7744");
    expect(global.fetch).toHaveBeenCalledWith(
      "http://localhost:7744/reports",
      expect.objectContaining({ method: "POST" }),
    );
  });

  it("does not throw when server is down", async () => {
    global.fetch = vi.fn().mockRejectedValue(new Error("ECONNREFUSED"));
    await expect(submitReport({ id: "test-id" }, "http://localhost:7744")).resolves.toBeUndefined();
  });
});
```

**Step 2: Run spec to verify it fails (because `pickPreferred` is no longer exported)**

```bash
bun test packages/sensei/src/commands/benchmark-promote.spec.ts
```

Expected: some FAIL due to removed export, or pass if `buildFeedback`/`submitReport` still exist

**Step 3: Rewrite `benchmark-promote.ts`**

```typescript
// packages/sensei/src/commands/benchmark-promote.ts
import { writeFile } from "fs/promises";
import { join } from "path";
import { intro, outro, select, text, isCancel, note, log, confirm } from "@clack/prompts";
import {
  branchExists, checkoutBranch, mergeBranch, deleteBranch, readFileFromBranch,
} from "../git.js";

// ── Pure helpers (exported for testing) ────────────────────────────────────────

export function buildFeedback(
  preferred: string,
  autoPromoted: string,
  noteText?: string,
): { preferred: string; systemAgreed: boolean; note?: string } {
  const fb: { preferred: string; systemAgreed: boolean; note?: string } = {
    preferred,
    systemAgreed: preferred === autoPromoted,
  };
  if (noteText) fb.note = noteText;
  return fb;
}

export async function submitReport(report: unknown, baseUrl: string): Promise<void> {
  try {
    await fetch(`${baseUrl}/reports`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(report),
    });
  } catch {
    // Telemetry is best-effort — never throw
  }
}

// ── Main command ────────────────────────────────────────────────────────────────

export async function benchmarkPromote(runName: string, repoPath: string): Promise<void> {
  intro("sensei benchmark promote");

  const branches = {
    a: `benchmark/${runName}-a`,
    b: `benchmark/${runName}-b`,
    c: `benchmark/${runName}-c`,
  };

  // ── Verify branches exist ────────────────────────────────────────────────────
  for (const [letter, branch] of Object.entries(branches)) {
    if (!branchExists(repoPath, branch)) {
      log.error(`Branch not found: ${branch}`);
      outro("Aborted.");
      return;
    }
  }

  // ── Read results from each branch ────────────────────────────────────────────
  const resultsFile = `.sensei/benchmark-${runName}.json`;
  let data: any;
  try {
    data = JSON.parse(readFileFromBranch(repoPath, branches.a, resultsFile));
  } catch {
    log.error(`Could not read ${resultsFile} from ${branches.a}`);
    outro("Aborted.");
    return;
  }

  const { autoPromoted, baseBranch, report } = data;
  const results = report?.results ?? data.scores;
  const strategyNames: Record<string, string> = {
    a: report?.strategies?.a?.name ?? "Targeted index",
    b: report?.strategies?.b?.name ?? "Raw content",
    c: report?.strategies?.c?.name ?? "Full repo index",
  };

  // ── Display comparison table ─────────────────────────────────────────────────
  note(
    (["a", "b", "c"] as const).map(k => {
      const r = results[k];
      const marker = k === autoPromoted ? " ← auto-winner" : "";
      return `${k.toUpperCase()} (${strategyNames[k]}): struct=${r.structuralScore} judge=${r.judgeScore} tokens=${r.tokensIn}→${r.tokensOut} files=${r.filesGenerated}${marker}`;
    }).join("\n"),
    "Benchmark results"
  );

  // ── Pick strategy ────────────────────────────────────────────────────────────
  const preferred = await select({
    message: "Which strategy to promote?",
    options: [
      { value: "a", label: `A — ${strategyNames.a} (struct=${results.a.structuralScore} judge=${results.a.judgeScore})` },
      { value: "b", label: `B — ${strategyNames.b} (struct=${results.b.structuralScore} judge=${results.b.judgeScore})` },
      { value: "c", label: `C — ${strategyNames.c} (struct=${results.c.structuralScore} judge=${results.c.judgeScore})` },
    ],
  });
  if (isCancel(preferred)) { outro("Cancelled."); return; }

  const noteText = await text({ message: "Optional note (press Enter to skip):", placeholder: "" });
  if (isCancel(noteText)) { outro("Cancelled."); return; }

  // ── Git permission prompt ────────────────────────────────────────────────────
  const chosenBranch = branches[preferred as "a" | "b" | "c"];
  log.info(
    `sensei will perform these git operations:\n` +
    `  git checkout ${baseBranch}\n` +
    `  git merge ${chosenBranch}`
  );
  const okMerge = await confirm({ message: "Proceed with merge?" });
  if (isCancel(okMerge) || !okMerge) { outro("Cancelled."); return; }

  // ── Merge ────────────────────────────────────────────────────────────────────
  checkoutBranch(repoPath, baseBranch);
  mergeBranch(repoPath, chosenBranch);

  // ── Update results JSON ──────────────────────────────────────────────────────
  const feedback = buildFeedback(preferred as string, autoPromoted, (noteText as string) || undefined);
  data.userFeedback = feedback;
  data.promoted = preferred;
  if (data.report) {
    data.report.userFeedback = feedback;
    data.report.promoted = preferred;
  }
  await writeFile(join(repoPath, resultsFile), JSON.stringify(data, null, 2), "utf-8");

  // ── Submit telemetry ─────────────────────────────────────────────────────────
  const telemetryUrl = process.env.SENSEI_TELEMETRY_URL ?? "http://localhost:7744";
  submitReport(data.report ?? data, telemetryUrl).catch(() => {});

  // ── Offer to delete other branches ───────────────────────────────────────────
  const losers = (["a", "b", "c"] as const).filter(k => k !== preferred).map(k => branches[k]);
  log.info(`Other benchmark branches:\n${losers.map(b => `  ${b}`).join("\n")}`);
  const okDelete = await confirm({ message: `Delete ${losers.join(" and ")}?` });
  if (!isCancel(okDelete) && okDelete) {
    for (const branch of losers) {
      deleteBranch(repoPath, branch);
      log.success(`Deleted ${branch}`);
    }
  }

  note(`Promoted: Strategy ${(preferred as string).toUpperCase()} (${strategyNames[preferred as string]})`, "Done");
  outro("Done.");
}
```

**Step 4: Run spec to verify pass**

```bash
bun test packages/sensei/src/commands/benchmark-promote.spec.ts
```

Expected: PASS (4 tests)

**Step 5: Run full suite**

```bash
bun test
```

Expected: all tests pass

**Step 6: Commit**

```bash
git add packages/sensei/src/commands/benchmark-promote.ts packages/sensei/src/commands/benchmark-promote.spec.ts
git commit -m "feat(sensei): benchmark promote merges git branch and offers to delete losers"
```

---

## Task 5: Create `benchmark-inspect.ts`

**Files:**
- Create: `packages/sensei/src/commands/benchmark-inspect.ts`
- Create: `packages/sensei/src/commands/benchmark-inspect.spec.ts`

**Step 1: Write the failing test**

```typescript
// packages/sensei/src/commands/benchmark-inspect.spec.ts
import { describe, it, expect } from "vitest";
import { resolveBranchName } from "./benchmark-inspect.js";

describe("resolveBranchName", () => {
  it("prepends benchmark/ prefix", () => {
    expect(resolveBranchName("wild-cat-b")).toBe("benchmark/wild-cat-b");
  });

  it("does not double-prefix", () => {
    expect(resolveBranchName("benchmark/wild-cat-b")).toBe("benchmark/wild-cat-b");
  });
});
```

**Step 2: Run to verify failure**

```bash
bun test packages/sensei/src/commands/benchmark-inspect.spec.ts
```

Expected: FAIL — `Cannot find module './benchmark-inspect.js'`

**Step 3: Implement `benchmark-inspect.ts`**

```typescript
// packages/sensei/src/commands/benchmark-inspect.ts
import { intro, outro, log, confirm, isCancel } from "@clack/prompts";
import { branchExists, checkoutBranch } from "../git.js";

export function resolveBranchName(runBranch: string): string {
  return runBranch.startsWith("benchmark/") ? runBranch : `benchmark/${runBranch}`;
}

export async function benchmarkInspect(runBranch: string, repoPath: string): Promise<void> {
  intro("sensei benchmark inspect");

  const branch = resolveBranchName(runBranch);

  if (!branchExists(repoPath, branch)) {
    log.error(`Branch not found: ${branch}`);
    outro("Aborted.");
    return;
  }

  log.info(`sensei will perform:\n  git checkout ${branch}`);
  const ok = await confirm({ message: "Proceed?" });
  if (isCancel(ok) || !ok) { outro("Cancelled."); return; }

  checkoutBranch(repoPath, branch);
  log.success(`Switched to ${branch}`);
  outro("Done.");
}
```

**Step 4: Run to verify pass**

```bash
bun test packages/sensei/src/commands/benchmark-inspect.spec.ts
```

Expected: PASS (2 tests)

**Step 5: Commit**

```bash
git add packages/sensei/src/commands/benchmark-inspect.ts packages/sensei/src/commands/benchmark-inspect.spec.ts
git commit -m "feat(sensei): benchmark inspect switches to a strategy branch"
```

---

## Task 6: Wire `cli.ts`

**Files:**
- Modify: `packages/sensei/src/cli.ts`

**Step 1: Add `benchmark inspect` subcommand**

In `cli.ts`, inside the `case "benchmark":` block, find the `else if (subCmd === "promote")` block and add a new branch before the final `else`:

```typescript
} else if (subCmd === "inspect") {
  const { benchmarkInspect } = await import("./commands/benchmark-inspect.js");
  const runBranch = rest[1];
  if (!runBranch) {
    console.error("Usage: sensei benchmark inspect <run>-<letter>");
    process.exit(1);
  }
  await benchmarkInspect(runBranch, process.cwd());
```

**Step 2: Update `benchmarkPromote` call signature**

The `benchmarkPromote` now takes `runName` instead of `resultsDir`. Update the call:

```typescript
// Before:
await benchmarkPromote(resultsDir, process.cwd());

// After:
await benchmarkPromote(runName, process.cwd());
```

And update the variable name in the `promote` block:

```typescript
} else if (subCmd === "promote") {
  const { benchmarkPromote } = await import("./commands/benchmark-promote.js");
  const runName = rest[1];
  if (!runName) {
    console.error("Usage: sensei benchmark promote <run-name>");
    process.exit(1);
  }
  await benchmarkPromote(runName, process.cwd());
```

**Step 3: Update help text**

```typescript
  sensei benchmark inspect <run>-<a|b|c>
                                Switch to a benchmark strategy branch for inspection
  sensei benchmark promote <run-name>
                                Merge chosen strategy branch, submit telemetry
```

**Step 4: Run full test suite**

```bash
bun test
```

Expected: all tests pass

**Step 5: Commit**

```bash
git add packages/sensei/src/cli.ts
git commit -m "feat(sensei): wire benchmark inspect and update promote signature in cli"
```

---

## Final verification

```bash
bun test
```

Expected: all tests pass (≥68).

Check that these exports exist (used by tests):
- `benchmark-doctor.ts`: `buildTargetedIndexPrompt`, `buildRawContentPrompt`, `buildFullRepoIndexPrompt`, `parseOutputFolder`, `structuralScore`, `pickWinner`
- `benchmark-promote.ts`: `buildFeedback`, `submitReport`
- `benchmark-inspect.ts`: `resolveBranchName`
- `git.ts`: `getCurrentBranch`, `isCleanWorkingTree`, `branchExists`, `readFileFromBranch`
- `names.ts`: `generateRunName`
