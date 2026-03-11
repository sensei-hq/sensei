// Index reader
export { readLlmSpec, readSymbolMap, readIndexFile } from "./index-reader.js";

// Coverage
export { generateCoverage } from "./tools/coverage.js";
export type { CoverageEntry } from "./tools/coverage.js";

// Query tools
export { getLlmSpec, getFileContext, listExports, findPattern, getShortcuts } from "./tools/query.js";

// Reindex
export { reindexRepo } from "./tools/reindex.js";
export type { IndexSummary } from "./tools/reindex.js";

// Context
export { loadContext, recommendNext } from "./tools/context.js";
export type { ContextSlice } from "./tools/context.js";

// Drift
export { checkDrift } from "./tools/drift.js";
export type { DriftEntry, DriftResult } from "./tools/drift.js";

// Benchmark: indexer comparison
export { extractGroundTruth } from "./benchmark/indexer-comparison/ground-truth.js";
export type { GroundTruth } from "./benchmark/indexer-comparison/ground-truth.js";
export { loadSenseiIndex } from "./benchmark/indexer-comparison/sensei-adapter.js";
export type { SenseiIndex } from "./benchmark/indexer-comparison/sensei-adapter.js";
export { connectCocoindex } from "./benchmark/indexer-comparison/cocoindex-adapter.js";
export type { CocoIndex } from "./benchmark/indexer-comparison/cocoindex-adapter.js";
export { score } from "./benchmark/indexer-comparison/scorer.js";
export { printReport, writeMarkdownReport } from "./benchmark/indexer-comparison/reporter.js";

// Project memory
export {
  checkpoint,
  getSessionContext,
  addDecision,
  addPattern,
  askQuestion,
  getOpenItems,
  closeItem,
} from "./tools/project-memory.js";

// Search
export { search } from "./tools/search.js";
export type { SearchResult } from "./tools/search.js";
