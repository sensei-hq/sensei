# Doctor Benchmark Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add `sensei benchmark doctor <input-dir> <output-name>` command that runs 3 strategies to convert a docs folder into a new format, scores quality (structural + LLM judge) and token usage, and writes `results.json` + `summary.md`.

**Architecture:** Extract `callClaude()` into a shared `src/claude.ts` module. Add `src/commands/benchmark-doctor.ts` with three strategy functions (targeted index, raw content, full repo index), a structural scorer, and an LLM judge. Wire into `cli.ts` under `sensei benchmark doctor`. Results written to `results/benchmark-doctor-<date>/`.

**Tech Stack:** TypeScript, `@anthropic-ai/sdk` (streaming + finalMessage), `@clack/prompts` (spinner/note/outro), `fs/promises`, existing `SymbolMap` type from `src/types.ts`.

---

### Task 1: Extract `callClaude` into shared module

**Files:**
- Create: `packages/sensei/src/claude.ts`
- Modify: `packages/sensei/src/commands/doctor.ts`

**Step 1: Create `src/claude.ts`**

```typescript
// packages/sensei/src/claude.ts
import Anthropic from "@anthropic-ai/sdk";

const MODEL = "claude-opus-4-6";

export interface ClaudeUsage {
  tokensIn: number;
  tokensOut: number;
}

export interface ClaudeResult {
  text: string;
  usage: ClaudeUsage;
}

export async function callClaude(prompt: string): Promise<ClaudeResult> {
  const client = new Anthropic();
  const stream = client.messages.stream({
    model: MODEL,
    max_tokens: 16384,
    thinking: { type: "adaptive" },
    messages: [{ role: "user", content: prompt }],
  });
  const message = await stream.finalMessage();
  const text = message.content.find(b => b.type === "text");
  return {
    text: text?.text ?? "",
    usage: {
      tokensIn: message.usage.input_tokens,
      tokensOut: message.usage.output_tokens,
    },
  };
}
```

**Step 2: Update `doctor.ts` to use shared module**

Replace the local `callClaude` function and `MODEL` constant in `packages/sensei/src/commands/doctor.ts`:

```typescript
// Remove lines 5-20 (Anthropic import + MODEL + local callClaude)
// Replace with:
import { callClaude } from "../claude.js";
```

Update the call site at line 90:
```typescript
// Before:
const reformatted = await callClaude(prompt);

// After:
const { text: reformatted } = await callClaude(prompt);
```

**Step 3: Build and verify**

```bash
cd /Users/Jerry/Developer/skills
bun run build 2>&1 | grep -E "error|Bundled"
```
Expected: `Bundled 352 modules` (or similar), no errors.

**Step 4: Run tests**

```bash
bunx vitest run 2>&1 | tail -5
```
Expected: `50 passed`

**Step 5: Commit**

```bash
git add packages/sensei/src/claude.ts packages/sensei/src/commands/doctor.ts
git commit -m "refactor: extract callClaude to shared src/claude.ts"
```

---

### Task 2: Write failing tests for benchmark-doctor

**Files:**
- Create: `packages/sensei/src/commands/benchmark-doctor.spec.ts`

The benchmark command has a lot of Claude API calls, so tests mock `callClaude` and focus on: prompt construction, output file parsing, structural scoring logic.

**Step 1: Create the test file**

```typescript
// packages/sensei/src/commands/benchmark-doctor.spec.ts
import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { mkdirSync, writeFileSync, rmSync } from "fs";
import { join } from "path";

const TMP = "/tmp/sensei-benchmark-test";

// Mock callClaude to avoid real API calls
vi.mock("../claude.js", () => ({
  callClaude: vi.fn().mockResolvedValue({
    text: `## File: 01-core.md\n# Core\n## Features\n### Feature A\nTODO\n## Status\n| Feature | Status |\n|---|---|\n| A | 🔲 Planned |`,
    usage: { tokensIn: 100, tokensOut: 200 },
  }),
}));

