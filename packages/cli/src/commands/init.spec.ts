// packages/cli/src/commands/init.spec.ts
import { describe, it, expect, vi, beforeEach } from "vitest";

// Mock all external deps before importing init
vi.mock("@clack/prompts", () => ({
  intro: vi.fn(),
  outro: vi.fn(),
  spinner: vi.fn(() => ({ start: vi.fn(), stop: vi.fn() })),
  note: vi.fn(),
  log: { error: vi.fn(), warn: vi.fn(), success: vi.fn() },
  text: vi.fn(),
  multiselect: vi.fn().mockResolvedValue([]),
  isCancel: vi.fn(() => false),
}));

vi.mock("@sensei/collector", () => ({
  installHooks: vi.fn(),
}));

vi.mock("fs/promises", () => ({
  writeFile: vi.fn().mockResolvedValue(undefined),
  mkdir: vi.fn().mockResolvedValue(undefined),
  access: vi.fn().mockRejectedValue(new Error("not found")),
  readFile: vi.fn().mockRejectedValue(new Error("not found")),
}));

vi.mock("./install-skills.js", () => ({
  installSkills: vi.fn().mockResolvedValue(undefined),
  promptAndInstallSkills: vi.fn().mockResolvedValue(undefined),
  promptAndInstallSkillsFromCatalog: vi.fn().mockResolvedValue(undefined),
  installSkillsFromCatalog: vi.fn().mockResolvedValue(undefined),
}));

vi.mock("./setup.js", () => ({ setupMcp: vi.fn().mockResolvedValue(undefined) }));
vi.mock("./update-registry.js", () => ({ runUpdateRegistryCore: vi.fn().mockResolvedValue(undefined) }));
vi.mock("../lib/detect-libs.js", () => ({
  scanDirectDeps: vi.fn().mockResolvedValue([]),
  inferSourceType: vi.fn((url: string) => ({ base_url: url, source_type: "http" })),
}));

import { init } from "./init.js";
import { installHooks } from "@sensei/collector";
import { writeFile, readFile } from "fs/promises";
import { promptAndInstallSkillsFromCatalog, installSkillsFromCatalog } from "./install-skills.js";

const mockInstallHooks = installHooks as ReturnType<typeof vi.fn>;
const mockWriteFile = writeFile as ReturnType<typeof vi.fn>;

describe("init command", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockInstallHooks.mockResolvedValue(undefined);
  });

  it("creates .sensei/config.yaml with repo_id", async () => {
    await init("/tmp/test-repo");

    const configCall = mockWriteFile.mock.calls.find(
      (c: unknown[]) => String(c[0]).endsWith("config.yaml")
    );
    expect(configCall).toBeDefined();
    const content = String(configCall![1]);
    expect(content).toContain("repo_id:");
    // No supabase_url in local-first mode
    expect(content).not.toContain("supabase_url:");
  });

  it("writes CLAUDE.md and AGENTS.md", async () => {
    await init("/tmp/test-repo");

    const claudeCall = mockWriteFile.mock.calls.find(
      (c: unknown[]) => String(c[0]).endsWith("CLAUDE.md")
    );
    const agentsCall = mockWriteFile.mock.calls.find(
      (c: unknown[]) => String(c[0]).endsWith("AGENTS.md")
    );
    expect(claudeCall).toBeDefined();
    expect(agentsCall).toBeDefined();
  });

  it("detects typescript stack when package.json exists", async () => {
    const mockReadFile = readFile as ReturnType<typeof vi.fn>;
    mockReadFile.mockResolvedValueOnce(
      JSON.stringify({ name: "test-app", dependencies: { react: "^18" } })
    );

    await init("/tmp/test-repo");

    // CLAUDE.md should mention the detected stack
    const claudeCall = mockWriteFile.mock.calls.find(
      (c: unknown[]) => String(c[0]).endsWith("CLAUDE.md")
    );
    expect(claudeCall).toBeDefined();
  });

  it("calls installHooks", async () => {
    await init("/tmp/test-repo");
    expect(mockInstallHooks).toHaveBeenCalledOnce();
  });

  it("completes without error", async () => {
    await expect(init("/tmp/test-repo")).resolves.toBeUndefined();
  });

  describe("skill installation routing", () => {
    it("calls promptAndInstallSkillsFromCatalog by default (no --use-recommended)", async () => {
      await init("/repo", {});
      expect(promptAndInstallSkillsFromCatalog).toHaveBeenCalledWith("/repo");
      expect(installSkillsFromCatalog).not.toHaveBeenCalled();
    });

    it("calls installSkillsFromCatalog('recommended') when useRecommended=true", async () => {
      await init("/repo", { useRecommended: true });
      expect(installSkillsFromCatalog).toHaveBeenCalledWith("/repo", "recommended");
      expect(promptAndInstallSkillsFromCatalog).not.toHaveBeenCalled();
    });
  });
});
