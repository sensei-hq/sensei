import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { OllamaBackend, extractJson, makeFallbackAnalysis } from "./ollama-backend.js";

describe("OllamaBackend", () => {
  const backend = new OllamaBackend({ model: "llama3.2:3b", baseUrl: "http://127.0.0.1:11434" });

  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  let fetchMock: any;
  let originalFetch: typeof globalThis.fetch;
  beforeEach(() => {
    originalFetch = globalThis.fetch;
    fetchMock = vi.fn();
    globalThis.fetch = fetchMock;
  });
  afterEach(() => { globalThis.fetch = originalFetch; });

  describe("isAvailable", () => {
    it("returns true when Ollama responds", async () => {
      fetchMock.mockResolvedValueOnce(new Response(JSON.stringify({ models: [] }), { status: 200 }));
      expect(await backend.isAvailable()).toBe(true);
    });

    it("returns false when fetch throws", async () => {
      fetchMock.mockRejectedValueOnce(new Error("ECONNREFUSED"));
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
      fetchMock.mockResolvedValueOnce(new Response(
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
      fetchMock.mockResolvedValueOnce(new Response(
        JSON.stringify({ response: `Here is the analysis:\n${JSON.stringify(analysis)}\nLet me know if you need more.` }),
        { status: 200 }
      ));
      const result = await backend.extract("export const bar = 1;", { filePath: "src/bar.ts" });
      expect(result.summary).toBe("Bar module");
    });

    it("returns fallback FileAnalysis when JSON parse fails", async () => {
      fetchMock.mockResolvedValueOnce(new Response(
        JSON.stringify({ response: "I cannot analyze this file." }),
        { status: 200 }
      ));
      const result = await backend.extract("???", { filePath: "src/bad.ts" });
      expect(result.path).toBe("src/bad.ts");
      expect(result.symbols).toEqual([]);
      expect(result.summary).toMatch(/could not/i);
    });
  });

  describe("generate", () => {
    it("returns the response string from Ollama", async () => {
      fetchMock.mockResolvedValueOnce(new Response(
        JSON.stringify({ response: '["packages/tools/src/tools/reindex.ts"]' }),
        { status: 200 }
      ));
      const result = await backend.generate("which files does this cover?");
      expect(result).toBe('["packages/tools/src/tools/reindex.ts"]');
    });

    it("returns empty string when fetch throws", async () => {
      fetchMock.mockRejectedValueOnce(new Error("ECONNREFUSED"));
      const result = await backend.generate("some prompt");
      expect(result).toBe("");
    });
  });

  describe("embed", () => {
    it("returns number array from Ollama embeddings endpoint", async () => {
      fetchMock.mockResolvedValueOnce(new Response(
        JSON.stringify({ embedding: [0.1, 0.2, 0.3] }),
        { status: 200 }
      ));
      const result = await backend.embed("some text");
      expect(result).toEqual([0.1, 0.2, 0.3]);
    });

    it("returns empty array when fetch throws", async () => {
      fetchMock.mockRejectedValueOnce(new Error("ECONNREFUSED"));
      const result = await backend.embed("some text");
      expect(result).toEqual([]);
    });

    it("returns empty array on HTTP error response", async () => {
      fetchMock.mockResolvedValueOnce(new Response(
        JSON.stringify({ error: "model not found" }), { status: 500 }
      ));
      const result = await backend.embed("some text");
      expect(result).toEqual([]);
    });

    it("embed() uses embeddingModel when set separately from generation model", async () => {
      const fetches: Array<{ model: string }> = [];
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      vi.stubGlobal("fetch", async (_url: string, opts: any) => {
        const body = JSON.parse(opts.body);
        fetches.push({ model: body.model });
        return { ok: true, json: async () => ({ embedding: [0.1, 0.2] }) } as any;
      });

      const backendWithEmbedding = new OllamaBackend({ model: "llama3.2:3b", embeddingModel: "nomic-embed-text" });
      await backendWithEmbedding.embed("hello");

      expect(fetches[0].model).toBe("nomic-embed-text");
    });
  });
});

describe("extractJson", () => {
  it("extracts JSON from clean string", () => {
    expect(extractJson('{"a":1}')).toEqual({ a: 1 });
  });
  it("extracts JSON wrapped in prose", () => {
    expect(extractJson('Here it is: {"a":1} done')).toEqual({ a: 1 });
  });
  it("returns null for non-JSON string", () => {
    expect(extractJson("no json here")).toBeNull();
  });
});
