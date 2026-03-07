export interface AnalyzedSymbol {
  name: string;
  kind: "function" | "class" | "type" | "const" | "interface" | "enum" | "method" | "hook" | "component";
  signature: string;      // L0 — concise "what"
  description: string;    // L1 — plain-English explanation
  visibility: "public" | "internal";
  tags?: string[];        // ["async", "pure", "exported", "deprecated"]
}

export interface Flow {
  name: string;           // e.g. "happy path", "error path"
  steps: string[];        // ordered plain-English steps
}

export interface Relation {
  kind: "imports" | "calls" | "implements" | "extends" | "covers";
  target: string;         // relative path or module name
}

export interface FileAnalysis {
  path: string;
  language: string;
  contentHash: string;    // sha256 of source — incremental cache key
  analyzedAt: string;     // ISO timestamp

  symbols: AnalyzedSymbol[];
  summary: string;        // 1-2 sentence file purpose
  role?: string;          // "component" | "service" | "util" | "config" | "test" | "unknown"

  flows?: Flow[];
  examples?: string[];
  relations?: Relation[];

  embedding?: number[];   // from embed(summary + symbol names)
}

export interface ExtractionInstructions {
  filePath: string;
  language?: string;
  techContext?: string;   // e.g. "React + TypeScript project"
  focusHints?: string[];  // e.g. ["extract React hooks", "note exported props"]
}

export interface ModelBackend {
  name: string;
  init(): Promise<void>;
  embed(text: string): Promise<number[]>;
  extract(content: string, instructions: ExtractionInstructions): Promise<FileAnalysis>;
  isAvailable(): Promise<boolean>;
}

export interface IndexConfig {
  backend: "ollama" | "regex";
  ollamaModel?: string;
  embeddingModel?: string;
  embeddingReady: boolean;
  indexedAt?: string;
  version: number;
}

export interface SetupStatus {
  ollamaBinary: boolean;
  ollamaRunning: boolean;
  ollamaModel: boolean;
  ollamaModelName: string;
  onnxModel: boolean;
  diskFreeGB: number;
  ramTotalGB: number;
  ramAvailableGB: number;
}
