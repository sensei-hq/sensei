/**
 * sensei benchmark indexer [--corpus <name>] [--all]
 *
 * Indexes a corpus and compares results against the gold standard
 * defined in sensei-benchmark.yaml.
 */

import { readFile, readdir } from "fs/promises";
import { existsSync } from "fs";
import { join, basename, resolve } from "path";
import yaml from "js-yaml";
import { indexRepo, getOrCreateDb } from "@sensei/graph-indexer";
import { lookupRepoId } from "@sensei/shared";

interface BenchmarkSpec {
  name: string;
  description: string;
  language: string;
  category: string;
  difficulty: string;
  source: string;
  tasks?: string;
  skills?: string;
  expected: {
    symbols: {
      total: number;
      functions?: number;
      classes?: number;
      types?: number;
      enums?: number;
      const?: number;
    };
    edges: {
      calls?: number;
      imports?: number;
    };
    communities?: {
      count: number;
      god_nodes?: string[];
    };
    docs?: {
      total: number;
    };
  };
  expected_insights?: string[];
  surprising_connections?: Array<{ from: string; to: string; why: string }>;
}

interface QualityResult {
  metric: string;
  expected: number;
  found: number;
  precision: number;
  recall: number;
}

function findBenchmarkYaml(path: string): string | null {
  const candidates = [
    join(path, "sensei-benchmark.yaml"),
    join(path, "sensei-benchmark.yml"),
  ];
  for (const c of candidates) {
    if (existsSync(c)) return c;
  }
  return null;
}

async function loadSpec(yamlPath: string): Promise<BenchmarkSpec> {
  const raw = await readFile(yamlPath, "utf-8");
  return yaml.load(raw) as BenchmarkSpec;
}

function pct(n: number): string {
  return `${(n * 100).toFixed(1)}%`;
}

