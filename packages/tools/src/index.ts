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
