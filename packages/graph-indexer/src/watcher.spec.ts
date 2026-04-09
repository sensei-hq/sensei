import { describe, it, expect, beforeAll, afterAll } from "vitest";
import { writeFile, mkdir, rm } from "node:fs/promises";
import { join } from "node:path";
import { randomBytes } from "node:crypto";
import { homedir } from "node:os";
import { watchRepo } from "./watcher.js";

const REPO_ID = `watcher-test-${randomBytes(4).toString("hex")}`;

const FILE_V1 = `
export function greet(name: string): string {
  return \`Hello, \${name}!\`;
}
`;

const FILE_V2 = `
export function greet(name: string): string {
  return \`Hi, \${name}!\`;
}

export function farewell(name: string): string {
  return \`Goodbye, \${name}!\`;
}
`;

let tmpDir: string;

beforeAll(async () => {
  tmpDir = join("/tmp", `sensei-watcher-${randomBytes(4).toString("hex")}`);
  await mkdir(tmpDir, { recursive: true });
});

afterAll(async () => {
  await rm(tmpDir, { recursive: true, force: true });
  const repoDbDir = join(homedir(), ".sensei", "projects", REPO_ID);
  await rm(repoDbDir, { recursive: true, force: true }).catch(() => {});
});

describe("watchRepo", () => {
  it("performs initial index on first start", async () => {
    await writeFile(join(tmpDir, "greet.ts"), FILE_V1);

    const updates: import("./watcher.js").IncrementalResult[] = [];
    const handle = await watchRepo({
      repoPath: tmpDir,
      repoId: REPO_ID,
      project: "watcher-test",
      include: ["**/*.ts"],
      exclude: ["**/*.spec.ts"],
      onUpdate: (r) => updates.push(r),
    });

    await handle.stop();
    expect(updates.length).toBeGreaterThanOrEqual(1);
    expect(updates[0].added).toBeGreaterThanOrEqual(1);
  });

  it("detects no changes on second start (manifest matches disk)", async () => {
    const updates: import("./watcher.js").IncrementalResult[] = [];
    const handle = await watchRepo({
      repoPath: tmpDir,
      repoId: REPO_ID,
      project: "watcher-test",
      include: ["**/*.ts"],
      exclude: ["**/*.spec.ts"],
      onUpdate: (r) => updates.push(r),
    });

    await handle.stop();
    // All files already indexed — no update callback should fire
    expect(updates.length).toBe(0);
  });

  it("detects file update via manual rescan", async () => {
    // Rewrite the file with new content
    await writeFile(join(tmpDir, "greet.ts"), FILE_V2);

    const updates: import("./watcher.js").IncrementalResult[] = [];
    const handle = await watchRepo({
      repoPath: tmpDir,
      repoId: REPO_ID,
      project: "watcher-test",
      include: ["**/*.ts"],
      exclude: ["**/*.spec.ts"],
      onUpdate: (r) => updates.push(r),
    });

    await handle.stop();
    // The changed file should be detected
    const totalUpdated = updates.reduce((s, r) => s + r.updated, 0);
    expect(totalUpdated).toBeGreaterThanOrEqual(1);
  });
});
