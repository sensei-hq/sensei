/**
 * Score a generated llmspec.yaml against the gold standard.
 *
 * Usage:
 *   bun tasks/score-coverage.ts <generated.yaml>
 *   bun tasks/score-coverage.ts .sensei/llmspec.yaml
 *
 * Outputs a score from 0-100 and a diff of missing/extra entries.
 */

import yaml from "js-yaml";
import { readFileSync } from "fs";
import { resolve } from "path";

const ROOT = new URL("..", import.meta.url).pathname;

interface DocEntry {
  path: string;
  covers: string[];
}

interface LlmSpec {
  project?: string;
  description?: string;
  entry_points?: Array<{ path: string; role?: string }>;
  concepts?: Array<{ name: string }>;
  patterns?: Array<{ name: string }>;
  docs?: DocEntry[];
}

function load(rel: string): LlmSpec {
  return yaml.load(readFileSync(resolve(ROOT, rel), "utf8")) as LlmSpec;
}

const expected = load(".sensei/llmspec-expected.yaml");
const generatedPath = process.argv[2] ?? ".sensei/llmspec.yaml";
const generated = load(generatedPath);

let score = 0;
const max = 100;
const issues: string[] = [];

// --- description (10pts) ---
const hasDesc = generated.description && !generated.description.includes("TODO");
if (hasDesc) score += 10;
else issues.push("MISSING description (or still TODO)");

// --- entry_points (15pts) ---
const expEP = new Set(expected.entry_points?.map(e => e.path) ?? []);
const genEP = new Set(generated.entry_points?.map(e => e.path) ?? []);
const epHits = [...expEP].filter(p => genEP.has(p)).length;
const epScore = Math.round((epHits / expEP.size) * 15);
score += epScore;
const missingEP = [...expEP].filter(p => !genEP.has(p));
if (missingEP.length) issues.push(`entry_points missing: ${missingEP.join(", ")}`);

// --- concepts (10pts) ---
const expConcepts = new Set(expected.concepts?.map(c => c.name.toLowerCase()) ?? []);
const genConcepts = new Set(generated.concepts?.map(c => c.name.toLowerCase()) ?? []);
const conceptHits = [...expConcepts].filter(c => genConcepts.has(c)).length;
score += Math.round((conceptHits / Math.max(expConcepts.size, 1)) * 10);
const missingConcepts = [...expConcepts].filter(c => !genConcepts.has(c));
if (missingConcepts.length) issues.push(`concepts missing: ${missingConcepts.join(", ")}`);

// --- patterns (10pts) ---
const expPatterns = new Set(expected.patterns?.map(p => p.name.toLowerCase()) ?? []);
const genPatterns = new Set(generated.patterns?.map(p => p.name.toLowerCase()) ?? []);
const patternHits = [...expPatterns].filter(p => genPatterns.has(p)).length;
score += Math.round((patternHits / Math.max(expPatterns.size, 1)) * 10);
const missingPatterns = [...expPatterns].filter(p => !genPatterns.has(p));
if (missingPatterns.length) issues.push(`patterns missing: ${missingPatterns.join(", ")}`);

// --- docs coverage (55pts) ---
const expDocs = new Map<string, string[]>(
  expected.docs?.map(d => [d.path, d.covers ?? []]) ?? []
);
const genDocs = new Map<string, string[]>(
  generated.docs?.map(d => [d.path, d.covers ?? []]) ?? []
);

let coverageHits = 0;
let coverageTotal = 0;

for (const [docPath, expCovers] of expDocs) {
  const genCovers = new Set(genDocs.get(docPath) ?? []);
  for (const cover of expCovers) {
    coverageTotal++;
    if (genCovers.has(cover)) coverageHits++;
    else issues.push(`  ${docPath} missing cover: ${cover}`);
  }
  if (!genDocs.has(docPath)) {
    issues.push(`docs[] missing entry: ${docPath}`);
  }
}

// penalise hallucinated paths
let hallucinated = 0;
for (const [docPath, genCovers] of genDocs) {
  const { existsSync } = await import("fs");
  for (const cover of genCovers) {
    if (!existsSync(resolve(ROOT, cover))) {
      hallucinated++;
      issues.push(`  HALLUCINATED path in ${docPath}: ${cover}`);
    }
  }
}

const coverageScore = Math.round((coverageHits / Math.max(coverageTotal, 1)) * 55);
const hallucinationPenalty = Math.min(hallucinated * 3, 20);
score += coverageScore - hallucinationPenalty;
score = Math.max(0, Math.min(score, max));

// --- output ---
console.log(`\nllmspec coverage score: ${score}/${max}`);
console.log(`  description:   ${hasDesc ? "✓" : "✗"} (10pts)`);
console.log(`  entry_points:  ${epHits}/${expEP.size} (${epScore}/15pts)`);
console.log(`  concepts:      ${conceptHits}/${expConcepts.size} (${Math.round((conceptHits / Math.max(expConcepts.size, 1)) * 10)}/10pts)`);
console.log(`  patterns:      ${patternHits}/${expPatterns.size} (${Math.round((patternHits / Math.max(expPatterns.size, 1)) * 10)}/10pts)`);
console.log(`  doc coverage:  ${coverageHits}/${coverageTotal} entries (${coverageScore}/55pts)`);
if (hallucinated) console.log(`  hallucinations: -${hallucinationPenalty}pts (${hallucinated} bad paths)`);

if (issues.length) {
  console.log("\nIssues:");
  issues.slice(0, 20).forEach(i => console.log(" ", i));
  if (issues.length > 20) console.log(`  ... and ${issues.length - 20} more`);
}
