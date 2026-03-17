import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { mkdirSync, writeFileSync, rmSync, existsSync } from "fs";
import { join } from "path";
import { extractChunks, buildChunksAndEmbeddings } from "./chunker.js";
import { senseiPath } from "@sensei/shared";
import type { SymbolMap } from "@sensei/shared";

// vi.hoisted() ensures mockEmbed is initialized before the vi.mock factory executes
// (Vitest hoists vi.mock calls above all imports; any variable used in the factory
//  must also be hoisted or it will be undefined when the factory runs)
const mockEmbed = vi.hoisted(() => vi.fn().mockResolvedValue(new Array(384).fill(0.1)));
vi.mock("./embedder.js", () => ({
  embed: mockEmbed,
  isAvailable: vi.fn().mockResolvedValue(true),
  ensureReady: vi.fn().mockResolvedValue(undefined),
}));

const TMP = "/tmp/sensei-test-chunker";

beforeEach(() => {
  mkdirSync(join(TMP, ".sensei"), { recursive: true });
  mkdirSync(join(TMP, "src"), { recursive: true });
  mkdirSync(join(TMP, "docs"), { recursive: true });
  mockEmbed.mockClear();
});
afterEach(() => rmSync(TMP, { recursive: true, force: true }));

describe("extractChunks", () => {
  it("produces one chunk per symbol from symbol map", async () => {
    const symbolMap: SymbolMap = {
      "src/auth.ts": {
        L0: ["login(email: string, password: string): Promise<User | null>"],
        L1: ["// Authenticate user and return session token or null on failure\n// login(email: string, password: string): Promise<User | null>"],
        L2: [],
      }
    };
    const chunks = await extractChunks(symbolMap, [], TMP);
    const id = "src/auth.ts:login";
    expect(chunks[id]).toBeDefined();
    expect(chunks[id].file).toBe("src/auth.ts");
    expect(chunks[id].type).toBe("symbol");
    expect(chunks[id].text).toContain("login");
  });

  it("produces one chunk per H2/H3 section for markdown", async () => {
    writeFileSync(join(TMP, "docs/design.md"), [
      "# Design",
      "",
      "## Symbol Map",
      "",
      "Extracted exports stored at L0 and L1 per file.",
      "",
      "### Storage Format",
      "",
      "Stored as JSON.",
    ].join("\n"));
    const chunks = await extractChunks({}, ["docs/design.md"], TMP);
    expect(chunks["docs/design.md#symbol-map"]).toBeDefined();
    expect(chunks["docs/design.md#storage-format"]).toBeDefined();
    expect(chunks["docs/design.md#symbol-map"].type).toBe("doc");
    expect(chunks["docs/design.md#symbol-map"].text).toContain("Symbol Map");
  });

  it("returns empty object for empty file with no symbols", async () => {
    const symbolMap: SymbolMap = { "src/empty.ts": { L0: [], L1: [], L2: [] } };
    const chunks = await extractChunks(symbolMap, [], TMP);
    const symbolChunks = Object.values(chunks).filter(c => c.file === "src/empty.ts");
    expect(symbolChunks).toHaveLength(0);
  });
});

function makeStatefulMockClient() {
  const store: Record<string, { id: string; content_hash: string; tf: Record<string, number>; embedding: number[] | null }> = {};
  return {
    schema: () => ({
      from: (table: string) => {
        if (table !== "chunks") return { select: () => ({ eq: () => ({ then: (r: (v: { data: unknown[] }) => void) => r({ data: [] }) }) }) };
        return {
          select: (_cols: string) => ({
            eq: (_col: string, _val: string) => ({
              then: (resolve: (v: { data: unknown[] }) => void) => resolve({ data: Object.values(store) }),
            }),
          }),
          upsert: (rows: Array<{ id: string; content_hash: string; tf: Record<string, number>; embedding: string | number[] | null }>, _opts: unknown) => {
            for (const row of rows) {
              // Supabase returns embeddings as number[], not as the "[x,y,z]" string we write
              const embedding = typeof row.embedding === "string" ? JSON.parse(row.embedding) : row.embedding;
              store[row.id] = { ...row, embedding };
            }
            return { then: (r: (v: { error: null }) => void) => r({ error: null }) };
          },
          delete: () => ({ eq: () => ({ in: () => ({ then: (r: (v: { error: null }) => void) => r({ error: null }) }) }) }),
        };
      },
    }),
  };
}

describe("buildChunksAndEmbeddings", () => {
  it("does nothing without a DB client (no local file fallback)", async () => {
    const symbolMap: SymbolMap = {
      "src/auth.ts": {
        L0: ["login(email: string): Promise<User>"],
        L1: ["// login(email: string): Promise<User>"],
        L2: [],
      }
    };
    await buildChunksAndEmbeddings(TMP, symbolMap, [], {});
    expect(mockEmbed.mock.calls.length).toBe(0);
    expect(existsSync(senseiPath(TMP, "chunks.json"))).toBe(false);
    expect(existsSync(senseiPath(TMP, "embeddings.json"))).toBe(false);
  });

  it("upserts chunks to DB when client and repoId provided", async () => {
    const mockClient = makeStatefulMockClient();
    const symbolMap: SymbolMap = {
      "src/auth.ts": {
        L0: ["login(email: string): Promise<User>"],
        L1: ["// login(email: string): Promise<User>"],
        L2: [],
      }
    };
    await buildChunksAndEmbeddings(TMP, symbolMap, [], { client: mockClient, repoId: "test-repo" });
    expect(mockEmbed.mock.calls.length).toBeGreaterThan(0);
  });

  it("does not call embed() for unchanged chunks when DB returns matching contentHash", async () => {
    const mockClient = makeStatefulMockClient();
    const symbolMap: SymbolMap = {
      "src/auth.ts": {
        L0: ["login(email: string): Promise<User>"],
        L1: ["// login(email: string): Promise<User>"],
        L2: [],
      }
    };
    // First run — should call embed() for the new chunk
    await buildChunksAndEmbeddings(TMP, symbolMap, [], { client: mockClient, repoId: "test-repo" });
    expect(mockEmbed.mock.calls.length).toBeGreaterThan(0);

    // Second run with same symbolMap — contentHash unchanged, must NOT re-embed
    mockEmbed.mockClear();
    await buildChunksAndEmbeddings(TMP, symbolMap, [], { client: mockClient, repoId: "test-repo" });
    expect(mockEmbed.mock.calls.length).toBe(0);
  });
});
