import { describe, it, expect, vi, beforeEach } from "vitest";

vi.mock("child_process", () => ({ execSync: vi.fn() }));
import { execSync } from "child_process";

import {
  getCurrentBranch,
  isCleanWorkingTree,
  branchExists,
  readFileFromBranch,
} from "./git.js";

const REPO = "/fake/repo";
const mockExec = execSync as ReturnType<typeof vi.fn>;

beforeEach(() => vi.clearAllMocks());

describe("getCurrentBranch", () => {
  it("returns trimmed branch name", () => {
    mockExec.mockReturnValueOnce("main\n");
    expect(getCurrentBranch(REPO)).toBe("main");
    expect(mockExec).toHaveBeenCalledWith("git rev-parse --abbrev-ref HEAD", { cwd: REPO, encoding: "utf-8" });
  });
});

describe("isCleanWorkingTree", () => {
  it("returns true when output is empty", () => {
    mockExec.mockReturnValueOnce("");
    expect(isCleanWorkingTree(REPO)).toBe(true);
  });
  it("returns false when files are modified", () => {
    mockExec.mockReturnValueOnce(" M src/foo.ts\n");
    expect(isCleanWorkingTree(REPO)).toBe(false);
  });
});

describe("branchExists", () => {
  it("returns true when branch is found", () => {
    mockExec.mockReturnValueOnce("benchmark/wild-cat-a\n");
    expect(branchExists(REPO, "benchmark/wild-cat-a")).toBe(true);
  });
  it("returns false when branch is not found", () => {
    mockExec.mockReturnValueOnce("");
    expect(branchExists(REPO, "benchmark/wild-cat-a")).toBe(false);
  });
});

describe("readFileFromBranch", () => {
  it("returns file content from branch", () => {
    mockExec.mockReturnValueOnce('{"run":"wild-cat"}');
    expect(readFileFromBranch(REPO, "benchmark/wild-cat-a", ".sensei/benchmark-wild-cat.json"))
      .toBe('{"run":"wild-cat"}');
    expect(mockExec).toHaveBeenCalledWith(
      'git show benchmark/wild-cat-a:.sensei/benchmark-wild-cat.json',
      { cwd: REPO, encoding: "utf-8" },
    );
  });
});
