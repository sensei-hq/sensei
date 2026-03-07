import { readFileSync, existsSync, readdirSync } from "fs";
import { readFile, mkdir, writeFile } from "fs/promises";
import { join, relative, extname, dirname } from "path";
import { intro, outro, spinner, note, log, confirm, isCancel } from "@clack/prompts";
import type { SymbolMap } from "../types.js";
import { callClaude } from "../claude.js";
import { getCurrentBranch, isCleanWorkingTree, createAndCheckoutBranch, checkoutBranch, stageFiles, commitFiles, branchExists } from "../git.js";
import { generateRunName } from "../names.js";

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
    // Match entries whose path starts with relInput or contains relInput as a segment
    const entries = Object.entries(map)
      .filter(([p]) => p.startsWith(relInput + "/") || p.includes(`/${relInput}/`))
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
    examplesSection = exFiles || "(no examples provided)";
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

// ── Winner selection ──────────────────────────────────────────────────────────

export function pickWinner(
  structA: number, judgeA: number,
  structB: number, judgeB: number,
  structC: number, judgeC: number,
): "a" | "b" | "c" {
  const scores = [
    { key: "a" as const, score: structA + judgeA },
    { key: "b" as const, score: structB + judgeB },
    { key: "c" as const, score: structC + judgeC },
  ];
  return scores.reduce((best, cur) => cur.score > best.score ? cur : best).key;
}

// ── Main command ──────────────────────────────────────────────────────────────

export interface BenchmarkDoctorOptions {
  template?: string;
  examples?: string;
  sample?: number;
}

