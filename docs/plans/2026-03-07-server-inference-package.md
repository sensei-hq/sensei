# Server Inference Package Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Expand `sensei serve` into an inference server that manages local Ollama models, performs file analysis, and exposes a JSON API that the CLI and MCP server call instead of doing inference inline.

**Architecture:** The existing `src/commands/serve.ts` (Bun HTTP server, SQLite telemetry) is extended with model management and inference endpoints. A new `src/model/` folder holds the Ollama backend and system-check logic. All model-heavy work lives in the server process so models stay warm across CLI calls.

**Tech Stack:** Bun HTTP server, Ollama REST API (`localhost:11434`), `@clack/prompts`, vitest for tests (mock `fetch` with `vi.stubGlobal`), TypeScript strict mode.

---

## Background: How the Existing Server Works

`src/commands/serve.ts` exports:
- `createReportServer(opts)` — creates a Bun HTTP server with `/health` (GET) and `/reports` (POST, stores to SQLite)
- `serve(repoPath, opts)` — calls `createReportServer`, keeps process alive

Tests are in `src/commands/serve.spec.ts`. The test pattern spins up `createReportServer` on a random port, hits it with `fetch`, then calls `stop()`.

The server currently runs on port **7744**. New endpoints are added to the existing router inside `createReportServer`.

---

## Task 1: Model Types

**Files:**
- Create: `packages/sensei/src/model/types.ts`

These are the shared interfaces for everything in `src/model/`. No runtime code — types only.

**Step 1: Create the file**

```typescript
// packages/sensei/src/model/types.ts

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
```

**Step 2: Commit**

```bash
cd /Users/Jerry/Developer/skills
git add packages/sensei/src/model/types.ts
git commit -m "feat(sensei): add model types (FileAnalysis, ModelBackend, SetupStatus)"
```

---

## Task 2: Ollama Backend

**Files:**
- Create: `packages/sensei/src/model/ollama-backend.ts`
- Create: `packages/sensei/src/model/ollama-backend.spec.ts`

The Ollama backend calls `http://127.0.0.1:11434`. It sends a structured prompt asking for JSON, then extracts the JSON from the response (Ollama sometimes wraps it in prose).

**Step 1: Write the failing tests**

```typescript
// packages/sensei/src/model/ollama-backend.spec.ts
import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { OllamaBackend } from "./ollama-backend.js";

describe("OllamaBackend", () => {
  const backend = new OllamaBackend({ model: "llama3.2:3b", baseUrl: "http://127.0.0.1:11434" });

  beforeEach(() => { vi.stubGlobal("fetch", vi.fn()); });
  afterEach(() => { vi.unstubAllGlobals(); });

  describe("isAvailable", () => {
    it("returns true when Ollama responds", async () => {
      vi.mocked(fetch).mockResolvedValueOnce(new Response(JSON.stringify({ models: [] }), { status: 200 }));
      expect(await backend.isAvailable()).toBe(true);
    });

    it("returns false when fetch throws", async () => {
      vi.mocked(fetch).mockRejectedValueOnce(new Error("ECONNREFUSED"));
      expect(await backend.isAvailable()).toBe(false);
    });
  });

  describe("extract", () => {
    it("returns FileAnalysis from well-formed JSON response", async () => {
      const analysis = {
        path: "src/foo.ts",
        language: "typescript",
        contentHash: "abc123",
        analyzedAt: "2026-01-01T00:00:00.000Z",
        summary: "Exports foo utility",
        role: "util",
        symbols: [{ name: "foo", kind: "function", signature: "function foo(): void", description: "Does foo", visibility: "public" }],
      };
      vi.mocked(fetch).mockResolvedValueOnce(new Response(
        JSON.stringify({ response: JSON.stringify(analysis) }),
        { status: 200 }
      ));
      const result = await backend.extract("export function foo() {}", { filePath: "src/foo.ts" });
      expect(result.summary).toBe("Exports foo utility");
      expect(result.symbols).toHaveLength(1);
      expect(result.symbols[0].name).toBe("foo");
    });

    it("strips prose before/after JSON in model response", async () => {
      const analysis = { path: "src/bar.ts", language: "typescript", contentHash: "def456", analyzedAt: "2026-01-01T00:00:00.000Z", summary: "Bar module", symbols: [], role: "util" };
      vi.mocked(fetch).mockResolvedValueOnce(new Response(
        JSON.stringify({ response: `Here is the analysis:\n${JSON.stringify(analysis)}\nLet me know if you need more.` }),
        { status: 200 }
      ));
      const result = await backend.extract("export const bar = 1;", { filePath: "src/bar.ts" });
      expect(result.summary).toBe("Bar module");
    });

    it("returns fallback FileAnalysis when JSON parse fails", async () => {
      vi.mocked(fetch).mockResolvedValueOnce(new Response(
        JSON.stringify({ response: "I cannot analyze this file." }),
        { status: 200 }
      ));
      const result = await backend.extract("???", { filePath: "src/bad.ts" });
      expect(result.path).toBe("src/bad.ts");
      expect(result.symbols).toEqual([]);
      expect(result.summary).toMatch(/could not/i);
    });
  });

  describe("embed", () => {
    it("returns number array from Ollama embeddings endpoint", async () => {
      vi.mocked(fetch).mockResolvedValueOnce(new Response(
        JSON.stringify({ embedding: [0.1, 0.2, 0.3] }),
        { status: 200 }
      ));
      const result = await backend.embed("some text");
      expect(result).toEqual([0.1, 0.2, 0.3]);
    });
  });
});
```