import {
  buildTargetedIndexPrompt,
  buildRawContentPrompt,
  buildFullRepoIndexPrompt,
  parseOutputFolder,
  structuralScore,
} from "./benchmark-doctor.js";

beforeEach(() => {
  mkdirSync(join(TMP, "requirements"), { recursive: true });
  mkdirSync(join(TMP, "examples"), { recursive: true });
  mkdirSync(join(TMP, ".index"), { recursive: true });
  writeFileSync(join(TMP, "requirements/01-core.md"), "# Core\nThe core module handles auth.\n");
  writeFileSync(join(TMP, "examples/01-example.md"), "# Example\n## Features\n### Login\nTODO\n## Status\n| Feature | Status |\n|---|---|\n");
  writeFileSync(join(TMP, ".index/symbol-map.json"), JSON.stringify({
    "docs/requirements/01-core.md": { L0: ["# Core"], L1: ["handles auth"], L2: [] },
  }));
});

afterEach(() => rmSync(TMP, { recursive: true, force: true }));

describe("buildTargetedIndexPrompt", () => {
  it("includes only input-dir entries from symbol-map", () => {
    const prompt = buildTargetedIndexPrompt({
      inputDir: join(TMP, "requirements"),
      repoPath: TMP,
      templateContent: "# Template",
      outputName: "features",
    });
    expect(prompt).toContain("01-core.md");
    expect(prompt).toContain("# Template");
    expect(prompt).not.toContain("src/");
  });
});

describe("buildRawContentPrompt", () => {
  it("includes full file content from input dir and examples", () => {
    const prompt = buildRawContentPrompt({
      inputDir: join(TMP, "requirements"),
      examplesDir: join(TMP, "examples"),
      outputName: "features",
    });
    expect(prompt).toContain("handles auth");
    expect(prompt).toContain("01-example.md");
  });
});

describe("buildFullRepoIndexPrompt", () => {
  it("includes entire symbol-map and template", () => {
    const prompt = buildFullRepoIndexPrompt({
      repoPath: TMP,
      templateContent: "# Template",
      outputName: "features",
    });
    expect(prompt).toContain("symbol-map");
    expect(prompt).toContain("# Template");
  });
});

describe("parseOutputFolder", () => {
  it("splits Claude response on ## File: markers", () => {
    const raw = `## File: README.md\n# Features\nOverview\n\n## File: 01-core.md\n# Core\n## Features\n`;
    const files = parseOutputFolder(raw);
    expect(files["README.md"]).toContain("# Features");
    expect(files["01-core.md"]).toContain("## Features");
  });

  it("handles response without file markers as README", () => {
    const raw = `# Features\nSome content`;
    const files = parseOutputFolder(raw);
    expect(files["README.md"]).toContain("# Features");
  });
});

describe("structuralScore", () => {
  it("scores 10 for perfect output matching template sections", () => {
    const template = "## Features\n## Status\n";
    const output = {
      "01.md": "## Features\n### A\nTODO\n## Status\n| F | S |\n|---|---|\n| A | 🔲 Planned |",
      "README.md": "# Overview",
    };
    const original = { "01-core.md": "# Core\nauth module" };
    const score = structuralScore({ template, output, original });
    expect(score).toBeGreaterThanOrEqual(8);
  });

  it("penalises missing README", () => {
    const template = "## Features\n## Status\n";
    const output = { "01.md": "## Features\n## Status\n" };
    const original = { "01-core.md": "auth" };
    const score = structuralScore({ template, output, original });
    expect(score).toBeLessThanOrEqual(7);
  });

  it("penalises TODO inflation", () => {
    const template = "## Features\n## Status\n";
    const output = { "01.md": "TODO TODO TODO TODO TODO TODO TODO\n## Features\n## Status\n", "README.md": "# R" };
    const original = { "01-core.md": "auth" }; // 0 TODOs originally
    const score = structuralScore({ template, output, original });
    expect(score).toBeLessThan(10);
  });
});
```

**Step 2: Run tests — verify they fail**

```bash
bunx vitest run packages/sensei/src/commands/benchmark-doctor.spec.ts 2>&1 | tail -10
```
Expected: FAIL — `Cannot find module './benchmark-doctor.js'`

---

### Task 3: Implement `benchmark-doctor.ts` — prompt builders + parsers

**Files:**
- Create: `packages/sensei/src/commands/benchmark-doctor.ts`

**Step 1: Create the file with exported pure functions**

```typescript
// packages/sensei/src/commands/benchmark-doctor.ts
import { readFileSync, existsSync, readdirSync } from "fs";
import { readFile, mkdir, writeFile } from "fs/promises";
import { join, relative, extname, basename } from "path";
import { intro, outro, spinner, note, log } from "@clack/prompts";
import type { SymbolMap } from "../types.js";
import { callClaude, type ClaudeResult } from "../claude.js";

