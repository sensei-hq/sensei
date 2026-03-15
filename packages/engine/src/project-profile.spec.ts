import { describe, it, expect } from "vitest";
import { join } from "path";
import { mkdtemp, writeFile, mkdir } from "fs/promises";
import { tmpdir } from "os";
import { extractProjectProfile } from "./project-profile.js";

function makeDb(opts: { repoError?: boolean; symbolRows?: Array<{ name: string; file_path: string }> } = {}) {
  return {
    from: (table: string) => ({
      select: () => ({
        eq: (_col: string, _val: string) => ({
          single: async () => {
            if (table === "repos") {
              if (opts.repoError) return { data: null, error: { message: "DB error" } };
              return { data: { name: "my-repo" }, error: null };
            }
            return { data: null, error: null };
          },
          limit: async () => {
            if (table === "symbols") return { data: opts.symbolRows ?? [], error: null };
            return { data: [], error: null };
          },
        }),
      }),
    }),
  };
}

describe("extractProjectProfile", () => {
  it("returns correct dominantLanguage and keySymbols from fixture symbols", async () => {
    const tmpDir = await mkdtemp(join(tmpdir(), "sensei-test-"));
    await mkdir(join(tmpDir, ".sensei"), { recursive: true });
    await writeFile(
      join(tmpDir, "package.json"),
      JSON.stringify({ name: "my-repo", scripts: { test: "vitest run" }, devDependencies: { vitest: "^4.0.0" } }),
      "utf-8"
    );

    const symbolRows = [
      { name: "createClient", file_path: "packages/shared/src/client.ts" },
      { name: "indexRepo", file_path: "packages/engine/src/indexer.ts" },
      { name: "runCli", file_path: "packages/cli/src/cli.ts" },
    ];
    const db = makeDb({ symbolRows });

    const profile = await extractProjectProfile(db as any, "repo-id", tmpDir);

    expect(profile.dominantLanguage).toBe("typescript");
    expect(profile.keySymbols).toContain("createClient");
    expect(profile.keySymbols).toContain("indexRepo");
    expect(profile.testPattern).toBe("*.spec.ts");
  });

  it("throws when package.json is missing", async () => {
    const tmpDir = await mkdtemp(join(tmpdir(), "sensei-test-"));
    const db = makeDb();
    await expect(extractProjectProfile(db as any, "repo-id", tmpDir)).rejects.toThrow("package.json not found");
  });

  it("throws on DB error fetching repo", async () => {
    const tmpDir = await mkdtemp(join(tmpdir(), "sensei-test-"));
    await writeFile(join(tmpDir, "package.json"), JSON.stringify({ name: "x" }), "utf-8");
    const db = makeDb({ repoError: true });
    await expect(extractProjectProfile(db as any, "repo-id", tmpDir)).rejects.toThrow("DB error");
  });
});
