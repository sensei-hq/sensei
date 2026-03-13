/**
 * Integration test — requires local Supabase.
 * Run with: SUPABASE_INTEGRATION=1 bunx vitest run src/indexer.integration.spec.ts
 */
import { describe, it, expect, beforeAll } from "vitest";
import { createClient } from "@supabase/supabase-js";
import { Indexer } from "./indexer.js";
import type { ScanResult, ParsedFile } from "@sensei/shared";

const SUPABASE_URL = process.env.SUPABASE_URL ?? "http://localhost:54321";
const SUPABASE_KEY = process.env.SUPABASE_SERVICE_KEY ?? "";
const RUN = process.env.SUPABASE_INTEGRATION === "1";

describe.skipIf(!RUN || !SUPABASE_KEY)("Indexer integration", () => {
  let client: ReturnType<typeof createClient>;
  const testRepoId = `test-${Date.now()}`;

  beforeAll(async () => {
    client = createClient(SUPABASE_URL, SUPABASE_KEY, {
      db: { schema: "sensei" },
      auth: { persistSession: false },
    });

    // Ensure a repo row exists for foreign key constraints
    await client.from("repos").upsert({
      id: testRepoId,
      name: "test-repo",
      local_path: `/tmp/test-${testRepoId}`,
      stack: [],
      entry_points: [],
    });
  });

  it("upserts symbols into Supabase and reads them back", async () => {
    const indexer = new Indexer(client as any);

    const scan: ScanResult = {
      repoId: testRepoId,
      files: [{ path: "src/test.ts", absPath: "/tmp/test.ts", mtime: Date.now(), hash: "abc", size: 100 }],
      changed: ["src/test.ts"],
      deleted: [],
    };

    const parsed: ParsedFile[] = [{
      filePath: "src/test.ts",
      language: "typescript",
      symbols: [{
        name: "createClient",
        kind: "function",
        signature: "(): Client",
        docstring: "Creates a client",
        lineStart: 1,
        lineEnd: 5,
        isExported: true,
      }],
      edges: [],
      imports: [],
    }];

    const result = await indexer.indexFiles(scan, parsed);
    expect(result.errors).toHaveLength(0);
    expect(result.symbolsUpserted).toBe(1);

    const { data } = await client
      .from("symbols")
      .select("*")
      .eq("repo_id", testRepoId)
      .eq("name", "createClient");

    expect(data).toHaveLength(1);
    expect(data![0].kind).toBe("function");
  });
});
