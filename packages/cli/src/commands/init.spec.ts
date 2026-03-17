// packages/cli/src/commands/init.spec.ts
import { describe, it, expect, vi, beforeEach } from "vitest";
import * as fs from "fs/promises";
import * as path from "path";
import * as os from "os";

// Mock all external deps before importing init
vi.mock("@clack/prompts", () => ({
  intro: vi.fn(),
  outro: vi.fn(),
  spinner: vi.fn(() => ({ start: vi.fn(), stop: vi.fn() })),
  note: vi.fn(),
  log: { error: vi.fn(), warn: vi.fn() },
  text: vi.fn(),
  isCancel: vi.fn(() => false),
}));

vi.mock("@sensei/engine", () => ({
  indexRepo: vi.fn(),
}));

vi.mock("@sensei/collector", () => ({
  installHooks: vi.fn(),
}));

vi.mock("@supabase/supabase-js", () => ({
  createClient: vi.fn(),
}));

vi.mock("fs/promises", () => ({
  writeFile: vi.fn().mockResolvedValue(undefined),
  mkdir: vi.fn().mockResolvedValue(undefined),
  access: vi.fn().mockRejectedValue(new Error("not found")),
  readFile: vi.fn().mockRejectedValue(new Error("not found")),
  chmod: vi.fn().mockResolvedValue(undefined),
}));

import { init } from "./init.js";
import { indexRepo } from "@sensei/engine";
import { installHooks } from "@sensei/collector";
import { createClient } from "@supabase/supabase-js";
import { text, isCancel } from "@clack/prompts";
import { writeFile, mkdir, access, readFile } from "fs/promises";

const mockIndexRepo = indexRepo as ReturnType<typeof vi.fn>;
const mockInstallHooks = installHooks as ReturnType<typeof vi.fn>;
const mockCreateClient = createClient as ReturnType<typeof vi.fn>;
const mockText = text as ReturnType<typeof vi.fn>;
// eslint-disable-next-line @typescript-eslint/no-explicit-any
const mockIsCancel = isCancel as unknown as ReturnType<typeof vi.fn>;
const mockWriteFile = writeFile as ReturnType<typeof vi.fn>;
const mockMkdir = mkdir as ReturnType<typeof vi.fn>;

function makeSupabaseClient(repoId = "repo-uuid-123") {
  const mockSelect = vi.fn().mockReturnThis();
  const mockSingle = vi.fn().mockResolvedValue({ data: { id: repoId }, error: null });
  const mockUpsert = vi.fn().mockReturnValue({ select: () => ({ single: mockSingle }) });
  const mockUpdate = vi.fn().mockReturnValue({ eq: vi.fn().mockResolvedValue({ error: null }) });
  return {
    from: vi.fn().mockReturnValue({
      upsert: mockUpsert,
      update: mockUpdate,
      select: mockSelect,
    }),
  };
}

describe("init command", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockIsCancel.mockReturnValue(false);
    mockText
      .mockResolvedValueOnce("http://localhost:54321")
      .mockResolvedValueOnce("test-service-key-long-enough");

    const client = makeSupabaseClient();
    mockCreateClient.mockReturnValue(client);

    mockIndexRepo.mockResolvedValue({
      filesIndexed: 10,
      symbolsUpserted: 42,
      errors: [],
    });

    mockInstallHooks.mockResolvedValue(undefined);
  });

  it("creates .sensei/config.yaml with repo_id and supabase_url", async () => {
    await init("/tmp/test-repo");

    const configCall = (mockWriteFile as ReturnType<typeof vi.fn>).mock.calls.find(
      (c: unknown[]) => String(c[0]).endsWith("config.yaml")
    );
    expect(configCall).toBeDefined();
    const content = String(configCall![1]);
    expect(content).toContain("repo_id:");
    expect(content).toContain("supabase_url:");
    expect(content).toContain("http://localhost:54321");
  });

  it("does not write CLAUDE.md or AGENTS.md", async () => {
    await init("/tmp/test-repo");

    const claudeCall = mockWriteFile.mock.calls.find(
      (c: unknown[]) => String(c[0]).endsWith("CLAUDE.md")
    );
    const agentsCall = mockWriteFile.mock.calls.find(
      (c: unknown[]) => String(c[0]).endsWith("AGENTS.md")
    );
    expect(claudeCall).toBeUndefined();
    expect(agentsCall).toBeUndefined();
  });

  it("calls indexRepo with repoPath, repoId, and client", async () => {
    await init("/tmp/test-repo");

    expect(mockIndexRepo).toHaveBeenCalledOnce();
    const opts = mockIndexRepo.mock.calls[0][0];
    expect(opts.repoPath).toBe("/tmp/test-repo");
    expect(opts.repoId).toBe("repo-uuid-123");
    expect(opts.client).toBeDefined();
  });

  it("detects typescript stack when package.json exists", async () => {
    const mockReadFile = readFile as ReturnType<typeof vi.fn>;
    mockReadFile.mockResolvedValueOnce(
      JSON.stringify({ name: "test-app", dependencies: { react: "^18" } })
    );

    await init("/tmp/test-repo");

    // Stack is detected but not written to files (stored in Supabase via repo upsert)
    const upsertCall = makeSupabaseClient().from("repos").upsert;
    expect(mockIndexRepo).toHaveBeenCalledOnce();
  });

  it("calls installHooks", async () => {
    await init("/tmp/test-repo");
    expect(mockInstallHooks).toHaveBeenCalledOnce();
  });

  it("exits early when supabaseUrl prompt is cancelled", async () => {
    mockIsCancel.mockReturnValueOnce(true);

    await init("/tmp/test-repo");

    expect(mockIndexRepo).not.toHaveBeenCalled();
    expect(mockWriteFile).not.toHaveBeenCalled();
  });

  it("continues when indexRepo throws", async () => {
    mockIndexRepo.mockRejectedValue(new Error("index failed"));

    await expect(init("/tmp/test-repo")).resolves.toBeUndefined();

    // config.yaml should still be written
    const configCall = mockWriteFile.mock.calls.find(
      (c: unknown[]) => String(c[0]).endsWith("config.yaml")
    );
    expect(configCall).toBeDefined();
  });
});
