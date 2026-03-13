import { describe, it, expect } from "vitest";
import type { Repo, RepoSymbol, FileEntry, ScanResult, ParsedFile, IndexResult } from "./domain.js";

describe("domain types", () => {
  it("Repo type has required fields", () => {
    const repo: Repo = {
      id: "uuid-1",
      name: "test-repo",
      local_path: "/tmp/test",
      remote_url: null,
      stack: ["typescript"],
      entry_points: [],
      last_indexed_at: null,
      created_at: new Date().toISOString(),
    };
    expect(repo.id).toBe("uuid-1");
    expect(repo.stack).toContain("typescript");
  });

  it("RepoSymbol kind is restricted to valid values", () => {
    const sym: RepoSymbol = {
      id: "sym-1",
      repo_id: "repo-1",
      file_path: "src/index.ts",
      name: "createClient",
      kind: "function",
      signature: "(): Client",
      docstring: null,
      line_start: 1,
      line_end: 10,
      is_exported: true,
      updated_at: new Date().toISOString(),
    };
    expect(sym.kind).toBe("function");
  });

  it("ScanResult separates changed from deleted", () => {
    const result: ScanResult = {
      repoId: "repo-1",
      files: [],
      changed: ["src/a.ts"],
      deleted: ["src/old.ts"],
    };
    expect(result.changed).toHaveLength(1);
    expect(result.deleted).toHaveLength(1);
  });
});
