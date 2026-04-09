import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { mkdirSync, writeFileSync, rmSync } from "fs";
import { join } from "path";

const TMP = "/tmp/sensei-test-drift";
const STATE_DIR = join(TMP, "sensei-projects", "test-repo");

// Mock homedir to point at TMP so index-state.json writes go to a temp dir
vi.mock("os", async (importOriginal) => {
  const actual = await importOriginal<typeof import("os")>();
  return { ...actual, homedir: () => join(TMP, "sensei-home") };
});

vi.mock("@sensei/shared", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@sensei/shared")>();
  return {
    ...actual,
    loadSenseiConfig: vi.fn().mockResolvedValue({ repo_id: "test-repo" }),
  };
});

const { mockExecFileSync } = vi.hoisted(() => ({
  mockExecFileSync: vi.fn().mockReturnValue(Buffer.from("")),
}));
vi.mock("child_process", () => ({ execFileSync: mockExecFileSync }));

import { checkDrift } from "./drift.js";

const STATE_HOME = join(TMP, "sensei-home", ".sensei", "projects", "test-repo");

function writeStateFile(lastCommit = "abc1234def5678") {
  mkdirSync(STATE_HOME, { recursive: true });
  writeFileSync(
    join(STATE_HOME, "index-state.json"),
    JSON.stringify({ lastCommit, indexedAt: new Date().toISOString(), repoPath: TMP }),
  );
}

beforeEach(() => {
  mkdirSync(join(TMP, ".sensei"), { recursive: true });
  mkdirSync(join(TMP, "docs"), { recursive: true });
  writeFileSync(join(TMP, "README.md"), "# App");
  writeFileSync(join(TMP, "docs/design.md"), "# Design");
  mockExecFileSync.mockReturnValue(Buffer.from(""));
});

afterEach(() => {
  rmSync(TMP, { recursive: true, force: true });
  mockExecFileSync.mockReset();
  mockExecFileSync.mockReturnValue(Buffer.from(""));
});

describe("checkDrift", () => {
  it("returns no-index message when index-state.json is missing", async () => {
    const result = await checkDrift(TMP);
    expect(result.summary).toContain("Run sensei index");
  });

  it("reports no drift when git diff returns empty", async () => {
    writeStateFile();
    mockExecFileSync.mockReturnValue(Buffer.from(""));
    const result = await checkDrift(TMP);
    expect(result.drifted).toHaveLength(0);
    expect(result.summary).toContain("No drift");
  });

  it("reports raw-modified for changed doc files (no traceability)", async () => {
    writeStateFile();
    mockExecFileSync.mockReturnValue(Buffer.from("README.md\ndocs/design.md\nsrc/app.ts\n"));
    const result = await checkDrift(TMP);
    // Only .md files are reported when no traceability.json
    expect(result.drifted.some(d => d.docPath === "README.md")).toBe(true);
    expect(result.drifted.every(d => d.reason === "raw-modified")).toBe(true);
    // .ts files are not flagged without traceability
    expect(result.drifted.some(d => d.docPath === "src/app.ts")).toBe(false);
  });

  it("reports code-changed when covered source file changed but doc didn't", async () => {
    writeStateFile();
    // Write traceability: design.md covers src/app.ts
    writeFileSync(
      join(TMP, ".sensei", "traceability.json"),
      JSON.stringify({ "docs/design.md": ["src/app.ts"] }),
    );
    // Only src/app.ts changed, docs/design.md did not
    mockExecFileSync.mockReturnValue(Buffer.from("src/app.ts\n"));
    const result = await checkDrift(TMP);
    expect(result.drifted).toHaveLength(1);
    expect(result.drifted[0].docPath).toBe("docs/design.md");
    expect(result.drifted[0].reason).toBe("code-changed");
    expect(result.drifted[0].changedFiles).toContain("src/app.ts");
  });

  it("returns not-initialised message when config is missing", async () => {
    const { loadSenseiConfig } = await import("@sensei/shared");
    vi.mocked(loadSenseiConfig).mockResolvedValueOnce(null);
    const result = await checkDrift(TMP);
    expect(result.summary).toContain("sensei init");
  });

  it("exposes lastIndexedCommit and indexedAt in result", async () => {
    writeStateFile("deadbeef1234");
    const result = await checkDrift(TMP);
    expect(result.lastIndexedCommit).toBe("deadbeef1234");
    expect(typeof result.indexedAt).toBe("string");
  });
});