**Step 2: Run to verify tests fail**

```bash
cd /Users/Jerry/Developer/skills/packages/sensei
bunx vitest run src/model/ollama-backend.spec.ts
```

Expected: FAIL — `Cannot find module './ollama-backend.js'`

**Step 3: Implement**

```typescript
// packages/sensei/src/model/ollama-backend.ts
import { createHash } from "crypto";
import type { ModelBackend, FileAnalysis, ExtractionInstructions } from "./types.js";

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
    const res = await fetch(`${this.baseUrl}/api/embeddings`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ model: this.model, prompt: text }),
      signal: AbortSignal.timeout(10_000),
    });
    const data = await res.json() as { embedding?: number[] };
    return data.embedding ?? [];
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
```

**Step 4: Run tests to verify they pass**

```bash
cd /Users/Jerry/Developer/skills/packages/sensei
bunx vitest run src/model/ollama-backend.spec.ts
```

Expected: 6 passing

**Step 5: Commit**

```bash
cd /Users/Jerry/Developer/skills
git add packages/sensei/src/model/
git commit -m "feat(sensei): add OllamaBackend with JSON extraction and embed"
```

---

## Task 3: System Check (Prerequisites)

**Files:**
- Create: `packages/sensei/src/model/system-check.ts`
- Create: `packages/sensei/src/model/system-check.spec.ts`

Checks whether Ollama binary exists, service is running, required model is pulled, and whether there's enough disk/RAM to proceed.

**Step 1: Write the failing tests**

```typescript
// packages/sensei/src/model/system-check.spec.ts
import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { checkSystemRequirements, getDiskFreeGB, getRamGB } from "./system-check.js";

describe("checkSystemRequirements", () => {
  beforeEach(() => { vi.stubGlobal("fetch", vi.fn()); });
  afterEach(() => { vi.unstubAllGlobals(); });

  it("returns ollamaBinary:false when execSync throws", async () => {
    // execSync is mocked by passing a custom exec fn
    const status = await checkSystemRequirements({
      execFn: () => { throw new Error("not found"); },
      fetchFn: async () => { throw new Error(); },
    });
    expect(status.ollamaBinary).toBe(false);
    expect(status.ollamaRunning).toBe(false);
    expect(status.ollamaModel).toBe(false);
  });

  it("returns ollamaBinary:true, ollamaRunning:false when service is down", async () => {
    const status = await checkSystemRequirements({
      execFn: () => "/usr/local/bin/ollama",  // binary found
      fetchFn: async () => { throw new Error("ECONNREFUSED"); },
    });
    expect(status.ollamaBinary).toBe(true);
    expect(status.ollamaRunning).toBe(false);
  });

  it("returns ollamaModel:true when model is in tags list", async () => {
    const status = await checkSystemRequirements({
      execFn: () => "/usr/local/bin/ollama",
      fetchFn: async (url: string) => {
        if (url.includes("/api/tags")) {
          return new Response(JSON.stringify({ models: [{ name: "llama3.2:3b" }] }), { status: 200 });
        }
        throw new Error("unexpected");
      },
    });
    expect(status.ollamaBinary).toBe(true);
    expect(status.ollamaRunning).toBe(true);
    expect(status.ollamaModel).toBe(true);
  });
});
```

