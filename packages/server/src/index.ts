// Server HTTP API
export { createReportServer, serve } from "./serve.js";
export type { ServeOptions } from "./serve.js";

// Model / inference
export { OllamaBackend, makeFallbackAnalysis, extractJson } from "./model/ollama-backend.js";
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
