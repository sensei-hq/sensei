import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { mkdirSync, writeFileSync, rmSync } from "fs";
import { join } from "path";
import { inferEntryPoints } from "./entry-point-adapters.js";

const TMP = "/tmp/sensei-test-entry-points";

beforeEach(() => mkdirSync(TMP, { recursive: true }));
afterEach(() => rmSync(TMP, { recursive: true, force: true }));

describe("inferFromPackageJson — root bin field", () => {
  it("resolves bin dist path to src counterpart when source exists", async () => {
    mkdirSync(join(TMP, "src"), { recursive: true });
    writeFileSync(join(TMP, "src/cli.ts"), "// entry");
    writeFileSync(join(TMP, "package.json"), JSON.stringify({
      name: "my-app",
      bin: { "my-app": "./dist/cli.js" },
    }));
    const results = await inferEntryPoints(TMP);
    expect(results.some(r => r.path === "src/cli.ts")).toBe(true);
  });

  it("falls back to dist path when source does not exist", async () => {
    writeFileSync(join(TMP, "package.json"), JSON.stringify({
      name: "my-app",
      bin: { "my-app": "./dist/cli.js" },
    }));
    const results = await inferEntryPoints(TMP);
    expect(results.some(r => r.path === "dist/cli.js")).toBe(true);
  });

  it("uses bin key as basis for inferredRole", async () => {
    mkdirSync(join(TMP, "src"), { recursive: true });
    writeFileSync(join(TMP, "src/cli.ts"), "// entry");
    writeFileSync(join(TMP, "package.json"), JSON.stringify({
      name: "my-app",
      bin: { "my-app": "./dist/cli.js" },
    }));
    const results = await inferEntryPoints(TMP);
    const entry = results.find(r => r.path === "src/cli.ts");
    expect(entry?.inferredRole).toContain("my-app");
  });

  it("reads workspace packages bin fields", async () => {
    mkdirSync(join(TMP, "packages/cli/src"), { recursive: true });
    writeFileSync(join(TMP, "packages/cli/src/cli.ts"), "// entry");
    writeFileSync(join(TMP, "packages/cli/package.json"), JSON.stringify({
      name: "@my/cli",
      bin: { "my-cli": "./dist/cli.js" },
    }));
    writeFileSync(join(TMP, "package.json"), JSON.stringify({
      name: "root",
      workspaces: ["packages/*"],
    }));
    const results = await inferEntryPoints(TMP);
    expect(results.some(r => r.path === "packages/cli/src/cli.ts")).toBe(true);
  });
});

describe("inferFromPackageJson — bin string shorthand", () => {
  it("handles bin as a string (shorthand form)", async () => {
    mkdirSync(join(TMP, "src"), { recursive: true });
    writeFileSync(join(TMP, "src/cli.ts"), "// entry");
    writeFileSync(join(TMP, "package.json"), JSON.stringify({
      name: "@my/tool",
      bin: "./dist/cli.js",
    }));
    const results = await inferEntryPoints(TMP);
    expect(results.some(r => r.path === "src/cli.ts")).toBe(true);
  });

  it("derives role from package name stripping scope for bin string", async () => {
    mkdirSync(join(TMP, "src"), { recursive: true });
    writeFileSync(join(TMP, "src/cli.ts"), "// entry");
    writeFileSync(join(TMP, "package.json"), JSON.stringify({
      name: "@my/tool",
      bin: "./dist/cli.js",
    }));
    const results = await inferEntryPoints(TMP);
    const entry = results.find(r => r.path === "src/cli.ts");
    expect(entry?.inferredRole).toContain("tool");
  });
});

describe("inferFromPackageJson — main field", () => {
  it("resolves main dist path to src counterpart", async () => {
    mkdirSync(join(TMP, "src"), { recursive: true });
    writeFileSync(join(TMP, "src/index.ts"), "// main");
    writeFileSync(join(TMP, "package.json"), JSON.stringify({
      name: "my-lib",
      main: "./dist/index.js",
    }));
    const results = await inferEntryPoints(TMP);
    expect(results.some(r => r.path === "src/index.ts")).toBe(true);
  });

  it("labels main entry with 'main entry' role", async () => {
    mkdirSync(join(TMP, "src"), { recursive: true });
    writeFileSync(join(TMP, "src/index.ts"), "// main");
    writeFileSync(join(TMP, "package.json"), JSON.stringify({
      name: "my-lib",
      main: "./dist/index.js",
    }));
    const results = await inferEntryPoints(TMP);
    const entry = results.find(r => r.path === "src/index.ts");
    expect(entry?.inferredRole).toContain("main entry");
  });
});