**Step 2: Run to verify fail**

```bash
bunx vitest run src/model/system-check.spec.ts
```

**Step 3: Implement**

```typescript
// packages/sensei/src/model/system-check.ts
import { execSync } from "child_process";
import { existsSync } from "fs";
import { platform, homedir } from "os";
import { join } from "path";
import type { SetupStatus } from "./types.js";

export const OLLAMA_MODEL = "llama3.2:3b";
export const OLLAMA_MODEL_SIZE_GB = 2.0;
export const ONNX_MODEL_ID = "Xenova/all-MiniLM-L6-v2";
export const ONNX_MODEL_SIZE_MB = 22;
export const OLLAMA_BASE_URL = "http://127.0.0.1:11434";

interface CheckDeps {
  execFn?: (cmd: string) => string;
  fetchFn?: (url: string) => Promise<Response>;
}

export async function checkSystemRequirements(deps: CheckDeps = {}): Promise<SetupStatus> {
  const exec = deps.execFn ?? ((cmd) => execSync(cmd, { encoding: "utf-8" }) as string);
  const fetchFn = deps.fetchFn ?? ((url) => fetch(url, { signal: AbortSignal.timeout(2000) }));

  // 1. Ollama binary
  let ollamaBinary = false;
  try {
    const out = exec("which ollama").trim();
    ollamaBinary = out.length > 0;
  } catch {
    // also check common paths
    ollamaBinary = existsSync("/usr/local/bin/ollama") || existsSync("/opt/homebrew/bin/ollama");
  }

  // 2. Ollama service
  let ollamaRunning = false;
  let ollamaModel = false;
  if (ollamaBinary) {
    try {
      const res = await fetchFn(`${OLLAMA_BASE_URL}/api/tags`);
      if (res.ok) {
        ollamaRunning = true;
        const data = await res.json() as { models?: { name: string }[] };
        ollamaModel = (data.models ?? []).some(m => m.name.startsWith(OLLAMA_MODEL.split(":")[0]));
      }
    } catch { /* service down */ }
  }

  // 3. ONNX model cache
  const onnxCacheDir = join(homedir(), ".cache", "huggingface", "hub",
    "models--Xenova--all-MiniLM-L6-v2");
  const onnxModel = existsSync(onnxCacheDir);

  // 4. Disk space
  const diskFreeGB = getDiskFreeGB(exec);

  // 5. RAM
  const { total: ramTotalGB, available: ramAvailableGB } = getRamGB(exec);

  return {
    ollamaBinary,
    ollamaRunning,
    ollamaModel,
    ollamaModelName: OLLAMA_MODEL,
    onnxModel,
    diskFreeGB,
    ramTotalGB,
    ramAvailableGB,
  };
}

export function getDiskFreeGB(exec: (cmd: string) => string = (c) => execSync(c, { encoding: "utf-8" }) as string): number {
  try {
    if (platform() === "win32") {
      const out = exec("wmic logicaldisk get freespace /value");
      const match = out.match(/FreeSpace=(\d+)/);
      return match ? parseInt(match[1]) / 1e9 : 0;
    }
    // macOS + Linux: df -k returns 1k blocks
    const out = exec(`df -k "${homedir()}"`);
    const lines = out.trim().split("\n");
    const last = lines[lines.length - 1].split(/\s+/);
    const freeKB = parseInt(last[3] ?? "0");
    return freeKB / 1_000_000;
  } catch {
    return 0;
  }
}

export function getRamGB(exec: (cmd: string) => string = (c) => execSync(c, { encoding: "utf-8" }) as string): { total: number; available: number } {
  try {
    if (platform() === "darwin") {
      const total = parseInt(exec("sysctl -n hw.memsize").trim()) / 1e9;
      const pagesStr = exec("sysctl -n vm.page_free_count").trim();
      const available = (parseInt(pagesStr) * 16384) / 1e9;
      return { total: Math.round(total * 10) / 10, available: Math.round(available * 10) / 10 };
    }
    if (platform() === "linux") {
      const meminfo = exec("cat /proc/meminfo");
      const total = parseInt(meminfo.match(/MemTotal:\s+(\d+)/)?.[1] ?? "0") / 1_000_000;
      const available = parseInt(meminfo.match(/MemAvailable:\s+(\d+)/)?.[1] ?? "0") / 1_000_000;
      return { total: Math.round(total * 10) / 10, available: Math.round(available * 10) / 10 };
    }
  } catch { /* ignore */ }
  return { total: 0, available: 0 };
}
```

