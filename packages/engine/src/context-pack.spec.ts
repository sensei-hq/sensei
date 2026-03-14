import { describe, it, expect, vi } from "vitest";
import { mkdtemp, writeFile, rm, mkdir } from "fs/promises";
import { join } from "path";
import { tmpdir } from "os";
import { buildContextPack } from "./context-pack.js";
import type { ModelBackend } from "@sensei/shared";

function makeBackend(): ModelBackend {
  return {
    name: "mock",
    init: vi.fn(),
    isAvailable: vi.fn().mockResolvedValue(false),
    embed: vi.fn().mockResolvedValue([]),  // Ollama unavailable
    generate: vi.fn().mockResolvedValue(""),
    extract: vi.fn().mockResolvedValue({ path: "", language: "", contentHash: "", analyzedAt: "", summary: "", symbols: [] }),
  };
}

function makeDb() {
  const upsert = vi.fn().mockResolvedValue({ error: null });
  return {
    upsert,
    rpc: vi.fn().mockResolvedValue({ data: [], error: null }),
    from: vi.fn((table: string) => {
      if (table === "context_packs") return { upsert };
      return {
        select: vi.fn(() => ({
          eq: vi.fn(() => ({
            gt: vi.fn().mockResolvedValue({ data: [], error: null }),
            eq: vi.fn().mockResolvedValue({ data: [], error: null }),
          })),
        })),
        upsert,
      };
    }),
  } as any;
}

describe("buildContextPack", () => {
  it("returns a ContextPack with required fields", async () => {
    const dir = await mkdtemp(join(tmpdir(), "ctx-pack-"));
    const pack = await buildContextPack(makeDb(), "repo-1", dir, "fix auth", { backend: makeBackend() });

    expect(pack.id).toBeTruthy();
    expect(pack.task).toBe("fix auth");
    expect(pack.createdAt).toBeTruthy();
    expect(Array.isArray(pack.slices)).toBe(true);
    expect(pack.totalTokens).toBeGreaterThanOrEqual(0);

    await rm(dir, { recursive: true, force: true });
  });

  it("persists the pack to context_packs table", async () => {
    const db = makeDb();
    const dir = await mkdtemp(join(tmpdir(), "ctx-pack-persist-"));

    await buildContextPack(db, "repo-1", dir, "fix auth", { backend: makeBackend() });

    const contextPacksCalls = (db.from as ReturnType<typeof vi.fn>).mock.calls
      .filter((c: any[]) => c[0] === "context_packs");
    expect(contextPacksCalls.length).toBeGreaterThan(0);
    // Verify upsert was called with correct fields
    expect(db.upsert).toHaveBeenCalledWith(
      expect.objectContaining({ repo_id: "repo-1", task: "fix auth" })
    );

    await rm(dir, { recursive: true, force: true });
  });
});
