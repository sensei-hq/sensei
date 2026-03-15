// packages/cli/src/lib/detect-libs.spec.ts
import { describe, it, expect, afterEach } from "vitest";
import { mkdtemp, rm, writeFile } from "fs/promises";
import { join } from "path";
import { tmpdir } from "os";
import { scanDirectDeps, inferSourceType } from "./detect-libs.js";

describe("scanDirectDeps", () => {
  let tmpDir: string;
  afterEach(async () => { if (tmpDir) await rm(tmpDir, { recursive: true, force: true }); });

  it("returns direct dependencies from package.json, excluding @types/*", async () => {
    tmpDir = await mkdtemp(join(tmpdir(), "sensei-detect-test-"));
    await writeFile(join(tmpDir, "package.json"), JSON.stringify({
      dependencies: { rokkit: "^1.0.0", kavach: "^2.0.0", react: "^18.0.0" },
      devDependencies: { vitest: "^1.0.0", "@types/node": "^20.0.0" },
    }), "utf-8");

    const deps = await scanDirectDeps(tmpDir);

    expect(deps).toContain("rokkit");
    expect(deps).toContain("kavach");
    expect(deps).toContain("react");
    expect(deps).not.toContain("vitest");
    expect(deps).not.toContain("@types/node");
  });

  it("returns empty array when no manifest files found", async () => {
    tmpDir = await mkdtemp(join(tmpdir(), "sensei-detect-test-"));
    const deps = await scanDirectDeps(tmpDir);
    expect(deps).toEqual([]);
  });

  it("includes deps from requirements.txt when present", async () => {
    tmpDir = await mkdtemp(join(tmpdir(), "sensei-detect-test-"));
    await writeFile(join(tmpDir, "requirements.txt"), "requests==2.31.0\nfastapi>=0.100.0\npydantic\n", "utf-8");
    const deps = await scanDirectDeps(tmpDir);
    expect(deps).toContain("requests");
    expect(deps).toContain("fastapi");
    expect(deps).toContain("pydantic");
  });
});

describe("inferSourceType", () => {
  it("detects llms.txt URL", () => {
    expect(inferSourceType("https://rokkit.dev/llms.txt").source_type).toBe("llms.txt");
    expect(inferSourceType("https://docs.example.com/llms.txt").source_type).toBe("llms.txt");
  });

  it("detects http URL (non-llms.txt)", () => {
    expect(inferSourceType("https://kavach.dev/docs").source_type).toBe("http");
    expect(inferSourceType("https://raw.githubusercontent.com/user/repo/main/README.md").source_type).toBe("http");
  });

  it("detects local path", () => {
    expect(inferSourceType("/home/user/mylib/docs").source_type).toBe("local");
    expect(inferSourceType("./docs").source_type).toBe("local");
  });

  it("returns base_url for http/llms.txt and local_path for local", () => {
    expect(inferSourceType("https://rokkit.dev/llms.txt").base_url).toBe("https://rokkit.dev/llms.txt");
    expect(inferSourceType("/docs").local_path).toBe("/docs");
  });
});