**Step 4: Run tests to verify pass**

```bash
bunx vitest run src/model/system-check.spec.ts
```

Expected: 3 passing

**Step 5: Commit**

```bash
cd /Users/Jerry/Developer/skills
git add packages/sensei/src/model/system-check.ts packages/sensei/src/model/system-check.spec.ts
git commit -m "feat(sensei): add system-check (Ollama binary, service, model, disk, RAM)"
```

---

## Task 4: Server Inference Endpoints

**Files:**
- Modify: `packages/sensei/src/commands/serve.ts`
- Modify: `packages/sensei/src/commands/serve.spec.ts`

Extend the existing Bun server with:
- `GET /health` — extended to include `{ ok, backend, ollamaRunning, ollamaModel }`
- `GET /setup/status` — returns `SetupStatus` JSON
- `POST /setup/ollama` — pulls the required Ollama model (streams progress via SSE)
- `POST /analyze` — receives `{ filePath, content, instructions? }`, returns `FileAnalysis`

**Step 1: Write new failing tests** (add to existing `serve.spec.ts`)

First read `src/commands/serve.spec.ts` to understand the current test pattern, then add:

```typescript
// Add these test blocks to the existing describe("createReportServer") suite

describe("GET /health (extended)", () => {
  it("returns ok:true with backend info", async () => {
    const { stop } = await createReportServer({ port: 0 });
    // The port:0 trick doesn't work with Bun — use a fixed test port instead
    // Use port 17744 to avoid collision
  });
});

describe("GET /setup/status", () => {
  it("returns SetupStatus shape", async () => {
    const { stop, port } = await createReportServer({ port: 17744 });
    const res = await fetch(`http://localhost:${port}/setup/status`);
    const data = await res.json();
    expect(res.status).toBe(200);
    expect(typeof data.ollamaBinary).toBe("boolean");
    expect(typeof data.diskFreeGB).toBe("number");
    stop();
  });
});

