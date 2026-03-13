import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { mkdir, writeFile, rm } from "fs/promises";
import { join } from "path";
import os from "os";

// Tests use real temp directories — no mocking of fs
describe("loadSenseiConfig", () => {
  let tmpRepo: string;
  let tmpHome: string;

  beforeEach(async () => {
    tmpRepo = await makeTmpDir("repo");
    tmpHome = await makeTmpDir("home");
  });

  afterEach(async () => {
    await rm(tmpRepo, { recursive: true, force: true });
    await rm(tmpHome, { recursive: true, force: true });
  });

  it("returns null when .sensei/config.yaml does not exist", async () => {
    const { loadSenseiConfig } = await import("./config.js");
    const result = await loadSenseiConfig(tmpRepo);
    expect(result).toBeNull();
  });

  it("reads repo_id and supabase_url from .sensei/config.yaml", async () => {
    const { loadSenseiConfig } = await import("./config.js");
    await mkdir(join(tmpRepo, ".sensei"), { recursive: true });
    await writeFile(join(tmpRepo, ".sensei", "config.yaml"),
      "repo_id: abc-123\nsupabase_url: http://localhost:54321\n");
    const result = await loadSenseiConfig(tmpRepo);
    expect(result?.repo_id).toBe("abc-123");
    expect(result?.supabase_url).toBe("http://localhost:54321");
  });

  it("reads service key from credentials.yaml", async () => {
    const { loadCredentials } = await import("./config.js");
    await mkdir(join(tmpHome, ".config", "sensei"), { recursive: true });
    await writeFile(join(tmpHome, ".config", "sensei", "credentials.yaml"),
      "supabase_service_key: sk-test-key\n");
    const result = await loadCredentials(tmpHome);
    expect(result?.supabase_service_key).toBe("sk-test-key");
  });

  it("returns null for credentials when file does not exist", async () => {
    const { loadCredentials } = await import("./config.js");
    const result = await loadCredentials(tmpHome);
    expect(result).toBeNull();
  });

  it("prefers SUPABASE_SERVICE_KEY env over credentials file", async () => {
    const { loadCredentials } = await import("./config.js");
    process.env.SUPABASE_SERVICE_KEY = "env-key";
    const result = await loadCredentials(tmpHome);
    expect(result?.supabase_service_key).toBe("env-key");
    delete process.env.SUPABASE_SERVICE_KEY;
  });
});

async function makeTmpDir(prefix: string): Promise<string> {
  const dir = join(os.tmpdir(), `sensei-config-test-${prefix}-${Date.now()}`);
  await mkdir(dir, { recursive: true });
  return dir;
}