// ── Prompt builders ───────────────────────────────────────────────────────────

export interface TargetedIndexPromptOptions {
  inputDir: string;
  repoPath: string;
  templateContent: string;
  outputName: string;
}

export function buildTargetedIndexPrompt(opts: TargetedIndexPromptOptions): string {
  const symbolMapPath = join(opts.repoPath, ".index/symbol-map.json");
  let indexSection = "(no index found)";
  if (existsSync(symbolMapPath)) {
    const map: SymbolMap = JSON.parse(readFileSync(symbolMapPath, "utf-8"));
    const relInput = relative(opts.repoPath, opts.inputDir);
    const entries = Object.entries(map)
      .filter(([path]) => path.startsWith(relInput))
      .map(([path, entry]) => `### ${path}\n${[...entry.L0, ...entry.L1].join("\n")}`)
      .join("\n\n");
    indexSection = entries || "(no entries for input dir)";
  }

  return `You are converting a requirements folder into a ${opts.outputName} folder.

## Template (each output file must follow this structure)

${opts.templateContent}

## Index of input documents (L0/L1 summaries)

${indexSection}

## Instructions

Generate a complete \`${opts.outputName}/\` folder from the requirements above.
Output each file preceded by a marker on its own line: \`## File: <filename>\`
Include a \`README.md\` that summarises all features.
Follow the template structure exactly. Use "TODO: [description]" for missing information.
Output files only — no preamble or explanation.`;
}

export interface RawContentPromptOptions {
  inputDir: string;
  examplesDir: string | null;
  outputName: string;
}

export function buildRawContentPrompt(opts: RawContentPromptOptions): string {
  const inputFiles = readdirSync(opts.inputDir)
    .filter(f => extname(f) === ".md")
    .map(f => {
      const content = readFileSync(join(opts.inputDir, f), "utf-8");
      return `### ${f}\n${content}`;
    })
    .join("\n\n");

  let examplesSection = "(no examples provided)";
  if (opts.examplesDir && existsSync(opts.examplesDir)) {
    const exFiles = readdirSync(opts.examplesDir)
      .filter(f => extname(f) === ".md")
      .map(f => {
        const content = readFileSync(join(opts.examplesDir!, f), "utf-8");
        return `### ${f}\n${content}`;
      })
      .join("\n\n");
    examplesSection = exFiles;
  }

  return `You are converting a requirements folder into a ${opts.outputName} folder.

## Example output documents (match this style and structure)

${examplesSection}

## Input documents (requirements to convert)

${inputFiles}

## Instructions

Generate a complete \`${opts.outputName}/\` folder from the input documents.
Output each file preceded by a marker on its own line: \`## File: <filename>\`
Include a \`README.md\` that summarises all features.
Match the style and structure of the example documents exactly.
Output files only — no preamble or explanation.`;
}

export interface FullRepoIndexPromptOptions {
  repoPath: string;
  templateContent: string;
  outputName: string;
}

