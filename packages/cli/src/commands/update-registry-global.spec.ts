// packages/cli/src/commands/update-registry-global.spec.ts
import { describe, it, expect, vi, beforeEach } from "vitest";
import { runUpdateRegistryCore } from "./update-registry.js";
import { loadSenseiConfig, makeSenseiClient } from "@sensei/shared";

// Shared mock state for LibIndexer — tests override these as needed
let mockIndex = vi.fn();
let mockIndexShared = vi.fn().mockResolvedValue({ sectionsIndexed: 3 });

vi.mock("@sensei/shared", async () => {
  const actual = await vi.importActual<typeof import("@sensei/shared")>("@sensei/shared");
  return { ...actual, loadSenseiConfig: vi.fn(), makeSenseiClient: vi.fn() };
});
vi.mock("@sensei/engine", async () => {
  const actual = await vi.importActual<typeof import("@sensei/engine")>("@sensei/engine");
  return {
    ...actual,
    LibIndexer: class {
      index(...args: unknown[]) { return mockIndex(...args); }
      indexShared(...args: unknown[]) { return mockIndexShared(...args); }
    },
    extractProjectProfile: vi.fn().mockResolvedValue({
      repoName: "test-repo",
      dominantLanguage: "typescript",
      keySymbols: [],
      dependencies: [],
    }),
    LlmsTxtAdapter: class {
      fetch() { return Promise.resolve([{ url: "https://rokkit.dev/", content: "# Rokkit docs" }]); }
    },
    HttpAdapter: class {
      fetch() { return Promise.resolve([]); }
    },
    LocalAdapter: class {
      fetch() { return Promise.resolve([]); }
    },
  };
});

const MOCK_CONFIG = {
  repo_id: "repo-abc",
  custom_libs: [{ name: "rokkit", source_type: "llms.txt", base_url: "https://rokkit.dev/llms.txt" }],
};

function makeDb() {
  const sharedLibId = "shared-lib-uuid";
  const upsertedRows: unknown[] = [];
  const fromMocks: Record<string, unknown> = {};

  const sharedLibsMock = {
    upsert: vi.fn().mockReturnValue({
      select: vi.fn().mockReturnValue({
        single: vi.fn().mockResolvedValue({ data: { id: sharedLibId }, error: null }),
      }),
    }),
    update: vi.fn().mockReturnValue({
      eq: vi.fn().mockResolvedValue({ error: null }),
    }),
  };

  const repoLibsMock = {
    upsert: vi.fn().mockResolvedValue({ error: null }),
  };

  fromMocks["shared_libs"] = sharedLibsMock;
  fromMocks["repo_libs"] = repoLibsMock;

  const schema = {
    from: vi.fn().mockImplementation((table: string) => fromMocks[table] ?? {}),
  };

  return { schema: vi.fn().mockReturnValue(schema), _upsertedRows: upsertedRows, _sharedLibId: sharedLibId, _fromMocks: fromMocks };
}

describe("runUpdateRegistryCore --global", () => {
  beforeEach(() => {
    mockIndex = vi.fn();
    mockIndexShared = vi.fn().mockResolvedValue({ sectionsIndexed: 3 });
    vi.mocked(loadSenseiConfig).mockResolvedValue(MOCK_CONFIG as any);
    vi.mocked(makeSenseiClient).mockResolvedValue({} as any);
  });

  it("calls indexShared (not index) when opts.global is true", async () => {
    const db = makeDb();
    vi.mocked(makeSenseiClient).mockResolvedValue(db as any);

    await runUpdateRegistryCore("/repo", "rokkit", { global: true });

    expect(mockIndexShared).toHaveBeenCalledTimes(1);
    expect(mockIndex).not.toHaveBeenCalled();
  });

  it("upserts shared_libs catalog and updates section_count + indexed_at", async () => {
    const db = makeDb();
    vi.mocked(makeSenseiClient).mockResolvedValue(db as any);

    await runUpdateRegistryCore("/repo", "rokkit", { global: true });

    const sharedLibsMock = (db._fromMocks["shared_libs"] as any);
    expect(sharedLibsMock.upsert).toHaveBeenCalledWith(
      expect.objectContaining({ name: "rokkit" }),
      { onConflict: "name" }
    );
    expect(sharedLibsMock.update).toHaveBeenCalledWith(
      expect.objectContaining({ section_count: 3 })
    );
  });

  it("upserts repo_libs with shared_lib_id set", async () => {
    const db = makeDb();
    vi.mocked(makeSenseiClient).mockResolvedValue(db as any);

    await runUpdateRegistryCore("/repo", "rokkit", { global: true });

    const repoLibsMock = (db._fromMocks["repo_libs"] as any);
    expect(repoLibsMock.upsert).toHaveBeenCalledWith(
      expect.objectContaining({ shared_lib_id: db._sharedLibId }),
      { onConflict: "repo_id,name" }
    );
  });

  it("running --global twice replaces sections (indexShared called each time)", async () => {
    const db = makeDb();
    vi.mocked(makeSenseiClient).mockResolvedValue(db as any);

    await runUpdateRegistryCore("/repo", "rokkit", { global: true });
    await runUpdateRegistryCore("/repo", "rokkit", { global: true });

    expect(mockIndexShared).toHaveBeenCalledTimes(2);
  });

  it("exits non-zero when config.yaml is missing and --global is set", async () => {
    vi.mocked(loadSenseiConfig).mockResolvedValue(null);
    const exitSpy = vi.spyOn(process, "exit").mockImplementation(() => { throw new Error("exit"); });

    await expect(runUpdateRegistryCore("/repo", "rokkit", { global: true })).rejects.toThrow("exit");
    expect(exitSpy).toHaveBeenCalledWith(1);
    exitSpy.mockRestore();
  });
});
