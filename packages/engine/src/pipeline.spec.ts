import { describe, it, expect, vi } from "vitest";
import { mkdtemp, writeFile, rm, mkdir } from "fs/promises";
import { join } from "path";
import { tmpdir } from "os";
import { indexRepo } from "./pipeline.js";
import type { ModelBackend } from "@sensei/shared";

describe("indexRepo pipeline", () => {
  it("returns IndexResult with repoId", async () => {
    // This test uses the real Scanner and TypeScriptAdapter but a mock Supabase client.
    // Point it at a directory that contains some .ts files.
    const eqChain: any = {
      eq: vi.fn(() => eqChain),
      in: vi.fn().mockResolvedValue({ data: [], error: null }),
      then: (resolve: any) => Promise.resolve({ data: [], error: null }).then(resolve),
    };
    const mockClient = {
      from: vi.fn(() => ({
        upsert: vi.fn().mockResolvedValue({ error: null }),
        delete: vi.fn(() => ({ eq: vi.fn(() => ({ in: vi.fn().mockResolvedValue({ error: null }) })) })),
        select: vi.fn(() => eqChain),
      })),
    };

    const result = await indexRepo({
      repoPath: process.cwd(), // packages/engine itself has .ts files
      repoId: "test-pipeline",
      client: mockClient as any,
    });

    expect(result.repoId).toBe("test-pipeline");
    expect(result.symbolsUpserted).toBeGreaterThan(0);
    expect(result.filesIndexed).toBeGreaterThan(0);
    expect(result.errors).toHaveLength(0);
  });
});

function makePipelineClient() {
  const upsert = vi.fn().mockResolvedValue({ error: null });
  return {
    from: vi.fn(() => ({
      select: vi.fn(() => ({
        eq: vi.fn(() => ({
          gt: vi.fn().mockResolvedValue({ data: [], error: null }),
        })),
      })),
      upsert,
      delete: vi.fn(() => ({ eq: vi.fn(() => ({ in: vi.fn().mockResolvedValue({ error: null }) })) })),
    })),
    _upsert: upsert,
  } as any;
}

function makeMockBackend(): ModelBackend {
  return {
    name: "mock",
    init: vi.fn(),
    isAvailable: vi.fn().mockResolvedValue(true),
    embed: vi.fn().mockResolvedValue(new Array(768).fill(0.1)),
    generate: vi.fn().mockResolvedValue(""),
    extract: vi.fn().mockResolvedValue({ path: "", language: "", contentHash: "", analyzedAt: "", summary: "", symbols: [] }),
  };
}

describe("indexRepo with backend", () => {
  it("calls backend.embed for each indexed TS file when backend provided", async () => {
    const dir = await mkdtemp(join(tmpdir(), "pipeline-embed-"));
    await mkdir(join(dir, "src"), { recursive: true });
    await writeFile(join(dir, "src/foo.ts"), "export function foo() { return 1; }");

    const backend = makeMockBackend();
    await indexRepo({ repoPath: dir, repoId: "repo-1", client: makePipelineClient(), backend });

    expect(backend.embed).toHaveBeenCalledWith(expect.stringContaining("foo"));
    await rm(dir, { recursive: true, force: true });
  });

  it("does not throw when no backend provided", async () => {
    const dir = await mkdtemp(join(tmpdir(), "pipeline-no-embed-"));
    await mkdir(join(dir, "src"), { recursive: true });
    await writeFile(join(dir, "src/foo.ts"), "export function foo() {}");

    const result = await indexRepo({ repoPath: dir, repoId: "repo-1", client: makePipelineClient() });
    expect(result.errors).toHaveLength(0);
    await rm(dir, { recursive: true, force: true });
  });
});
