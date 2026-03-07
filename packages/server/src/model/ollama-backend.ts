import { createHash } from "crypto";
import type { ModelBackend, FileAnalysis, ExtractionInstructions } from "@sensei/shared";

export interface OllamaOptions {
  model?: string;
  baseUrl?: string;
}

const EXTRACTION_PROMPT = (filePath: string, language: string, hints: string, content: string) => `
Analyze the following ${language} file and respond with ONLY a JSON object matching this exact structure (no prose before or after):

{
  "path": "${filePath}",
  "language": "${language}",
  "contentHash": "",
  "analyzedAt": "",
  "summary": "1-2 sentence description of what this file does",
  "role": "one of: component, service, util, config, test, cli, unknown",
  "symbols": [
    {
      "name": "symbolName",
      "kind": "one of: function, class, type, const, interface, enum, method, hook, component",
      "signature": "concise signature",
      "description": "brief plain-English explanation",
      "visibility": "public or internal",
      "tags": []
    }
  ],
  "flows": [],
  "examples": [],
  "relations": []
}

${hints ? `Focus on: ${hints}` : ""}

File: ${filePath}
---
${content.slice(0, 6000)}
`.trim();

/** Extract the first complete JSON object from a string that may have prose around it. */
export function extractJson(text: string): unknown | null {
  const start = text.indexOf("{");
  const end = text.lastIndexOf("}");
  if (start === -1 || end === -1 || end <= start) return null;
  try {
    return JSON.parse(text.slice(start, end + 1));
  } catch {
    return null;
  }
}

export function makeFallbackAnalysis(filePath: string): FileAnalysis {
  return {
    path: filePath,
    language: "unknown",
    contentHash: "",
    analyzedAt: new Date().toISOString(),
    summary: "Could not analyze this file",
    role: "unknown",
    symbols: [],
  };
}

export class OllamaBackend implements ModelBackend {
  readonly name = "ollama";
  private model: string;
  private baseUrl: string;

  constructor(opts: OllamaOptions = {}) {
    this.model = opts.model ?? "llama3.2:3b";
    this.baseUrl = opts.baseUrl ?? "http://127.0.0.1:11434";
  }

  async init(): Promise<void> {}

  async isAvailable(): Promise<boolean> {
    try {
      const res = await fetch(`${this.baseUrl}/api/tags`, { signal: AbortSignal.timeout(2000) });
      return res.ok;
    } catch {
      return false;
    }
  }

  async extract(content: string, instructions: ExtractionInstructions): Promise<FileAnalysis> {
    const language = instructions.language ?? inferLanguage(instructions.filePath);
    const hints = instructions.focusHints?.join(", ") ?? "";
    const prompt = EXTRACTION_PROMPT(instructions.filePath, language, hints, content);

    try {
      const res = await fetch(`${this.baseUrl}/api/generate`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ model: this.model, prompt, stream: false }),
        signal: AbortSignal.timeout(60_000),
      });
      const data = await res.json() as { response?: string };
      const parsed = extractJson(data.response ?? "") as Partial<FileAnalysis> | null;
      if (!parsed || !parsed.summary) return makeFallbackAnalysis(instructions.filePath);

      return {
        ...makeFallbackAnalysis(instructions.filePath),
        ...parsed,
        path: instructions.filePath,
        contentHash: createHash("sha256").update(content).digest("hex").slice(0, 16),
        analyzedAt: new Date().toISOString(),
        symbols: parsed.symbols ?? [],
      };
    } catch {
      return makeFallbackAnalysis(instructions.filePath);
    }
  }

  async embed(text: string): Promise<number[]> {
    try {
      const res = await fetch(`${this.baseUrl}/api/embeddings`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ model: this.model, prompt: text }),
        signal: AbortSignal.timeout(10_000),
      });
      if (!res.ok) return [];
      const data = await res.json() as { embedding?: number[] };
      return data.embedding ?? [];
    } catch {
      return [];
    }
  }
}

function inferLanguage(filePath: string): string {
  const ext = filePath.split(".").at(-1) ?? "";
  const map: Record<string, string> = {
    ts: "typescript", tsx: "typescript", js: "javascript", jsx: "javascript",
    py: "python", go: "go", rs: "rust", rb: "ruby", java: "java",
    md: "markdown", yaml: "yaml", yml: "yaml", json: "json", sh: "bash",
  };
  return map[ext] ?? "unknown";
}