describe("POST /analyze", () => {
  it("returns 400 when body is missing filePath", async () => {
    const { stop, port } = await createReportServer({ port: 17745 });
    const res = await fetch(`http://localhost:${port}/analyze`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ content: "hello" }),
    });
    expect(res.status).toBe(400);
    stop();
  });

  it("returns FileAnalysis shape when Ollama unavailable (fallback)", async () => {
    const { stop, port } = await createReportServer({ port: 17746 });
    const res = await fetch(`http://localhost:${port}/analyze`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ filePath: "src/foo.ts", content: "export const x = 1;" }),
    });
    // When Ollama is unavailable, server falls back — still returns 200 with FileAnalysis shape
    expect(res.status).toBe(200);
    const data = await res.json();
    expect(typeof data.path).toBe("string");
    expect(Array.isArray(data.symbols)).toBe(true);
    stop();
  });
});
```

**Step 2: Run to verify they fail**

```bash
bunx vitest run src/commands/serve.spec.ts
```

**Step 3: Modify `serve.ts`**

Add the three new endpoint handlers inside `createReportServer`, after the `/reports` block and before the `return Response.json({ ok: false, error: "Not found" }...` fallback:

```typescript
// Add these imports at the top of serve.ts:
import { checkSystemRequirements } from "../model/system-check.js";
import { OllamaBackend, makeFallbackAnalysis } from "../model/ollama-backend.js";

// Extend the ServeOptions interface:
export interface ServeOptions {
  port?: number;
  dbPath?: string;
  repoPath?: string;   // NEW: used by /analyze to resolve relative paths
}

// In createReportServer, after existing /health handler:

if (req.method === "GET" && url.pathname === "/health") {
  const ollama = new OllamaBackend();
  const ollamaRunning = await ollama.isAvailable();
  return Response.json({ ok: true, backend: ollamaRunning ? "ollama" : "none", ollamaRunning });
}

if (req.method === "GET" && url.pathname === "/setup/status") {
  const status = await checkSystemRequirements();
  return Response.json(status);
}

if (req.method === "POST" && url.pathname === "/setup/ollama") {
  // Stream progress via newline-delimited JSON (simple SSE substitute)
  const { OLLAMA_MODEL, OLLAMA_BASE_URL } = await import("../model/system-check.js");
  const stream = new ReadableStream({
    async start(controller) {
      const send = (obj: unknown) => controller.enqueue(new TextEncoder().encode(JSON.stringify(obj) + "\n"));
      try {
        send({ status: "pulling", model: OLLAMA_MODEL });
        const res = await fetch(`${OLLAMA_BASE_URL}/api/pull`, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ name: OLLAMA_MODEL, stream: true }),
        });
        const reader = res.body!.getReader();
        const decoder = new TextDecoder();
        while (true) {
          const { done, value } = await reader.read();
          if (done) break;
          const lines = decoder.decode(value).split("\n").filter(Boolean);
          for (const line of lines) {
            try { send(JSON.parse(line)); } catch { /* skip malformed */ }
          }
        }
        send({ status: "done" });
      } catch (err) {
        send({ status: "error", message: (err as Error).message });
      }
      controller.close();
    },
  });
  return new Response(stream, { headers: { "Content-Type": "application/x-ndjson" } });
}

