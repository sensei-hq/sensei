import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { extractGroundTruth } from "./ground-truth.js";
import { writeFile, mkdir, rm } from "fs/promises";
import { join } from "path";
import { tmpdir } from "os";

describe("extractGroundTruth", () => {
  let dir: string;

  beforeEach(async () => {
    dir = join(tmpdir(), `gt-test-${Date.now()}`);
    await mkdir(join(dir, "packages", "test-pkg", "src"), { recursive: true });
  });

  afterEach(() => rm(dir, { recursive: true, force: true }));

  it("finds exported functions", async () => {
    await writeFile(join(dir, "packages", "test-pkg", "src", "foo.ts"), `
      export function doThing() {}
      export const helper = () => {};
      function internal() {}
    `);
    const result = await extractGroundTruth(dir);
    expect(result.files).toContain("packages/test-pkg/src/foo.ts");
    expect(result.exportCount).toBe(2);
  });

  it("skips spec files", async () => {
    await writeFile(join(dir, "packages", "test-pkg", "src", "foo.spec.ts"), `export function test() {}`);
    const result = await extractGroundTruth(dir);
    expect(result.exportCount).toBe(0);
  });
});
