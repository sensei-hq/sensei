// packages/server/src/tools/get-lib-docs.spec.ts
import { describe, it, expect, vi } from "vitest";
import { getLibDocsTool } from "./get-lib-docs.js";
import type { ModelBackend } from "@sensei/shared";

const makeMockBackend = (): ModelBackend => ({
  name: "mock", init: vi.fn(), isAvailable: vi.fn().mockResolvedValue(true),
  generate: vi.fn().mockResolvedValue(""), embed: vi.fn().mockResolvedValue([0.1, 0.2, 0.3]),
  extract: vi.fn().mockResolvedValue({}),
});

const ROW = { title: "Button", url: "https://rokkit.dev/button", local_path: null, description: "A button", content: null, source_type: "llms.txt", component: "Forms" };

describe("getLibDocsTool", () => {
  it("embeds query and calls match_lib_doc_sections RPC", async () => {
    const db = {
      rpc: vi.fn().mockResolvedValue({ data: [ROW], error: null }),
      from: vi.fn(),
    };
    const backend = makeMockBackend();

    const result = await getLibDocsTool(db as any, backend, "repo-1", "rokkit", { query: "button" });

    expect(backend.embed).toHaveBeenCalledWith("button");
    expect(db.rpc).toHaveBeenCalledWith("match_lib_doc_sections", expect.objectContaining({
      p_repo_id: "repo-1", p_lib_name: "rokkit",
    }));
    expect(result.lib).toBe("rokkit");
    expect(result.sections).toHaveLength(1);
    expect(result.sections[0].title).toBe("Button");
  });

  it("returns all sections sorted by title when no query provided", async () => {
    const rows = [{ ...ROW, title: "Select" }, { ...ROW, title: "Button" }];
    const orderMock = vi.fn().mockResolvedValue({ data: rows, error: null });
    const db = {
      rpc: vi.fn(),
      from: vi.fn().mockReturnValue({
        select: vi.fn().mockReturnValue({
          eq: vi.fn().mockReturnValue({ eq: vi.fn().mockReturnValue({ order: orderMock }) }),
        }),
      }),
    };
    const result = await getLibDocsTool(db as any, makeMockBackend(), "repo-1", "rokkit");
    expect(result.sections).toHaveLength(2);
  });

  it("returns empty sections on any error — never throws", async () => {
    const db = { rpc: vi.fn().mockResolvedValue({ data: null, error: { message: "DB error" } }), from: vi.fn() };
    const result = await getLibDocsTool(db as any, makeMockBackend(), "repo-1", "rokkit", { query: "test" });
    expect(result.sections).toEqual([]);
  });
});
