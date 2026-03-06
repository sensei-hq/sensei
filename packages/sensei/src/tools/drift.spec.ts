import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { mkdirSync, writeFileSync, rmSync, statSync } from "fs";
import { join } from "path";
import { checkDrift } from "./drift.js";

const TMP = "/tmp/sensei-test-drift";

beforeEach(() => {
  mkdirSync(join(TMP, ".index"), { recursive: true });
  mkdirSync(join(TMP, "docs"), { recursive: true });
  writeFileSync(join(TMP, "README.md"), "# App");
  writeFileSync(join(TMP, "docs/design.md"), "# Design");
  // Fingerprint from 10 seconds ago — will look drifted
  const past = Date.now() - 10_000;
  writeFileSync(join(TMP, ".index/doc-index.json"), JSON.stringify({
    "README.md": { mtime: past, size: 5 },
    "docs/design.md": { mtime: past, size: 8 },
  }));
});

afterEach(() => rmSync(TMP, { recursive: true, force: true }));

describe("checkDrift", () => {
  it("reports no drift when fingerprints match current state", async () => {
    const readmeS = statSync(join(TMP, "README.md"));
    const designS = statSync(join(TMP, "docs/design.md"));
    writeFileSync(join(TMP, ".index/doc-index.json"), JSON.stringify({
      "README.md": { mtime: readmeS.mtimeMs, size: readmeS.size },
      "docs/design.md": { mtime: designS.mtimeMs, size: designS.size },
    }));
    const result = await checkDrift(TMP);
    expect(result.drifted).toHaveLength(0);
    expect(result.summary).toContain("No drift");
  });

  it("reports drift when a file has been modified", async () => {
    const result = await checkDrift(TMP);
    expect(result.drifted.length).toBeGreaterThan(0);
    expect(result.drifted[0]).toContain("modified");
  });

  it("reports deleted files", async () => {
    writeFileSync(join(TMP, ".index/doc-index.json"), JSON.stringify({
      "README.md": { mtime: Date.now(), size: 5 },
      "docs/deleted.md": { mtime: Date.now(), size: 100 },
    }));
    const result = await checkDrift(TMP);
    expect(result.drifted.some(d => d.includes("deleted.md"))).toBe(true);
  });

  it("returns no-index message if doc-index.json missing", async () => {
    rmSync(join(TMP, ".index/doc-index.json"));
    const result = await checkDrift(TMP);
    expect(result.summary).toContain("Run reindex_repo");
  });
});
