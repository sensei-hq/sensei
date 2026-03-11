import type { GroundTruth } from "./ground-truth.js";
import type { CocoIndex } from "./cocoindex-adapter.js";
import type { SenseiIndex } from "./sensei-adapter.js";
import type { ComparisonReport, QueryComparison, SpotCheckRow } from "./types.js";

const TEST_QUERIES = [
  { query: "reindex repository symbols", expectedFiles: ["reindex.ts"] },
  { query: "check documentation drift", expectedFiles: ["drift.ts"] },
  { query: "load session context", expectedFiles: ["context.ts"] },
  { query: "checkpoint project memory", expectedFiles: ["project-memory.ts"] },
  { query: "list exported symbols", expectedFiles: ["query.ts"] },
];

function fileMatchesAny(filePath: string, patterns: string[]): boolean {
  return patterns.some(p => filePath.includes(p));
}

export async function score(
  groundTruth: GroundTruth,
  cocoIndex: CocoIndex,
  senseiIndex: SenseiIndex
): Promise<ComparisonReport> {
  const gtSet = new Set(groundTruth.files);
  const cocoMatched = cocoIndex.files.filter(f => gtSet.has(f)).length;
  const senseiMatched = senseiIndex.files.filter(f => gtSet.has(f)).length;

  const queryResults: QueryComparison[] = [];
  for (const { query, expectedFiles } of TEST_QUERIES) {
    const cocoResults = await cocoIndex.search(query, 5);
    const cocoHit = cocoResults.some(r => fileMatchesAny(r.filePath, expectedFiles));

    const senseiHit = senseiIndex.symbols.some(s =>
      fileMatchesAny(s.path, expectedFiles) ||
      (s.L1?.toLowerCase().includes(query.split(" ")[0].toLowerCase()) ?? false)
    );

    queryResults.push({ query, cocoHit, senseiHit });
  }

  const sampleFiles = groundTruth.files.slice(0, 15);
  const spotCheck: SpotCheckRow[] = await Promise.all(
    sampleFiles.map(async (filePath): Promise<SpotCheckRow> => {
      const cocoChunks = await cocoIndex.search(`file:${filePath}`, 1);
      const cocoContent = cocoChunks[0]?.content.slice(0, 200) ?? null;

      const senseiSymbol = senseiIndex.symbols.find(s => s.path === filePath);
      const senseiDescription = senseiSymbol?.L1 ?? senseiSymbol?.L0 ?? null;

      return { filePath, cocoContent, senseiDescription };
    })
  );

  return {
    cocoFilesIndexed: cocoIndex.files.length,
    senseiFilesIndexed: senseiIndex.files.length,
    groundTruthExports: groundTruth.exportCount,
    cocoCoverage: gtSet.size > 0 ? cocoMatched / gtSet.size : 0,
    senseiCoverage: gtSet.size > 0 ? senseiMatched / gtSet.size : 0,
    queryResults,
    spotCheck,
  };
}
