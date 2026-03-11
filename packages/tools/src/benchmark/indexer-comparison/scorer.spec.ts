import { describe, it, expect } from "vitest";
import { score } from "./scorer.js";
import type { GroundTruth } from "./ground-truth.js";
import type { CocoIndex } from "./cocoindex-adapter.js";
import type { SenseiIndex } from "./sensei-adapter.js";

const groundTruth: GroundTruth = {
  files: ["packages/tools/src/tools/reindex.ts", "packages/tools/src/tools/drift.ts"],
  exportCount: 10,
};

const cocoIndex: CocoIndex = {
  files: ["packages/tools/src/tools/reindex.ts"],
  async search(query: string) {
    if (query.includes("reindex")) return [{ filePath: "packages/tools/src/tools/reindex.ts", language: "typescript", content: "export async function reindexRepo(", startLine: 1, endLine: 5, score: 0.9 }];
    return [];
  },
  async close() {},
};

const senseiIndex: SenseiIndex = {
  symbols: [{ name: "reindexRepo", path: "packages/tools/src/tools/reindex.ts", L0: "export function reindexRepo", L1: "Rebuilds the index." }],
  files: ["packages/tools/src/tools/reindex.ts", "packages/tools/src/tools/drift.ts"],
  missing: false,
};

describe("score", () => {
  it("computes file coverage correctly", async () => {
    const report = await score(groundTruth, cocoIndex, senseiIndex);
    expect(report.cocoFilesIndexed).toBe(1);
    expect(report.senseiFilesIndexed).toBe(2);
    expect(report.cocoCoverage).toBeCloseTo(0.5, 1);
    expect(report.senseiCoverage).toBeCloseTo(1.0, 1);
  });

  it("scores query hits correctly", async () => {
    const report = await score(groundTruth, cocoIndex, senseiIndex);
    const reindexQuery = report.queryResults.find(q => q.query.includes("reindex"));
    expect(reindexQuery?.cocoHit).toBe(true);
  });
});
