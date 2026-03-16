// packages/server/src/tools/get-lib-docs-routing.spec.ts
import { describe, it, expect, vi } from "vitest";
import { getLibDocsTool } from "./get-lib-docs.js";
import type { ModelBackend } from "@sensei/shared";

const makeMockBackend = (): ModelBackend => ({
  name: "mock", init: vi.fn(), isAvailable: vi.fn().mockResolvedValue(true),
  generate: vi.fn().mockResolvedValue(""), embed: vi.fn().mockResolvedValue([0.1, 0.2, 0.3]),
  extract: vi.fn().mockResolvedValue({}),
});

const SHARED_ROW = { title: "Button", url: "https://rokkit.dev/button", local_path: null, description: "A button", content: null, source_type: "llms.txt", component: "Forms" };

/** Build a mock DB that returns a repo_libs row with the given shared_lib_id. */
function makeDbWithSharedLib(sharedLibId: string | null) {
  const repoLibsChain = {
    select: vi.fn().mockReturnValue({
      eq: vi.fn().mockReturnValue({
        eq: vi.fn().mockReturnValue({
          maybeSingle: vi.fn().mockResolvedValue({
            data: { shared_lib_id: sharedLibId },
            error: null,
          }),
        }),
      }),
    }),
  };

  const sharedRpcResult = { data: [SHARED_ROW], error: null };
  const perRepoRpcResult = { data: [], error: null };

  return {
    schema: vi.fn().mockReturnValue({
      from: vi.fn().mockReturnValue(repoLibsChain),
    }),
    rpc: vi.fn().mockImplementation((name: string) => {
      if (name === "match_shared_lib_sections") return Promise.resolve(sharedRpcResult);
      return Promise.resolve(perRepoRpcResult);
    }),
    from: vi.fn().mockReturnValue({
      select: vi.fn().mockReturnValue({
        eq: vi.fn().mockReturnValue({
          eq: vi.fn().mockReturnValue({
            order: vi.fn().mockResolvedValue({ data: [], error: null }),
          }),
        }),
      }),
    }),
  };
}

describe("getLibDocsTool routing", () => {
  it("calls match_shared_lib_sections RPC when shared_lib_id is set (query path)", async () => {
    const db = makeDbWithSharedLib("shared-uuid-1");
    const result = await getLibDocsTool(db as any, makeMockBackend(), "repo-1", "rokkit", { query: "button" });

    expect(db.rpc).toHaveBeenCalledWith("match_shared_lib_sections", expect.objectContaining({
      p_shared_lib_id: "shared-uuid-1",
    }));
    expect(db.rpc).not.toHaveBeenCalledWith("match_lib_doc_sections", expect.anything());
    expect(result.sections).toHaveLength(1);
    expect(result.sections[0].title).toBe("Button");
  });

  it("calls match_lib_doc_sections RPC when shared_lib_id is null (query path)", async () => {
    const db = makeDbWithSharedLib(null);
    await getLibDocsTool(db as any, makeMockBackend(), "repo-1", "rokkit", { query: "button" });

    expect(db.rpc).toHaveBeenCalledWith("match_lib_doc_sections", expect.objectContaining({
      p_repo_id: "repo-1",
    }));
    expect(db.rpc).not.toHaveBeenCalledWith("match_shared_lib_sections", expect.anything());
  });

  it("queries shared_lib_sections directly when shared_lib_id is set (browse path, no query)", async () => {
    const sharedBrowseMock = vi.fn().mockResolvedValue({ data: [SHARED_ROW], error: null });
    const db = {
      schema: vi.fn().mockReturnValue({
        from: vi.fn().mockImplementation((table: string) => {
          if (table === "repo_libs") {
            return {
              select: vi.fn().mockReturnValue({
                eq: vi.fn().mockReturnValue({
                  eq: vi.fn().mockReturnValue({
                    maybeSingle: vi.fn().mockResolvedValue({ data: { shared_lib_id: "shared-uuid-1" }, error: null }),
                  }),
                }),
              }),
            };
          }
          // shared_lib_sections browse query
          return {
            select: vi.fn().mockReturnValue({
              eq: vi.fn().mockReturnValue({
                order: sharedBrowseMock,
              }),
            }),
          };
        }),
      }),
      rpc: vi.fn(),
      from: vi.fn(),
    };

    const result = await getLibDocsTool(db as any, makeMockBackend(), "repo-1", "rokkit");
    expect(sharedBrowseMock).toHaveBeenCalled();
    expect(db.rpc).not.toHaveBeenCalled();
    expect(result.sections).toHaveLength(1);
  });

  it("returns empty sections when repo_libs row is missing — no error", async () => {
    const db = {
      schema: vi.fn().mockReturnValue({
        from: vi.fn().mockReturnValue({
          select: vi.fn().mockReturnValue({
            eq: vi.fn().mockReturnValue({
              eq: vi.fn().mockReturnValue({
                maybeSingle: vi.fn().mockResolvedValue({ data: null, error: null }),
              }),
            }),
          }),
        }),
      }),
      rpc: vi.fn().mockResolvedValue({ data: [], error: null }),
      from: vi.fn().mockReturnValue({
        select: vi.fn().mockReturnValue({
          eq: vi.fn().mockReturnValue({
            eq: vi.fn().mockReturnValue({
              order: vi.fn().mockResolvedValue({ data: [], error: null }),
            }),
          }),
        }),
      }),
    };

    const result = await getLibDocsTool(db as any, makeMockBackend(), "repo-1", "rokkit", { query: "test" });
    expect(result.sections).toEqual([]);
  });
});
