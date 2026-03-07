// Query tools
export { getLlmSpec, getFileContext, listExports, findPattern, getShortcuts } from "./tools/query.js";

// Reindex
export { reindexRepo } from "./tools/reindex.js";
export type { IndexSummary } from "./tools/reindex.js";

// Context
export { loadContext, recommendNext } from "./tools/context.js";

// Drift
export { checkDrift } from "./tools/drift.js";

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
