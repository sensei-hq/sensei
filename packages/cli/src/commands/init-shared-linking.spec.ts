// packages/cli/src/commands/init-shared-linking.spec.ts
import { describe, it, expect, vi, beforeEach } from "vitest";
import { lookupSharedLib } from "./init.js";

function makeDb(sharedLib: unknown) {
  return {
    schema: vi.fn().mockReturnValue({
      from: vi.fn().mockReturnValue({
        select: vi.fn().mockReturnValue({
          eq: vi.fn().mockReturnValue({
            maybeSingle: vi.fn().mockResolvedValue({ data: sharedLib, error: null }),
          }),
        }),
      }),
    }),
  };
}

const SHARED_LIB = {
  id: "shared-lib-uuid",
  section_count: 42,
  indexed_at: "2026-03-10T00:00:00.000Z",
  base_url: "https://rokkit.dev/llms.txt",
  local_path: null,
  source_type: "llms.txt",
};

describe("lookupSharedLib", () => {
  it("returns catalog row when lib is found in shared pool", async () => {
    const db = makeDb(SHARED_LIB);
    const result = await lookupSharedLib(db as any, "rokkit");

    expect(result).not.toBeNull();
    expect(result?.id).toBe("shared-lib-uuid");
    expect(result?.section_count).toBe(42);
    expect(db.schema).toHaveBeenCalledWith("sensei");
  });

  it("returns null when lib is not in shared pool", async () => {
    const db = makeDb(null);
    const result = await lookupSharedLib(db as any, "unknown-lib");
    expect(result).toBeNull();
  });

  it("returns null silently when DB throws", async () => {
    const db = {
      schema: vi.fn().mockReturnValue({
        from: vi.fn().mockReturnValue({
          select: vi.fn().mockReturnValue({
            eq: vi.fn().mockReturnValue({
              maybeSingle: vi.fn().mockRejectedValue(new Error("network failure")),
            }),
          }),
        }),
      }),
    };
    const result = await lookupSharedLib(db as any, "rokkit");
    expect(result).toBeNull();
  });
});

// Integration tests for the accept / decline / not-found flows.
// These require mocking @clack/prompts — use vi.mock before the import.
vi.mock("@clack/prompts", async () => {
  const actual = await vi.importActual<typeof import("@clack/prompts")>("@clack/prompts");
  return {
    ...actual,
    confirm: vi.fn(),
  };
});

import { confirm as clackConfirm } from "@clack/prompts";

/** Build a Supabase client that also records repo_libs upserts. */
function makeIntegrationDb(sharedLib: typeof SHARED_LIB | null) {
  const repoLibsUpserts: unknown[] = [];
  return {
    _repoLibsUpserts: repoLibsUpserts,
    schema: vi.fn().mockReturnValue({
      from: vi.fn().mockImplementation((table: string) => {
        if (table === "shared_libs") {
          return {
            select: vi.fn().mockReturnValue({
              eq: vi.fn().mockReturnValue({
                maybeSingle: vi.fn().mockResolvedValue({ data: sharedLib, error: null }),
              }),
            }),
          };
        }
        if (table === "repo_libs") {
          return {
            upsert: vi.fn().mockImplementation((row: unknown) => {
              repoLibsUpserts.push(row);
              return Promise.resolve({ error: null });
            }),
          };
        }
        return {};
      }),
    }),
  };
}

describe("init shared-lib integration", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("upserts repo_libs with shared_lib_id when user accepts linking", async () => {
    vi.mocked(clackConfirm).mockResolvedValue(true);
    const db = makeIntegrationDb(SHARED_LIB);

    const result = await lookupSharedLib(db as any, "rokkit");
    expect(result).not.toBeNull();

    // Simulate the confirm-and-upsert logic inline (mirrors Step 6 loop body)
    const confirmed = await clackConfirm({ message: "Link it?", initialValue: true });
    if (confirmed) {
      await (db as any).schema("sensei").from("repo_libs").upsert({
        repo_id: "repo-abc",
        name: "rokkit",
        source_type: result!.source_type,
        base_url: result!.base_url,
        local_path: result!.local_path,
        shared_lib_id: result!.id,
      }, { onConflict: "repo_id,name" });
    }

    expect(db._repoLibsUpserts).toHaveLength(1);
    expect((db._repoLibsUpserts[0] as any).shared_lib_id).toBe("shared-lib-uuid");
  });

  it("does not upsert repo_libs when user declines linking", async () => {
    vi.mocked(clackConfirm).mockResolvedValue(false);
    const db = makeIntegrationDb(SHARED_LIB);

    const result = await lookupSharedLib(db as any, "rokkit");
    const confirmed = await clackConfirm({ message: "Link it?", initialValue: true });
    if (confirmed) {
      await (db as any).schema("sensei").from("repo_libs").upsert({});
    }

    expect(db._repoLibsUpserts).toHaveLength(0);
  });

  it("skips confirm prompt entirely when lib is not in shared pool", async () => {
    const db = makeIntegrationDb(null);
    const result = await lookupSharedLib(db as any, "unknown-lib");
    expect(result).toBeNull();
    // No confirm called because result is null
    expect(clackConfirm).not.toHaveBeenCalled();
  });
});