describe("inferFromPyprojectToml", () => {
  it("extracts [project.scripts] entries", async () => {
    writeFileSync(join(TMP, "pyproject.toml"),
      `[project]\nname = "my-pkg"\n\n[project.scripts]\nmy-cli = "my_package.cli:main"\n`);
    mkdirSync(join(TMP, "my_package"), { recursive: true });
    writeFileSync(join(TMP, "my_package/cli.py"), "# cli");
    const results = await inferEntryPoints(TMP);
    expect(results.some(r => r.path === "my_package/cli.py")).toBe(true);
  });

  it("uses script name in inferredRole", async () => {
    writeFileSync(join(TMP, "pyproject.toml"),
      `[project.scripts]\nmy-cli = "my_package.cli:main"\n`);
    mkdirSync(join(TMP, "my_package"), { recursive: true });
    writeFileSync(join(TMP, "my_package/cli.py"), "# cli");
    const results = await inferEntryPoints(TMP);
    const entry = results.find(r => r.path === "my_package/cli.py");
    expect(entry?.inferredRole).toContain("my-cli");
  });

  it("skips script if source file does not exist", async () => {
    writeFileSync(join(TMP, "pyproject.toml"),
      `[project.scripts]\nghost = "ghost.module:main"\n`);
    const results = await inferEntryPoints(TMP);
    expect(results.some(r => r.path === "ghost/module.py")).toBe(false);
  });
});

describe("inferFromGoConvention", () => {
  it("finds cmd/*/main.go files", async () => {
    mkdirSync(join(TMP, "cmd/server"), { recursive: true });
    writeFileSync(join(TMP, "cmd/server/main.go"), "package main");
    const results = await inferEntryPoints(TMP);
    expect(results.some(r => r.path === "cmd/server/main.go")).toBe(true);
  });

  it("uses directory name as basis for inferredRole", async () => {
    mkdirSync(join(TMP, "cmd/worker"), { recursive: true });
    writeFileSync(join(TMP, "cmd/worker/main.go"), "package main");
    const results = await inferEntryPoints(TMP);
    const entry = results.find(r => r.path === "cmd/worker/main.go");
    expect(entry?.inferredRole).toContain("worker");
  });
});

describe("inferFromCargoToml", () => {
  it("parses [[bin]] entries with name and path", async () => {
    writeFileSync(join(TMP, "Cargo.toml"),
      `[package]\nname = "my-crate"\n\n[[bin]]\nname = "my-tool"\npath = "src/bin/main.rs"\n`);
    mkdirSync(join(TMP, "src/bin"), { recursive: true });
    writeFileSync(join(TMP, "src/bin/main.rs"), "fn main() {}");
    const results = await inferEntryPoints(TMP);
    expect(results.some(r => r.path === "src/bin/main.rs")).toBe(true);
  });

  it("infers role from bin name", async () => {
    writeFileSync(join(TMP, "Cargo.toml"),
      `[[bin]]\nname = "my-tool"\npath = "src/bin/main.rs"\n`);
    mkdirSync(join(TMP, "src/bin"), { recursive: true });
    writeFileSync(join(TMP, "src/bin/main.rs"), "fn main() {}");
    const results = await inferEntryPoints(TMP);
    const entry = results.find(r => r.path === "src/bin/main.rs");
    expect(entry?.inferredRole).toContain("my-tool");
  });
});

describe("inferEntryPoints — deduplication", () => {
  it("deduplicates candidates with the same path", async () => {
    mkdirSync(join(TMP, "src"), { recursive: true });
    writeFileSync(join(TMP, "src/cli.ts"), "// cli");
    writeFileSync(join(TMP, "package.json"), JSON.stringify({
      name: "root",
      bin: { "tool": "./dist/cli.js" },
    }));
    const results = await inferEntryPoints(TMP);
    const count = results.filter(r => r.path === "src/cli.ts").length;
    expect(count).toBe(1);
  });
});
