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

// Platform auth middleware
export { resolveCallerContext, handleAuthVerify, callerFilter } from "./auth-middleware.js";
export type { SessionCallerContext, AuthVerifyResult } from "./auth-middleware.js";
