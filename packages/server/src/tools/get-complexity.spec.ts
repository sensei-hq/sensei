import { describe, it, expect, vi } from "vitest";

vi.mock("@sensei/graph-indexer", () => ({
  getOrCreateDb: vi.fn(),
}));
vi.mock("@sensei/shared", () => ({
  loadSenseiConfig: vi.fn().mockResolvedValue({ repo_id: "proj" }),
  lookupRepoId: vi.fn().mockResolvedValue("proj"),
}));

import { getComplexity } from "./get-complexity.js";
import { getOrCreateDb } from "@sensei/graph-indexer";

const mockGetOrCreateDb = getOrCreateDb as ReturnType<typeof vi.fn>;

function makeConn(rows: Record<string, unknown>[]) {
  return {
    query: vi.fn().mockResolvedValue({
      getAll: vi.fn().mockResolvedValue(rows),
    }),
    close: vi.fn().mockResolvedValue(undefined),
  };
}

describe("getComplexity", () => {
  it("returns hotspots sorted by complexity", async () => {
    const hotRows = [
      { name: "complexFn", file: "/repo/src/a.ts", line: 10, complexity: 15 },
      { name: "simpleFn", file: "/repo/src/b.ts", line: 5, complexity: 2 },
    ];
    const conn = {
      query: vi.fn()
        .mockResolvedValueOnce({ getAll: vi.fn().mockResolvedValue(hotRows) })
        .mockResolvedValueOnce({ getAll: vi.fn().mockResolvedValue([]) })
        .mockResolvedValueOnce({ getAll: vi.fn().mockResolvedValue([{ total: 2, avg: 8.5, high: 1 }]) }),
      close: vi.fn().mockResolvedValue(undefined),
    };
    mockGetOrCreateDb.mockResolvedValue({ db: { close: vi.fn() }, conn });

    const result = await getComplexity("repo-1", "/repo", {});
    expect(result.hotspots[0].name).toBe("complexFn");
    expect(result.hotspots[0].complexity).toBe(15);
    expect(result.hotspots[0].file).toBe("src/a.ts"); // repo-relative
    expect(result.summary.highComplexityCount).toBe(1);
  });
});
