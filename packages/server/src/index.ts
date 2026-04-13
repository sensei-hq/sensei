// Server HTTP API
export { createReportServer, serve } from "./serve.js";
export type { ServeOptions } from "./serve.js";

// Model / inference
export { OllamaBackend, makeFallbackAnalysis, extractJson } from "./model/ollama-backend.js";
export { ClaudeBackend } from "./model/claude-backend.js";
export type { ClaudeBackendOptions } from "./model/claude-backend.js";
export {
  checkSystemRequirements,
  getDiskFreeGB,
  getRamGB,
  OLLAMA_BASE_URL,
  OLLAMA_MODEL,
  OLLAMA_MODEL_SIZE_GB,
  ONNX_MODEL_ID,
  ONNX_MODEL_SIZE_MB,
} from "./model/system-check.js";

export * from "./mcp-server.js";

export { getActivityLog, ActivityLog } from "./activity-log.js";
export type { ActivitySession, ActivityAction, Decision, BacklogItem } from "./activity-log.js";

export { LibIndexer } from "./lib-indexer.js";
export { getLibDocs, replaceLibDocs, listLibraries, getLibMeta, writeLibMeta, addLibUser } from "./lib-store.js";
