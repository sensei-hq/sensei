export interface LlmSpec {
  project: string;
  version: string;
  description: string;
  stack: string[];
  entry_points: Array<{ path: string; role: string }>;
  concepts: Array<{ name: string; definition: string }>;
  patterns: Array<{ name: string; files: string; convention: string }>;
  api_surface: Array<{ name: string; path: string; io: string; flow: string }>;
  doc_layers: { design: string; code: string; public: string[] };
  shortcuts: Record<string, string>;
}

export type ResolutionLevel = "L0" | "L1" | "L2" | "L3";

export interface SymbolEntry {
  L0: string[];
  L1: string[];
  L2: string[];
  // L3 is the raw file — never cached in symbol-map
}

export type SymbolMap = Record<string, SymbolEntry>;