export async function benchmarkDoctor(
  inputDir: string,
  outputName: string,
  repoPath: string,
  opts: BenchmarkDoctorOptions,
): Promise<void> {
  intro("sensei benchmark doctor");

  // ── Preconditions ────────────────────────────────────────────────────────────
  if (!isCleanWorkingTree(repoPath)) {
    log.error("Working tree is not clean. Please commit or stash your changes before running benchmark doctor.");
    outro("Aborted.");
    return;
  }

  const baseBranch = getCurrentBranch(repoPath);
  if (baseBranch === "HEAD") {
    log.error("Detached HEAD state detected. Please checkout a branch before running benchmark doctor.");
    outro("Aborted.");
    return;
  }

  // ── Read inputs ──────────────────────────────────────────────────────────────
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
  const templateContent = await readFile(templatePath, "utf-8");

  const allMd = readdirSync(fullInputDir).filter(f => extname(f) === ".md");
  const sample = opts.sample ? allMd.slice(0, opts.sample) : allMd;
  const original: Record<string, string> = {};
  for (const f of sample) {
    original[f] = await readFile(join(fullInputDir, f), "utf-8");
  }

  // ── Run all 3 strategies ─────────────────────────────────────────────────────
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

  // ── Generate run name and branch names ───────────────────────────────────────
  const runName = generateRunName();
  const branches = {
    a: `benchmark/${runName}-a`,
    b: `benchmark/${runName}-b`,
    c: `benchmark/${runName}-c`,
  } as const;

  const winner = pickWinner(structA, judge.scoreA, structB, judge.scoreB, structC, judge.scoreC);

  // ── Check no branches already exist ─────────────────────────────────────────
  for (const branch of Object.values(branches)) {
    if (branchExists(repoPath, branch)) {
      log.error(`Branch already exists: ${branch}. Re-run to get a new run name.`);
      outro("Aborted.");
      return;
    }
  }

  // ── Build results data ───────────────────────────────────────────────────────
  function cleanPaths(s: string): string {
    return s.replaceAll(repoPath + "/", "").replaceAll(repoPath, "");
  }

  const exFiles = examplesDir && existsSync(examplesDir)
    ? readdirSync(examplesDir).filter(f => extname(f) === ".md")
    : [];
  const inputLines = Object.values(original).join("\n").split("\n").length;
  const inputChars = Object.values(original).join("").length;

  const relInput = relative(repoPath, fullInputDir);
  const targetDir = join(repoPath, dirname(relInput), outputName);
  const senseiJsonPath = join(repoPath, ".sensei", `benchmark-${runName}.json`);

  const strategyNames = {
    a: "Targeted index",
    b: "Raw content",
    c: "Full repo index",
  } as const;

  const strategyOutputs = { a: outputA, b: outputB, c: outputC };
  const strategyResults = {
    a: { tokensIn: resultA.usage.tokensIn, tokensOut: resultA.usage.tokensOut, filesGenerated: Object.keys(outputA).length, structuralScore: structA, judgeScore: judge.scoreA },
    b: { tokensIn: resultB.usage.tokensIn, tokensOut: resultB.usage.tokensOut, filesGenerated: Object.keys(outputB).length, structuralScore: structB, judgeScore: judge.scoreB },
    c: { tokensIn: resultC.usage.tokensIn, tokensOut: resultC.usage.tokensOut, filesGenerated: Object.keys(outputC).length, structuralScore: structC, judgeScore: judge.scoreC },
  };

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
      scenario: {
        inputFileCount: Object.keys(original).length,
        inputTotalChars: inputChars,
        inputTotalLines: inputLines,
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
      results: strategyResults,
      judgeReasoning: judge.reasoning,
      userFeedback: null,
      promoted: null,
    },
  };

  // ── Permission prompt ────────────────────────────────────────────────────────
  const commitMsgA = `chore: sensei benchmark doctor using "${strategyNames.a}": docs/${outputName} (${Object.keys(outputA).length} files)`;
  const commitMsgB = `chore: sensei benchmark doctor using "${strategyNames.b}": docs/${outputName} (${Object.keys(outputB).length} files)`;
  const commitMsgC = `chore: sensei benchmark doctor using "${strategyNames.c}": docs/${outputName} (${Object.keys(outputC).length} files)`;

  const gitOpsLines = [
    `git checkout -b ${branches.a} ${baseBranch}`,
    `git add docs/${outputName}/ .sensei/benchmark-${runName}.json`,
    `git commit -m "${commitMsgA}"`,
    `git checkout ${baseBranch}`,
    `git checkout -b ${branches.b} ${baseBranch}`,
    `git add docs/${outputName}/ .sensei/benchmark-${runName}.json`,
    `git commit -m "${commitMsgB}"`,
    `git checkout ${baseBranch}`,
    `git checkout -b ${branches.c} ${baseBranch}`,
    `git add docs/${outputName}/ .sensei/benchmark-${runName}.json`,
    `git commit -m "${commitMsgC}"`,
    `git checkout ${baseBranch}`,
    `git checkout ${branches[winner]}  ← winner`,
  ];

  log.info(`sensei will perform these git operations:\n  ${gitOpsLines.join("\n  ")}`);
  const proceed = await confirm({ message: "Proceed?" });

  if (isCancel(proceed) || !proceed) {
    outro("Cancelled.");
    return;
  }

  // ── Create branches ──────────────────────────────────────────────────────────
  const strategies: Array<"a" | "b" | "c"> = ["a", "b", "c"];
  const commitMessages = { a: commitMsgA, b: commitMsgB, c: commitMsgC };

  for (const key of strategies) {
    const branch = branches[key];
    const output = strategyOutputs[key];
    const commitMsg = commitMessages[key];

    createAndCheckoutBranch(repoPath, branch, baseBranch);

    await mkdir(targetDir, { recursive: true });
    for (const [name, content] of Object.entries(output)) {
      await writeFile(join(targetDir, name), content, "utf-8");
    }

    await mkdir(join(repoPath, ".sensei"), { recursive: true });
    await writeFile(senseiJsonPath, JSON.stringify(resultsData, null, 2), "utf-8");

    stageFiles(repoPath, [targetDir, senseiJsonPath]);
    commitFiles(repoPath, commitMsg);

    checkoutBranch(repoPath, baseBranch);
  }

  // ── Checkout winner branch ───────────────────────────────────────────────────
  checkoutBranch(repoPath, branches[winner]);

  // ── Announce results ─────────────────────────────────────────────────────────
  note(
    `Run: ${runName}\n` +
    `A (${strategyNames.a}): struct=${structA} judge=${judge.scoreA}\n` +
    `B (${strategyNames.b}): struct=${structB} judge=${judge.scoreB}\n` +
    `C (${strategyNames.c}): struct=${structC} judge=${judge.scoreC}\n` +
    `Winner: ${winner.toUpperCase()} → ${branches[winner]}\n` +
    `\n` +
    `To inspect: sensei benchmark inspect ${runName}-<a|b|c>\n` +
    `To promote: sensei benchmark promote ${runName}`
  );
  outro("Done.");
}