export function buildFullRepoIndexPrompt(opts: FullRepoIndexPromptOptions): string {
  const symbolMapPath = join(opts.repoPath, ".index/symbol-map.json");
  let symbolMapSection = "(no symbol-map found)";
  if (existsSync(symbolMapPath)) {
    symbolMapSection = readFileSync(symbolMapPath, "utf-8");
  }

  return `You are converting requirements into a ${opts.outputName} folder.
You have access to the full repository index (symbol-map) for context.

## Template (each output file must follow this structure)

${opts.templateContent}

## Full repository index (symbol-map.json)

${symbolMapSection}

## Instructions

Generate a complete \`${opts.outputName}/\` folder.
Output each file preceded by a marker on its own line: \`## File: <filename>\`
Include a \`README.md\` that summarises all features.
Follow the template structure exactly. Use "TODO: [description]" for missing information.
Output files only — no preamble or explanation.`;
}

// ── Output parser ─────────────────────────────────────────────────────────────

export function parseOutputFolder(raw: string): Record<string, string> {
  const files: Record<string, string> = {};
  const parts = raw.split(/^## File:\s*/m);

  if (parts.length === 1) {
    // No markers — treat entire response as README
    files["README.md"] = raw.trim();
    return files;
  }

  for (const part of parts.slice(1)) {
    const newline = part.indexOf("\n");
    if (newline === -1) continue;
    const name = part.slice(0, newline).trim();
    const content = part.slice(newline + 1).trim();
    if (name) files[name] = content;
  }
  return files;
}

// ── Structural scorer ─────────────────────────────────────────────────────────

export interface StructuralScoreOptions {
  template: string;
  output: Record<string, string>;
  original: Record<string, string>;
}

export function structuralScore(opts: StructuralScoreOptions): number {
  let score = 0;

  // +3: README present
  if (opts.output["README.md"]) score += 3;

  // +3: expected section headers from template present across output files
  const templateHeaders = [...opts.template.matchAll(/^##+ .+/gm)].map(m => m[0].replace(/^#+\s*/, ""));
  const allOutputText = Object.values(opts.output).join("\n");
  const foundHeaders = templateHeaders.filter(h => allOutputText.includes(h));
  if (templateHeaders.length > 0) {
    score += Math.round(3 * (foundHeaders.length / templateHeaders.length));
  }

  // +2: content coverage — key terms from original appear in output
  const originalText = Object.values(opts.original).join(" ");
  const terms = originalText.match(/\b[A-Za-z]{5,}\b/g) ?? [];
  const uniqueTerms = [...new Set(terms)].slice(0, 30);
  const covered = uniqueTerms.filter(t => allOutputText.toLowerCase().includes(t.toLowerCase()));
  if (uniqueTerms.length > 0) {
    const ratio = covered.length / uniqueTerms.length;
    score += ratio >= 0.8 ? 2 : ratio >= 0.5 ? 1 : 0;
  }

  // +2: no TODO inflation
  const originalTodos = (originalText.match(/TODO/gi) ?? []).length;
  const outputTodos = (allOutputText.match(/TODO/gi) ?? []).length;
  const maxAllowed = originalTodos + Object.keys(opts.output).length * 2;
  if (outputTodos <= maxAllowed) score += 2;

  return Math.min(score, 10);
}

// ── LLM judge ─────────────────────────────────────────────────────────────────

export interface JudgeResult {
  scoreA: number;
  scoreB: number;
  scoreC: number;
  reasoning: string;
}

export async function llmJudge(
  original: Record<string, string>,
  a: Record<string, string>,
  b: Record<string, string>,
  c: Record<string, string>,
): Promise<JudgeResult> {
  const fmt = (files: Record<string, string>) =>
    Object.entries(files).map(([name, content]) => `### ${name}\n${content}`).join("\n\n");

  const prompt = `You are evaluating three attempts to convert requirements into feature docs.

## Original requirements
${fmt(original)}

## Strategy A output (targeted index — lowest context)
${fmt(a)}

## Strategy B output (raw content — highest context)
${fmt(b)}

## Strategy C output (full repo index)
${fmt(c)}

## Scoring rubric (0–10 each)
- Structure conformance: does it follow a consistent template with clear sections?
- Content completeness: is all important information from requirements preserved?
- No invented content: does it avoid adding facts not present in the original?

Score each strategy 0–10 holistically across all three criteria.

Respond ONLY with valid JSON (no markdown, no explanation):
{"scoreA": <0-10>, "scoreB": <0-10>, "scoreC": <0-10>, "reasoning": "<1-2 sentences>"}`;

  const { text } = await callClaude(prompt);
  try {
    const cleaned = text.replace(/```json\n?|\n?```/g, "").trim();
    return JSON.parse(cleaned) as JudgeResult;
  } catch {
    return { scoreA: 0, scoreB: 0, scoreC: 0, reasoning: `Parse error: ${text.slice(0, 200)}` };
  }
}

// ── Main command ──────────────────────────────────────────────────────────────

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

  const examplesDir = opts.examples ? join(repoPath, opts.examples) : null;
  const outDir = join(repoPath, opts.out ?? "results", `benchmark-doctor-${new Date().toISOString().slice(0, 10)}`);

  const templateContent = await readFile(templatePath, "utf-8");

  // Load original docs
  const allMd = readdirSync(fullInputDir).filter(f => extname(f) === ".md");
  const sample = opts.sample ? allMd.slice(0, opts.sample) : allMd;
  const original: Record<string, string> = {};
  for (const f of sample) {
    original[f] = await readFile(join(fullInputDir, f), "utf-8");
  }

  // ── Run strategies ──────────────────────────────────────────────────────────
  const sp = spinner();

  sp.start("Strategy A: targeted index...");
  const promptA = buildTargetedIndexPrompt({ inputDir: fullInputDir, repoPath, templateContent, outputName });
  const resultA = await callClaude(promptA);
  const outputA = parseOutputFolder(resultA.text);
  sp.stop(`Strategy A done (${resultA.usage.tokensIn}→${resultA.usage.tokensOut} tokens)`);

  sp.start("Strategy B: raw content...");
  const promptB = buildRawContentPrompt({ inputDir: fullInputDir, examplesDir, outputName });
  const resultB = await callClaude(promptB);
  const outputB = parseOutputFolder(resultB.text);
  sp.stop(`Strategy B done (${resultB.usage.tokensIn}→${resultB.usage.tokensOut} tokens)`);

  sp.start("Strategy C: full repo index...");
  const promptC = buildFullRepoIndexPrompt({ repoPath, templateContent, outputName });
  const resultC = await callClaude(promptC);
  const outputC = parseOutputFolder(resultC.text);
  sp.stop(`Strategy C done (${resultC.usage.tokensIn}→${resultC.usage.tokensOut} tokens)`);

  // ── Score ───────────────────────────────────────────────────────────────────
  sp.start("Scoring...");
  const structA = structuralScore({ template: templateContent, output: outputA, original });
  const structB = structuralScore({ template: templateContent, output: outputB, original });
  const structC = structuralScore({ template: templateContent, output: outputC, original });

  const judge = await llmJudge(original, outputA, outputB, outputC);
  sp.stop("Scoring done");

  // ── Write outputs ───────────────────────────────────────────────────────────
  await mkdir(join(outDir, "a", outputName), { recursive: true });
  await mkdir(join(outDir, "b", outputName), { recursive: true });
  await mkdir(join(outDir, "c", outputName), { recursive: true });

  for (const [name, content] of Object.entries(outputA))
    await writeFile(join(outDir, "a", outputName, name), content, "utf-8");
  for (const [name, content] of Object.entries(outputB))
    await writeFile(join(outDir, "b", outputName, name), content, "utf-8");
  for (const [name, content] of Object.entries(outputC))
    await writeFile(join(outDir, "c", outputName, name), content, "utf-8");

  const results = {
    date: new Date().toISOString().slice(0, 10),
    input: inputDir,
    outputName,
    strategies: {
      a: { name: "Targeted index", description: "L1 summaries of input dir only + template", templatePath },
      b: { name: "Raw content", description: "Full input content + example output folder", examplesPath: examplesDir ?? "none" },
      c: { name: "Full repo index", description: "Full symbol-map.json + template", indexPath: join(repoPath, ".index/symbol-map.json") },
    },
    scores: {
      a: { tokensIn: resultA.usage.tokensIn, tokensOut: resultA.usage.tokensOut, filesGenerated: Object.keys(outputA).length, structuralScore: structA, judgeScore: judge.scoreA },
      b: { tokensIn: resultB.usage.tokensIn, tokensOut: resultB.usage.tokensOut, filesGenerated: Object.keys(outputB).length, structuralScore: structB, judgeScore: judge.scoreB },
      c: { tokensIn: resultC.usage.tokensIn, tokensOut: resultC.usage.tokensOut, filesGenerated: Object.keys(outputC).length, structuralScore: structC, judgeScore: judge.scoreC },
    },
    judgeReasoning: judge.reasoning,
  };

  await writeFile(join(outDir, "results.json"), JSON.stringify(results, null, 2), "utf-8");

  const summary = `# Doctor Benchmark — ${results.date}

Input: \`${inputDir}\` → \`${outputName}\`

| Strategy | Approach | Structural | Judge | Tokens In | Tokens Out | Files |
|---|---|---|---|---|---|---|
| A | Targeted index (lowest context) | ${structA} | ${judge.scoreA} | ${resultA.usage.tokensIn.toLocaleString()} | ${resultA.usage.tokensOut.toLocaleString()} | ${Object.keys(outputA).length} |
| B | Raw content (highest context) | ${structB} | ${judge.scoreB} | ${resultB.usage.tokensIn.toLocaleString()} | ${resultB.usage.tokensOut.toLocaleString()} | ${Object.keys(outputB).length} |
| C | Full repo index | ${structC} | ${judge.scoreC} | ${resultC.usage.tokensIn.toLocaleString()} | ${resultC.usage.tokensOut.toLocaleString()} | ${Object.keys(outputC).length} |

## Judge Reasoning

${judge.reasoning}
`;

  await writeFile(join(outDir, "summary.md"), summary, "utf-8");

  note(
    `Results: ${relative(repoPath, outDir)}/\n` +
    `A: struct=${structA} judge=${judge.scoreA} tokens=${resultA.usage.tokensIn}→${resultA.usage.tokensOut}\n` +
    `B: struct=${structB} judge=${judge.scoreB} tokens=${resultB.usage.tokensIn}→${resultB.usage.tokensOut}\n` +
    `C: struct=${structC} judge=${judge.scoreC} tokens=${resultC.usage.tokensIn}→${resultC.usage.tokensOut}`,
    "Benchmark complete"
  );
  outro("Done.");
}
```

**Step 2: Run tests — verify they pass**

```bash
bunx vitest run packages/sensei/src/commands/benchmark-doctor.spec.ts 2>&1 | tail -10
```
Expected: all tests pass.

**Step 3: Run full suite**

```bash
bunx vitest run 2>&1 | tail -5
```
Expected: `50+ passed`

**Step 4: Commit**

```bash
git add packages/sensei/src/commands/benchmark-doctor.ts packages/sensei/src/commands/benchmark-doctor.spec.ts
git commit -m "feat: add benchmark-doctor command with 3-strategy comparison"
```

---

### Task 4: Wire benchmark into CLI

**Files:**
- Modify: `packages/sensei/src/cli.ts`

**Step 1: Add CLI flags**

In `cli.ts`, add to `parseArgs` options:
```typescript
options: {
  // existing...
  template: { type: "string" },
  examples: { type: "string" },
  sample: { type: "string" },    // parseArgs doesn't support number type; parse manually
  out: { type: "string" },
}
```

**Step 2: Add `benchmark` case**

Add before the `default` case:

```typescript
case "benchmark": {
  const subCmd = rest[0];
  if (subCmd === "doctor") {
    const { benchmarkDoctor } = await import("./commands/benchmark-doctor.js");
    const inputDir = rest[1];
    const outputName = rest[2];
    if (!inputDir || !outputName) {
      console.error("Usage: sensei benchmark doctor <input-dir> <output-name>");
      process.exit(1);
    }
    await benchmarkDoctor(inputDir, outputName, process.cwd(), {
      template: values.template,
      examples: values.examples,
      sample: values.sample ? parseInt(values.sample, 10) : undefined,
      out: values.out,
    });
  } else {
    console.error(`Unknown benchmark subcommand: ${subCmd}`);
    process.exit(1);
  }
  break;
}
```

**Step 3: Update help text**

In the `default` case log, add:
```
  sensei benchmark doctor <input-dir> <output-name> [--template <path>] [--examples <dir>] [--sample N] [--out <dir>]
                                Run 3-strategy doc conversion benchmark
```

**Step 4: Build**

```bash
bun run build 2>&1 | grep -E "error|Bundled"
```
Expected: no errors.

**Step 5: Smoke test (dry run)**

```bash
cd /Users/Jerry/Developer/strategos
sensei benchmark doctor docs/requirements features --sample 2 --examples docs/features/ 2>&1 | head -5
```
Expected: spinner starts for Strategy A (will call real Claude API — Ctrl+C if you don't want to spend tokens, just verify it starts).

**Step 6: Run full tests**

```bash
cd /Users/Jerry/Developer/skills
bunx vitest run 2>&1 | tail -5
```
Expected: all pass.

**Step 7: Commit**

```bash
git add packages/sensei/src/cli.ts
git commit -m "feat: wire sensei benchmark doctor into CLI"
```

---

### Task 5: Add `results/` to gitignore, commit summary.md

**Files:**
- Modify: `.gitignore` (if exists at repo root, else create)
- Modify: `README.md`

**Step 1: Check/update .gitignore**

```bash
cd /Users/Jerry/Developer/skills
cat .gitignore 2>/dev/null || echo "(no .gitignore)"
```

If `.gitignore` exists, append:
```
# Benchmark raw results (summaries are committed manually)
results/**/*.json
results/**/a/
results/**/b/
results/**/c/
```

If it doesn't exist, create it with those lines.

**Step 2: Update README CLI table**

In `README.md`, add to the CLI section:
```
sensei benchmark doctor <input> <output> [--template] [--examples] [--sample N]
                             3-strategy doc conversion benchmark
```

**Step 3: Build and test**

```bash
bun run build 2>&1 | grep -E "error|Bundled"
bunx vitest run 2>&1 | tail -5
```

**Step 4: Commit**

```bash
git add .gitignore README.md
git commit -m "chore: gitignore benchmark raw results, document benchmark doctor CLI"
```

---

### Task 6: Run real benchmark on strategos

This task is manual verification — no code changes.

**Step 1: Ensure strategos is indexed**

```bash
cd /Users/Jerry/Developer/strategos
sensei status 2>&1 | head -10
```
Expected: shows index age and file count. If missing, run `sensei init` first.

**Step 2: Run benchmark**

```bash
sensei benchmark doctor docs/requirements features \
  --examples docs/features/ \
  --sample 3 \
  --out results/
```

Wait for all three strategies + scoring (~2-3 minutes).

**Step 3: Inspect results**

```bash
cat results/benchmark-doctor-*/summary.md
ls results/benchmark-doctor-*/a/features/
ls results/benchmark-doctor-*/b/features/
ls results/benchmark-doctor-*/c/features/
```

**Step 4: Commit summary to strategos repo**

```bash
git add results/benchmark-doctor-*/summary.md
git commit -m "chore: doctor benchmark results"
```