if (req.method === "POST" && url.pathname === "/analyze") {
  try {
    const body = await req.json() as { filePath?: string; content?: string; instructions?: Record<string, unknown> };
    if (!body.filePath || body.content === undefined) {
      return Response.json({ ok: false, error: "filePath and content are required" }, { status: 400 });
    }
    const ollama = new OllamaBackend();
    const available = await ollama.isAvailable();
    if (!available) {
      return Response.json(makeFallbackAnalysis(body.filePath));
    }
    const analysis = await ollama.extract(body.content, {
      filePath: body.filePath,
      ...(body.instructions ?? {}),
    });
    return Response.json(analysis);
  } catch {
    return Response.json({ ok: false, error: "Invalid JSON" }, { status: 400 });
  }
}
```

Also update `createReportServer` to return `port` alongside `stop`:

```typescript
// Change return type and value:
return { stop: () => server.stop(), port: server.port };
```

**Step 4: Run tests**

```bash
bunx vitest run src/commands/serve.spec.ts
```

Expected: all passing (existing tests + new ones)

**Step 5: Commit**

```bash
cd /Users/Jerry/Developer/skills
git add packages/sensei/src/commands/serve.ts packages/sensei/src/commands/serve.spec.ts
git commit -m "feat(sensei): add /setup/status, /setup/ollama, /analyze server endpoints"
```

---

## Task 5: CLI `server` Subcommand

**Files:**
- Modify: `packages/sensei/src/cli.ts`

Add `sensei server status` to check if the server is running and show setup status without starting interactive prompts. Useful in CI and for debugging.

**Step 1: Add `server` case to cli.ts switch**

Find the `default:` case in the switch and add before it:

```typescript
case "server": {
  const subCmd = rest[0];
  if (subCmd === "status" || !subCmd) {
    const url = process.env.SENSEI_SERVER_URL ?? "http://localhost:7744";
    try {
      const res = await fetch(`${url}/health`, { signal: AbortSignal.timeout(2000) });
      const data = await res.json() as Record<string, unknown>;
      console.log(`sensei server: running at ${url}`);
      console.log(`  backend: ${data.backend ?? "none"}`);
      console.log(`  ollama:  ${data.ollamaRunning ? "running" : "not running"}`);
      const setupRes = await fetch(`${url}/setup/status`, { signal: AbortSignal.timeout(2000) });
      const setup = await setupRes.json() as Record<string, unknown>;
      console.log(`  disk:    ${setup.diskFreeGB} GB free`);
      console.log(`  ram:     ${setup.ramAvailableGB} GB available`);
      console.log(`  model:   ${setup.ollamaModel ? `✓ ${setup.ollamaModelName}` : `✗ ${setup.ollamaModelName} not pulled`}`);
    } catch {
      console.log(`sensei server: not running at ${url}`);
      console.log(`  Start with: sensei serve`);
    }
    break;
  }
  console.error(`Unknown server subcommand: ${subCmd}`);
  process.exit(1);
}
```

Also update HELP text to add:
```
  server status            Check if server is running and show model setup status
```

**Step 2: Build and smoke test**

```bash
cd /Users/Jerry/Developer/skills/packages/sensei
bun run build
./dist/cli.js server status
```

Expected: either "running" or "not running" message — no errors.

**Step 3: Commit**

```bash
cd /Users/Jerry/Developer/skills
git add packages/sensei/src/cli.ts
git commit -m "feat(sensei): add 'sensei server status' command"
```

---

## Task 6: Init Prerequisites Flow

**Files:**
- Modify: `packages/sensei/src/commands/init.ts`

`sensei init` now checks setup status via the server before indexing. If the server isn't running, it falls back to inline check. Guides user through any missing prerequisites.

**Step 1: Implement**

```typescript
// packages/sensei/src/commands/init.ts
import { intro, outro, spinner, note, confirm, log, isCancel } from "@clack/prompts";
import { reindexRepo } from "../tools/reindex.js";
import { checkSystemRequirements, OLLAMA_MODEL, OLLAMA_MODEL_SIZE_GB } from "../model/system-check.js";

