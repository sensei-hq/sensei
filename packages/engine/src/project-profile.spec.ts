import { describe, it, expect } from "vitest";
import { join } from "path";
import { mkdtemp, writeFile, mkdir } from "fs/promises";
import { tmpdir } from "os";
import { extractProjectProfile } from "./project-profile.js";

describe("extractProjectProfile", () => {
  it("returns correct profile from package.json", async () => {
    const tmpDir = await mkdtemp(join(tmpdir(), "sensei-test-"));
    await mkdir(join(tmpDir, ".sensei"), { recursive: true });
    await writeFile(
      join(tmpDir, "package.json"),
      JSON.stringify({
        name: "my-repo",
        scripts: { test: "vitest run" },
        devDependencies: { vitest: "^4.0.0", typescript: "^5.0.0" },
      }),
      "utf-8"
    );

    const profile = await extractProjectProfile("repo-id", tmpDir);

    expect(profile.repoName).toBe("my-repo");
    expect(profile.dominantLanguage).toBe("typescript");
    expect(profile.testPattern).toBe("*.spec.ts");
    expect(profile.keySymbols).toEqual([]);
  });

  it("detects python when pyproject.toml exists", async () => {
    const tmpDir = await mkdtemp(join(tmpdir(), "sensei-test-"));
    await writeFile(join(tmpDir, "package.json"), JSON.stringify({ name: "x" }), "utf-8");
    await writeFile(join(tmpDir, "pyproject.toml"), "[tool.poetry]\nname = 'x'\n", "utf-8");

    const profile = await extractProjectProfile("repo-id", tmpDir);
    expect(profile.dominantLanguage).toBe("python");
  });

  it("detects sveltekit framework from dependencies", async () => {
    const tmpDir = await mkdtemp(join(tmpdir(), "sensei-test-"));
    await writeFile(
      join(tmpDir, "package.json"),
      JSON.stringify({ name: "app", dependencies: { "@sveltejs/kit": "^2.0.0" } }),
      "utf-8"
    );

    const profile = await extractProjectProfile("repo-id", tmpDir);
    expect(profile.framework).toBe("sveltekit");
  });

  it("throws when package.json is missing", async () => {
    const tmpDir = await mkdtemp(join(tmpdir(), "sensei-test-"));
    await expect(extractProjectProfile("repo-id", tmpDir)).rejects.toThrow("package.json not found");
  });
});