export async function benchmarkIndexer(
  repoPath: string,
  opts: { corpus?: string } = {},
): Promise<void> {
  // Find benchmark spec
  const targetPath = opts.corpus ? resolve(opts.corpus) : repoPath;
  const yamlPath = findBenchmarkYaml(targetPath);

  if (!yamlPath) {
    // Try examples/benchmarks/ and examples/sample/
    const examples = join(repoPath, "examples");
    const dirs = existsSync(examples) ? await readdir(examples) : [];
    const available: string[] = [];
    for (const d of dirs) {
      if (findBenchmarkYaml(join(examples, d))) available.push(d);
    }
    if (available.length > 0) {
      console.log("No sensei-benchmark.yaml found in current directory.\n");
      console.log("Available corpora:");
      for (const a of available) console.log(`  sensei benchmark indexer --corpus examples/${a}`);
    } else {
      console.error("No sensei-benchmark.yaml found. Create one with the benchmark spec.");
    }
    process.exit(1);
  }

  const spec = await loadSpec(yamlPath);
  const sourceDir = resolve(join(targetPath, spec.source));

  console.log(`sensei benchmark indexer`);
  console.log(`  corpus:     ${spec.name}`);
  console.log(`  language:   ${spec.language}`);
  console.log(`  difficulty: ${spec.difficulty}`);
  console.log(`  source:     ${sourceDir}\n`);

  // Index the corpus
  const repoId = `benchmark-${spec.name}`;
  console.log("Indexing...");
  const result = await indexRepo({
    repoPath: targetPath,
    repoId,
    project: repoId,
    include: [`${spec.source}/**/*`],
  });
  console.log(`  ${result.filesIndexed} files, ${result.functionsIndexed} functions, ${result.typesIndexed} types, ${result.edgesCreated} edges\n`);

  // Query the graph for actual counts
  const { db, conn } = await getOrCreateDb(repoId);
  let actualSymbols = 0;
  let actualFunctions = 0;
  let actualTypes = 0;
  let actualEdges = 0;
  let actualCalls = 0;
  let actualImports = 0;
  let godNodeNames: string[] = [];

  try {
    // Count symbols
    const fnResult = await conn.query(`MATCH (f:Function {project: '${repoId}'}) RETURN COUNT(*) AS cnt`);
    const fnRows = Array.isArray(fnResult) ? fnResult[0] : fnResult;
    const fnData = await fnRows.getAll();
    actualFunctions = Number((fnData[0] as Record<string, unknown>)?.["cnt"] ?? 0);

    const typeResult = await conn.query(`MATCH (t:Type {project: '${repoId}'}) RETURN COUNT(*) AS cnt`);
    const typeRows = Array.isArray(typeResult) ? typeResult[0] : typeResult;
    const typeData = await typeRows.getAll();
    actualTypes = Number((typeData[0] as Record<string, unknown>)?.["cnt"] ?? 0);

    actualSymbols = actualFunctions + actualTypes;

    // Count edges
    try {
      const callResult = await conn.query(`MATCH ()-[r:CALLS]->() RETURN COUNT(*) AS cnt`);
      const callRows = Array.isArray(callResult) ? callResult[0] : callResult;
      const callData = await callRows.getAll();
      actualCalls = Number((callData[0] as Record<string, unknown>)?.["cnt"] ?? 0);
    } catch { /* edge table may not exist */ }

    try {
      const impResult = await conn.query(`MATCH ()-[r:IMPORTS]->() RETURN COUNT(*) AS cnt`);
      const impRows = Array.isArray(impResult) ? impResult[0] : impResult;
      const impData = await impRows.getAll();
      actualImports = Number((impData[0] as Record<string, unknown>)?.["cnt"] ?? 0);
    } catch { /* edge table may not exist */ }

    actualEdges = actualCalls + actualImports;

    // Find god nodes (top degree)
    try {
      const godResult = await conn.query(
        `MATCH (f:Function {project: '${repoId}'})<-[:CALLS]-(caller:Function)
         RETURN f.name AS name, COUNT(caller) AS degree
         ORDER BY degree DESC LIMIT 10`
      );
      const godRows = Array.isArray(godResult) ? godResult[0] : godResult;
      const godData = await godRows.getAll();
      godNodeNames = (godData as Record<string, unknown>[]).map(r => String(r["name"]));
    } catch { /* ignore */ }
  } finally {
    await conn.close();
    await db.close();
  }

  // Compare against expected
  const results: QualityResult[] = [];
  const exp = spec.expected;

  function addResult(metric: string, expected: number, found: number) {
    const precision = expected > 0 ? Math.min(found / expected, 1) : found === 0 ? 1 : 0;
    const recall = expected > 0 ? Math.min(found / expected, 1) : found === 0 ? 1 : 0;
    results.push({ metric, expected, found, precision, recall });
  }

  addResult("Symbols (total)", exp.symbols.total, actualSymbols);
  if (exp.symbols.functions != null) addResult("  functions", exp.symbols.functions, actualFunctions);
  if (exp.symbols.types != null) addResult("  types", exp.symbols.types, actualTypes);
  if (exp.edges.calls != null) addResult("CALLS edges", exp.edges.calls, actualCalls);
  if (exp.edges.imports != null) addResult("IMPORTS edges", exp.edges.imports, actualImports);

  // Print report
  console.log(`── Indexer Quality: ${spec.name} ───────────────────────────────────────────`);
  console.log(`${"Metric".padEnd(20)} ${"Expected".padStart(10)} ${"Found".padStart(10)} ${"Recall".padStart(10)}`);
  console.log("─".repeat(55));

  for (const r of results) {
    const recallStr = r.expected > 0 ? pct(r.found / r.expected) : "—";
    console.log(`${r.metric.padEnd(20)} ${String(r.expected).padStart(10)} ${String(r.found).padStart(10)} ${recallStr.padStart(10)}`);
  }
  console.log("─".repeat(55));

  // God nodes
  if (exp.communities?.god_nodes) {
    console.log(`\nGod nodes:`);
    for (const expected of exp.communities.god_nodes) {
      const found = godNodeNames.includes(expected);
      console.log(`  ${found ? "✓" : "✗"} ${expected}${found ? "" : " (not found)"}`);
    }
  }

  // Overall score
  const totalExpected = results.reduce((s, r) => s + r.expected, 0);
  const totalFound = results.reduce((s, r) => s + Math.min(r.found, r.expected), 0);
  const overallScore = totalExpected > 0 ? totalFound / totalExpected : 0;
  console.log(`\nOverall score: ${pct(overallScore)}`);
}