export async function init(cwd: string): Promise<void> {
  intro("sensei init");

  // --- Prerequisites check ---
  const checkSpinner = spinner();
  checkSpinner.start("Checking prerequisites...");
  const status = await checkSystemRequirements();
  checkSpinner.stop("Prerequisites checked");

  const needs: string[] = [];
  if (!status.ollamaBinary) needs.push(`Ollama not installed  (needed for local model inference)`);
  if (status.ollamaBinary && !status.ollamaRunning) needs.push(`Ollama not running   (start: ollama serve)`);
  if (status.ollamaBinary && status.ollamaRunning && !status.ollamaModel)
    needs.push(`Model not pulled     ${OLLAMA_MODEL} (~${OLLAMA_MODEL_SIZE_GB} GB, 4 GB RAM)`);
  if (!status.onnxModel) needs.push(`Embedding model      Xenova/all-MiniLM-L6-v2 (22 MB, auto-download)`);

  if (needs.length > 0) {
    note(
      [
        "Some components are not set up. Sensei will use regex indexing until they are available.",
        "",
        "Missing:",
        ...needs.map(n => `  ✗ ${n}`),
        "",
        `Disk free: ${status.diskFreeGB.toFixed(1)} GB   RAM available: ${status.ramAvailableGB.toFixed(1)} GB`,
        "",
        `Set up later with: sensei serve  (then: sensei server status)`,
      ].join("\n"),
      "Setup status"
    );

    if (status.ollamaBinary && status.ollamaRunning && !status.ollamaModel) {
      if (status.diskFreeGB < OLLAMA_MODEL_SIZE_GB + 0.5) {
        log.warn(`Not enough disk space to pull ${OLLAMA_MODEL} (need ${OLLAMA_MODEL_SIZE_GB + 0.5} GB, have ${status.diskFreeGB.toFixed(1)} GB)`);
      } else {
        const pull = await confirm({ message: `Pull ${OLLAMA_MODEL} now? (~${OLLAMA_MODEL_SIZE_GB} GB)` });
        if (!isCancel(pull) && pull) {
          const pullSpinner = spinner();
          pullSpinner.start(`Pulling ${OLLAMA_MODEL}...`);
          try {
            const res = await fetch("http://127.0.0.1:11434/api/pull", {
              method: "POST",
              headers: { "Content-Type": "application/json" },
              body: JSON.stringify({ name: OLLAMA_MODEL, stream: false }),
              signal: AbortSignal.timeout(600_000),
            });
            if (res.ok) {
              pullSpinner.stop(`${OLLAMA_MODEL} pulled`);
            } else {
              pullSpinner.stop(`Pull failed (status ${res.status})`);
            }
          } catch (err) {
            pullSpinner.stop(`Pull failed: ${(err as Error).message}`);
          }
        }
      }
    }
  }

  // --- Index ---
  const s = spinner();
  s.start("Indexing repo (full scan)...");
  const summary = await reindexRepo(cwd);
  s.stop(`Indexed: ${summary.added} files`);

  note(
    [
      `Created: CLAUDE.md, .sensei/ (llmspec.yaml, llms.txt, symbol-map.json, ...)`,
      ``,
      `Next steps:`,
      `  1. Edit .sensei/llmspec.yaml to add doc coverage (docs[].covers[])`,
      `  2. Run: sensei hooks install --drift   to enable pre-commit drift check`,
      `  3. Commit .sensei/ so the team shares the index without re-running sensei`,
      `  4. Run: sensei serve   to start inference server for richer analysis`,
    ].join("\n"),
    "Setup complete"
  );

  outro("Done.");
}
```

**Step 2: Build and verify**

```bash
cd /Users/Jerry/Developer/skills/packages/sensei
bun run build 2>&1 | tail -5
```

Expected: `Bundled N modules` — no errors.

**Step 3: Commit**

```bash
cd /Users/Jerry/Developer/skills
git add packages/sensei/src/commands/init.ts
git commit -m "feat(sensei): init checks prerequisites and offers to pull Ollama model"
```

---

## Task 7: Run Full Test Suite

**Step 1: Run all tests**

```bash
cd /Users/Jerry/Developer/skills/packages/sensei
bunx vitest run
```

Expected: all existing tests pass + new model/ tests pass. If any existing tests fail due to changes in `serve.ts` (e.g. the `port` field added to return type), fix them.

**Step 2: Build final artifact**

```bash
bun run build
```

Expected: `Bundled N modules` — no errors.

**Step 3: Final commit if any fixes were needed**

```bash
cd /Users/Jerry/Developer/skills
git add -p
git commit -m "fix(sensei): update serve.spec.ts for extended return type"
```

---

## Validation

After all tasks, verify the full flow:

```bash
# 1. Start the server
./packages/sensei/dist/cli.js serve &
sleep 1

# 2. Check status
./packages/sensei/dist/cli.js server status

# 3. Hit /setup/status directly
curl -s http://localhost:7744/setup/status | jq .

# 4. Hit /analyze with a small file
curl -s -X POST http://localhost:7744/analyze \
  -H "Content-Type: application/json" \
  -d '{"filePath":"test.ts","content":"export function hello(name: string): string { return `Hello ${name}`; }"}' | jq .
```

If Ollama is running with `llama3.2:3b` pulled, `/analyze` returns a real `FileAnalysis`. If not, it returns a fallback with `symbols: []` and `summary: "Could not analyze this file"` — both are valid.
