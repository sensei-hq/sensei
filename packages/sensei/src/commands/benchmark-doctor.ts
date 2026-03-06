import { readFileSync, existsSync, readdirSync } from "fs";
import { readFile, mkdir, writeFile } from "fs/promises";
import { join, relative, extname } from "path";
import { intro, outro, spinner, note, log } from "@clack/prompts";
import type { SymbolMap } from "../types.js";
import { callClaude } from "../claude.js";

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
      .filter(([path]) => path.startsWith(relInput) || path.includes(`/${relInput}/`) || path.includes(`/${relInput}`))
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

  const allMd = readdirSync(fullInputDir).filter(f => extname(f) === ".md");
  const sample = opts.sample ? allMd.slice(0, opts.sample) : allMd;
  const original: Record<string, string> = {};
  for (const f of sample) {
    original[f] = await readFile(join(fullInputDir, f), "utf-8");
  }

  const sp = spinner();

  sp.start("Strategy A: targeted index...");
  const resultA = await callClaude(buildTargetedIndexPrompt({ inputDir: fullInputDir, repoPath, templateContent, outputName }));
  const outputA = parseOutputFolder(resultA.text);
  sp.stop(`Strategy A done (${resultA.usage.tokensIn}→${resultA.usage.tokensOut} tokens)`);

  sp.start("Strategy B: raw content...");
  const resultB = await callClaude(buildRawContentPrompt({ inputDir: fullInputDir, examplesDir, outputName }));
  const outputB = parseOutputFolder(resultB.text);
  sp.stop(`Strategy B done (${resultB.usage.tokensIn}→${resultB.usage.tokensOut} tokens)`);

  sp.start("Strategy C: full repo index...");
  const resultC = await callClaude(buildFullRepoIndexPrompt({ repoPath, templateContent, outputName }));
  const outputC = parseOutputFolder(resultC.text);
  sp.stop(`Strategy C done (${resultC.usage.tokensIn}→${resultC.usage.tokensOut} tokens)`);

  sp.start("Scoring...");
  const structA = structuralScore({ template: templateContent, output: outputA, original });
  const structB = structuralScore({ template: templateContent, output: outputB, original });
  const structC = structuralScore({ template: templateContent, output: outputC, original });
  const judge = await llmJudge(original, outputA, outputB, outputC);
  sp.stop("Scoring done");

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
