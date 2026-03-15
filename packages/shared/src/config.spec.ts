// packages/shared/src/config.spec.ts
import { describe, it, expect, afterEach } from "vitest";
import { writeFile, mkdir, rm, mkdtemp } from "fs/promises";
import { join } from "path";
import { tmpdir } from "os";
import { z } from "zod";
import { loadSenseiConfig } from "./config.js";

describe("loadSenseiConfig", () => {
  let tmpDir: string;

  afterEach(async () => {
    if (tmpDir) await rm(tmpDir, { recursive: true, force: true });
  });

  it("returns parsed config with custom_libs", async () => {
    tmpDir = await mkdtemp(join(tmpdir(), "sensei-config-test-"));
    await mkdir(join(tmpDir, ".sensei"), { recursive: true });
    await writeFile(
      join(tmpDir, ".sensei", "config.yaml"),
      `repo_id: test-repo-id\nsupabase_url: https://x.supabase.co\ncustom_libs:\n  - name: rokkit\n    source_type: llms.txt\n    base_url: https://rokkit.dev/llms.txt\n`,
      "utf-8",
    );

    const config = await loadSenseiConfig(tmpDir);

    expect(config).not.toBeNull();
    expect(config!.repo_id).toBe("test-repo-id");
    expect(config!.custom_libs).toHaveLength(1);
    expect(config!.custom_libs![0].name).toBe("rokkit");
    expect(config!.custom_libs![0].source_type).toBe("llms.txt");
  });

  it("returns null when config file is missing", async () => {
    tmpDir = await mkdtemp(join(tmpdir(), "sensei-config-test-"));
    const config = await loadSenseiConfig(tmpDir);
    expect(config).toBeNull();
  });

  it("throws ZodError when custom_libs contains invalid source_type", async () => {
    tmpDir = await mkdtemp(join(tmpdir(), "sensei-config-test-"));
    await mkdir(join(tmpDir, ".sensei"), { recursive: true });
    await writeFile(
      join(tmpDir, ".sensei", "config.yaml"),
      `repo_id: x\nsupabase_url: https://x.supabase.co\ncustom_libs:\n  - name: bad\n    source_type: invalid-type\n`,
      "utf-8",
    );

    await expect(loadSenseiConfig(tmpDir)).rejects.toThrow(z.ZodError);
  });
});
