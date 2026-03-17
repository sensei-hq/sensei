// packages/cli/src/commands/install-skills.spec.ts
import { describe, it, expect, vi, beforeEach } from "vitest";

vi.mock("@clack/prompts", () => ({
  select: vi.fn(),
  multiselect: vi.fn(),
  isCancel: vi.fn(() => false),
  spinner: vi.fn(() => ({ start: vi.fn(), stop: vi.fn() })),
  log: { warn: vi.fn() },
}));

vi.mock("fs/promises", () => ({
  // readdir is not called by the catalog functions — they iterate SKILL_CATALOG directly
  access: vi.fn().mockResolvedValue(undefined),
  copyFile: vi.fn().mockResolvedValue(undefined),
  mkdir: vi.fn().mockResolvedValue(undefined),
}));

import { installSkillsFromCatalog, promptAndInstallSkillsFromCatalog } from "./install-skills.js";
import { multiselect, isCancel } from "@clack/prompts";
import { copyFile } from "fs/promises";

const mockMultiselect = multiselect as ReturnType<typeof vi.fn>;
const mockIsCancel = isCancel as unknown as ReturnType<typeof vi.fn>;
const mockCopyFile = copyFile as ReturnType<typeof vi.fn>;

beforeEach(() => {
  vi.clearAllMocks();
});

describe("installSkillsFromCatalog", () => {
  it("installs only recommended skills in 'recommended' mode", async () => {
    await installSkillsFromCatalog("/repo", "recommended");
    const copiedNames = (mockCopyFile as ReturnType<typeof vi.fn>).mock.calls.map(
      (args: string[]) => args[0].split("/").slice(-2, -1)[0]
    );
    expect(copiedNames).toContain("zero-errors-policy");
    expect(copiedNames).toContain("managing-project-sessions");
    expect(copiedNames).toContain("pattern-based-development");
    // Non-recommended not included
    expect(copiedNames).not.toContain("decomposing-broad-tasks");
  });

  it("installs all catalog skills in 'all' mode", async () => {
    await installSkillsFromCatalog("/repo", "all");
    const copiedNames = (mockCopyFile as ReturnType<typeof vi.fn>).mock.calls.map(
      (args: string[]) => args[0].split("/").slice(-2, -1)[0]
    );
    expect(copiedNames).toContain("zero-errors-policy");
    expect(copiedNames).toContain("decomposing-broad-tasks");
  });
});

describe("promptAndInstallSkillsFromCatalog", () => {
  it("uses catalog recommended entries as initial values in multiselect", async () => {
    mockMultiselect.mockResolvedValue(["zero-errors-policy", "managing-project-sessions"]);
    await promptAndInstallSkillsFromCatalog("/repo");
    const call = mockMultiselect.mock.calls[0][0];
    expect(call.initialValues).toContain("zero-errors-policy");
    expect(call.initialValues).not.toContain("decomposing-broad-tasks");
  });

  it("installs only the user-selected skills", async () => {
    mockMultiselect.mockResolvedValue(["zero-errors-policy"]);
    await promptAndInstallSkillsFromCatalog("/repo");
    const copiedNames = (mockCopyFile as ReturnType<typeof vi.fn>).mock.calls.map(
      (args: string[]) => args[0].split("/").slice(-2, -1)[0]
    );
    expect(copiedNames).toEqual(["zero-errors-policy"]);
  });

  it("does nothing when user cancels", async () => {
    mockIsCancel.mockReturnValue(true);
    await promptAndInstallSkillsFromCatalog("/repo");
    expect(mockCopyFile).not.toHaveBeenCalled();
  });
});
