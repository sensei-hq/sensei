import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { mkdtemp, writeFile, rm, mkdir } from "fs/promises";
import { join } from "path";
import { tmpdir } from "os";
import { Scanner } from "./scanner.js";

describe("Scanner", () => {
  let dir: string;

  beforeEach(async () => {
    dir = await mkdtemp(join(tmpdir(), "engine-scanner-"));
    await mkdir(join(dir, "src"), { recursive: true });
    await writeFile(join(dir, "src/a.ts"), "export const a = 1;");
    await writeFile(join(dir, "src/b.ts"), "export const b = 2;");
    await mkdir(join(dir, "node_modules"), { recursive: true });
    await writeFile(join(dir, "node_modules/lib.js"), "// library");
  });

  afterEach(async () => {
    await rm(dir, { recursive: true, force: true });
  });

  it("discovers typescript files and excludes node_modules", async () => {
    const scanner = new Scanner({ repoPath: dir, repoId: "test-repo" });
    const result = await scanner.scan();
    const paths = result.files.map(f => f.path);
    expect(paths).toContain("src/a.ts");
    expect(paths).toContain("src/b.ts");
    expect(paths.some(p => p.includes("node_modules"))).toBe(false);
  });

  it("marks all files as changed on first scan (no prior state)", async () => {
    const scanner = new Scanner({ repoPath: dir, repoId: "test-repo" });
    const result = await scanner.scan();
    expect(result.changed).toContain("src/a.ts");
    expect(result.changed).toContain("src/b.ts");
    expect(result.deleted).toHaveLength(0);
  });

  it("marks only modified file as changed on re-scan", async () => {
    // Simulate prior state: both files already indexed with current hashes
    const scanner = new Scanner({ repoPath: dir, repoId: "test-repo" });
    const firstResult = await scanner.scan();

    // Modify one file
    await writeFile(join(dir, "src/a.ts"), "export const a = 99;");

    // Re-scan with prior state injected
    const scanner2 = new Scanner({
      repoPath: dir,
      repoId: "test-repo",
      priorState: firstResult.files.map(f => ({
        file_path: f.path,
        mtime: f.mtime,
        content_hash: f.hash,
      })),
    });
    const result2 = await scanner2.scan();
    expect(result2.changed).toContain("src/a.ts");
    expect(result2.changed).not.toContain("src/b.ts");
  });

  it("marks deleted file in result.deleted", async () => {
    const scanner = new Scanner({ repoPath: dir, repoId: "test-repo" });
    await scanner.scan();

    // Remove a.ts, re-scan with prior state that includes it
    await rm(join(dir, "src/a.ts"));
    const scanner2 = new Scanner({
      repoPath: dir,
      repoId: "test-repo",
      priorState: [{ file_path: "src/a.ts", mtime: 0, content_hash: "old" }],
    });
    const result2 = await scanner2.scan();
    expect(result2.deleted).toContain("src/a.ts");
    expect(result2.changed).not.toContain("src/a.ts");
  });
});
