import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { mkdirSync, writeFileSync, rmSync, existsSync, readFileSync } from "fs";
import { join } from "path";
import yaml from "js-yaml";
import { reindexRepo, extractExports, extractMarkdownSymbols, extractHeuristic, mergeLlmSpec } from "./reindex.js";

const TMP = "/tmp/sensei-test-reindex";

function writePkg(overrides: Record<string, unknown> = {}) {
  writeFileSync(join(TMP, "package.json"), JSON.stringify({
    name: "test-app", version: "1.0.0",
    scripts: { dev: "bun src/index.ts", test: "bunx vitest" },
    dependencies: { express: "^4.0.0" },
    ...overrides,
  }));
}

beforeEach(() => {
  mkdirSync(join(TMP, "src"), { recursive: true });
  writePkg();
  writeFileSync(join(TMP, "src/index.ts"),
    `export const app = "app";\nexport function startServer(port: number): void { }\n`);
  writeFileSync(join(TMP, "README.md"), "# Test App\nA test application.");
});

afterEach(() => rmSync(TMP, { recursive: true, force: true }));

describe("reindexRepo", () => {
  it("creates .sensei directory", async () => {
    await reindexRepo(TMP);
    expect(existsSync(join(TMP, ".sensei"))).toBe(true);
  });

  it("does not write symbol-map.json (symbols go to DB)", async () => {
    await reindexRepo(TMP);
    expect(existsSync(join(TMP, ".sensei/symbol-map.json"))).toBe(false);
  });

  it("does not write stack.md (content goes to DB)", async () => {
    await reindexRepo(TMP);
    expect(existsSync(join(TMP, ".sensei/stack.md"))).toBe(false);
  });

  it("does not write shortcuts.md (content goes to DB)", async () => {
    await reindexRepo(TMP);
    expect(existsSync(join(TMP, ".sensei/shortcuts.md"))).toBe(false);
  });

  it("creates .sensei/llmspec.yaml template if missing", async () => {
    await reindexRepo(TMP);
    expect(existsSync(join(TMP, ".sensei/llmspec.yaml"))).toBe(true);
  });

  it("preserves semantic fields in existing .sensei/llmspec.yaml on re-index", async () => {
    mkdirSync(join(TMP, ".sensei"), { recursive: true });
    const existing = yaml.dump({
      project: "my-custom-project",
      version: "0.0.0",
      description: "Human-authored description that must not be overwritten.",
      stack: ["old-stack"],
      entry_points: [{ path: "src/index.ts", role: "human role" }],
      concepts: [{ name: "MyConcept", definition: "Important concept." }],
      patterns: [],
      api_surface: [],
      doc_layers: { design: "docs/", code: "src/", public: ["README.md"] },
      shortcuts: {},
    });
    writeFileSync(join(TMP, ".sensei/llmspec.yaml"), existing);
    await reindexRepo(TMP);
    const content = readFileSync(join(TMP, ".sensei/llmspec.yaml"), "utf-8");
    const parsed = yaml.load(content) as Record<string, unknown>;
    expect((parsed as any).description).toBe("Human-authored description that must not be overwritten.");
    expect((parsed as any).concepts[0].name).toBe("MyConcept");
  });

  it("updates structural fields in existing .sensei/llmspec.yaml on re-index", async () => {
    mkdirSync(join(TMP, ".sensei"), { recursive: true });
    const existing = yaml.dump({
      project: "my-custom-project",
      version: "0.0.0",
      description: "desc",
      stack: ["old-stack"],
      entry_points: [{ path: "src/index.ts", role: "human role" }],
      concepts: [],
      patterns: [],
      api_surface: [],
      doc_layers: { design: "docs/", code: "src/", public: ["README.md"] },
      shortcuts: {},
    });
    writeFileSync(join(TMP, ".sensei/llmspec.yaml"), existing);
    await reindexRepo(TMP);
    const content = readFileSync(join(TMP, ".sensei/llmspec.yaml"), "utf-8");
    const parsed = yaml.load(content) as Record<string, unknown>;
    expect(Array.isArray((parsed as any).stack)).toBe(true);
    expect((parsed as any).shortcuts).toHaveProperty("dev");
  });

  it("does not write doc-index.json (fingerprints go to DB)", async () => {
    await reindexRepo(TMP);
    expect(existsSync(join(TMP, ".sensei/doc-index.json"))).toBe(false);
  });

  it("returns IndexSummary with correct counts on first run", async () => {
    const summary = await reindexRepo(TMP);
    expect(summary.forced).toBe(true);
    expect(summary.added).toBeGreaterThan(0);
    expect(typeof summary.unchanged).toBe("number");
    expect(typeof summary.skipped).toBe("number");
    expect(summary.total).toBe(summary.added + summary.updated + summary.unchanged + summary.skipped);
  });

  it("incremental: second run is forced=true when no DB (no persistent state)", async () => {
    // Without DB, there's no persistent state between runs, so each run is forced
    const summary1 = await reindexRepo(TMP);
    expect(summary1.forced).toBe(true);
    const summary2 = await reindexRepo(TMP);
    // Without DB or legacy files, existingDocIndex is null → force=true
    expect(summary2.forced).toBe(true);
  });

  it("incremental via legacy files: is not forced when legacy files exist", async () => {
    // Seed legacy files to simulate pre-existing state
    mkdirSync(join(TMP, ".sensei"), { recursive: true });
    // Use far-future mtime so files appear unchanged
    const futureTime = Date.now() + 10_000_000;
    const idxStats = require("fs").statSync(join(TMP, "src/index.ts"));
    const readmeStats = require("fs").statSync(join(TMP, "README.md"));
    const pkgStats = require("fs").statSync(join(TMP, "package.json"));
    writeFileSync(join(TMP, ".sensei/doc-index.json"), JSON.stringify({
      files: {
        "src/index.ts": { mtime: idxStats.mtimeMs, size: idxStats.size },
        "README.md":    { mtime: readmeStats.mtimeMs, size: readmeStats.size },
        "package.json": { mtime: pkgStats.mtimeMs, size: pkgStats.size },
      }
    }));
    writeFileSync(join(TMP, ".sensei/symbol-map.json"), JSON.stringify({
      "src/index.ts": { L0: ["export const app"], L1: [], L2: [] },
      "README.md": { L0: ["Test App"], L1: [], L2: [] },
    }));
    const summary2 = await reindexRepo(TMP);
    expect(summary2.forced).toBe(false);
  });

  it("incremental: new file detected on second run (via legacy fallback)", async () => {
    mkdirSync(join(TMP, ".sensei"), { recursive: true });
    const idxStats = require("fs").statSync(join(TMP, "src/index.ts"));
    const readmeStats = require("fs").statSync(join(TMP, "README.md"));
    const pkgStats = require("fs").statSync(join(TMP, "package.json"));
    writeFileSync(join(TMP, ".sensei/doc-index.json"), JSON.stringify({
      files: {
        "src/index.ts": { mtime: idxStats.mtimeMs, size: idxStats.size },
        "README.md":    { mtime: readmeStats.mtimeMs, size: readmeStats.size },
        "package.json": { mtime: pkgStats.mtimeMs, size: pkgStats.size },
      }
    }));
    writeFileSync(join(TMP, ".sensei/symbol-map.json"), JSON.stringify({
      "src/index.ts": { L0: ["export const app"], L1: [], L2: [] },
      "README.md": { L0: ["Test App"], L1: [], L2: [] },
    }));
    writeFileSync(join(TMP, "src/new.ts"), "export function newFn(): void {}\n");
    const summary2 = await reindexRepo(TMP);
    expect(summary2.added).toBeGreaterThanOrEqual(1);
    expect(summary2.forced).toBe(false);
  });

  it("force: re-extracts all files regardless of fingerprints", async () => {
    await reindexRepo(TMP);
    const summary2 = await reindexRepo(TMP, { force: true });
    expect(summary2.forced).toBe(true);
    expect(summary2.added + summary2.updated).toBeGreaterThan(0);
  });

  it("does not write traceability.json (traceability goes to DB)", async () => {
    await reindexRepo(TMP);
    expect(existsSync(join(TMP, ".sensei/traceability.json"))).toBe(false);
  });

  it("traceability: IndexSummary reflects correct counts when docs reference src/", async () => {
    mkdirSync(join(TMP, "docs"), { recursive: true });
    writeFileSync(join(TMP, "docs/design.md"), "See src/index.ts for details.");
    const summary = await reindexRepo(TMP);
    // docs/design.md is added as a new file
    expect(summary.added).toBeGreaterThan(0);
  });

  it("includes markdown files in the symbol extraction (verified via IndexSummary)", async () => {
    const summary = await reindexRepo(TMP);
    // README.md is a doc file and gets extracted as a new entry
    expect(summary.added).toBeGreaterThan(0);
    expect(summary.total).toBeGreaterThan(0);
  });

  it("indexes markdown files not in symbol map on incremental run (legacy fallback)", async () => {
    // Simulate a repo that was indexed before markdown support was added:
    // doc-index exists but symbol map has no README.md entry
    mkdirSync(join(TMP, ".sensei"), { recursive: true });
    const stats = require("fs").statSync(join(TMP, "src/index.ts"));
    writeFileSync(join(TMP, ".sensei/doc-index.json"), JSON.stringify({ files: { "README.md": { mtime: Date.now(), size: 100 } } }));
    writeFileSync(join(TMP, ".sensei/symbol-map.json"), JSON.stringify({}));
    const summary = await reindexRepo(TMP);
    // README.md was not in symbol map, so it gets added
    expect(summary.added).toBeGreaterThan(0);
  });
});

describe("extractExports", () => {
  it("L1 is L0 prefixed with //", () => {
    const { L0, L1 } = extractExports("export function foo(a: string): void {}");
    expect(L0).toEqual(["export function foo(a: string): void"]);
    expect(L1[0]).toBe("// export function foo(a: string): void");
  });

  it("L1 includes JSDoc description before signature", () => {
    const src = `/** Does the thing */\nexport function foo(): void {}`;
    const { L1 } = extractExports(src);
    expect(L1[0]).toContain("Does the thing");
    expect(L1[0]).toContain("// export function foo");
  });

  it("every L0 entry has a matching L1 entry", () => {
    const src = `export function a(): void {}\nexport function b(): void {}`;
    const { L0, L1 } = extractExports(src);
    expect(L1.length).toBe(L0.length);
  });
});

describe("extractMarkdownSymbols", () => {
  it("L0 is title — summary", () => {
    const { L0 } = extractMarkdownSymbols("# My Doc\n\nThis doc explains X.", "my-doc.md");
    expect(L0[0]).toBe("My Doc — This doc explains X.");
  });

  it("L1 is L0 prefixed with //", () => {
    const { L0, L1 } = extractMarkdownSymbols("# My Doc\n\nThis doc explains X.", "my-doc.md");
    expect(L1[0]).toBe(`// ${L0[0]}`);
  });

  it("L1 includes section headings prefixed with //", () => {
    const { L1 } = extractMarkdownSymbols("# Doc\n\n## Setup\n\n## Usage", "doc.md");
    expect(L1).toContain("// ## Setup");
    expect(L1).toContain("// ## Usage");
  });
});

describe("extractHeuristic", () => {
  it("extracts non-exported TS functions", () => {
    const { L0 } = extractHeuristic("function helper(x: number): string {}", "util.ts");
    expect(L0[0]).toContain("function helper");
  });

  it("extracts Python defs and classes", () => {
    const { L0 } = extractHeuristic("def process(data):\n    pass\nclass Handler:\n    pass", "handler.py");
    expect(L0).toContain("def process(data):");
    expect(L0).toContain("class Handler:");
  });

  it("extracts Go functions", () => {
    const { L0 } = extractHeuristic("func ProcessRequest(w http.ResponseWriter, r *http.Request) {}", "server.go");
    expect(L0[0]).toContain("func ProcessRequest");
  });

  it("extracts Rust functions", () => {
    const { L0 } = extractHeuristic("pub fn handle(req: Request) -> Response {}", "handler.rs");
    expect(L0[0]).toContain("fn handle");
  });

  it("L1 is L0 prefixed with //", () => {
    const { L0, L1 } = extractHeuristic("function foo(): void {}", "util.ts");
    expect(L1[0]).toBe(`// ${L0[0]}`);
  });
});

describe("mergeLlmSpec", () => {
  const structuralBase = {
    stack: ["typescript"],
    shortcuts: { test: "bunx vitest" },
    version: "0.2.0",
    entryPoints: [{ path: "src/cli.ts", inferredRole: "CLI binary" }],
    llmsTxtSections: [{ name: "Entry Points", sources: ["src/cli.ts"] }],
  };

  it("creates llmspec.yaml from scratch if missing", async () => {
    await mergeLlmSpec(TMP, structuralBase);
    expect(existsSync(join(TMP, ".sensei/llmspec.yaml"))).toBe(true);
  });

  it("written-from-scratch file has correct stack", async () => {
    await mergeLlmSpec(TMP, structuralBase);
    const parsed = yaml.load(readFileSync(join(TMP, ".sensei/llmspec.yaml"), "utf-8")) as any;
    expect(parsed.stack).toEqual(["typescript"]);
  });

  it("preserves description when updating existing file", async () => {
    mkdirSync(join(TMP, ".sensei"), { recursive: true });
    writeFileSync(join(TMP, ".sensei/llmspec.yaml"), yaml.dump({
      project: "test",
      version: "0.1.0",
      description: "Keep me.",
      stack: ["old"],
      entry_points: [],
      concepts: [],
      patterns: [],
      api_surface: [],
      doc_layers: { design: "docs/", code: "src/", public: [] },
      shortcuts: {},
    }));
    await mergeLlmSpec(TMP, structuralBase);
    const parsed = yaml.load(readFileSync(join(TMP, ".sensei/llmspec.yaml"), "utf-8")) as any;
    expect(parsed.description).toBe("Keep me.");
  });

  it("updates stack on existing file", async () => {
    mkdirSync(join(TMP, ".sensei"), { recursive: true });
    writeFileSync(join(TMP, ".sensei/llmspec.yaml"), yaml.dump({
      project: "test", version: "0.0.0", description: "d",
      stack: ["old-stack"], entry_points: [],
      concepts: [], patterns: [], api_surface: [],
      doc_layers: { design: "docs/", code: "src/", public: [] }, shortcuts: {},
    }));
    await mergeLlmSpec(TMP, { ...structuralBase, stack: ["typescript", "bun"] });
    const parsed = yaml.load(readFileSync(join(TMP, ".sensei/llmspec.yaml"), "utf-8")) as any;
    expect(parsed.stack).toContain("typescript");
    expect(parsed.stack).toContain("bun");
    expect(parsed.stack).not.toContain("old-stack");
  });

  it("preserves existing entry_point role when path already present", async () => {
    mkdirSync(join(TMP, ".sensei"), { recursive: true });
    writeFileSync(join(TMP, ".sensei/llmspec.yaml"), yaml.dump({
      project: "test", version: "0.0.0", description: "d", stack: [],
      entry_points: [{ path: "src/cli.ts", role: "Human-assigned role" }],
      concepts: [], patterns: [], api_surface: [],
      doc_layers: { design: "docs/", code: "src/", public: [] }, shortcuts: {},
    }));
    await mergeLlmSpec(TMP, structuralBase);
    const parsed = yaml.load(readFileSync(join(TMP, ".sensei/llmspec.yaml"), "utf-8")) as any;
    expect(parsed.entry_points[0].role).toBe("Human-assigned role");
  });

  it("adds new entry_point with inferredRole", async () => {
    mkdirSync(join(TMP, ".sensei"), { recursive: true });
    writeFileSync(join(TMP, ".sensei/llmspec.yaml"), yaml.dump({
      project: "test", version: "0.0.0", description: "d", stack: [],
      entry_points: [],
      concepts: [], patterns: [], api_surface: [],
      doc_layers: { design: "docs/", code: "src/", public: [] }, shortcuts: {},
    }));
    await mergeLlmSpec(TMP, structuralBase);
    const parsed = yaml.load(readFileSync(join(TMP, ".sensei/llmspec.yaml"), "utf-8")) as any;
    expect(parsed.entry_points.some((e: any) => e.path === "src/cli.ts")).toBe(true);
    expect(parsed.entry_points.find((e: any) => e.path === "src/cli.ts").role).toBe("CLI binary");
  });

  it("removes entry_point paths no longer in candidates", async () => {
    mkdirSync(join(TMP, ".sensei"), { recursive: true });
    writeFileSync(join(TMP, ".sensei/llmspec.yaml"), yaml.dump({
      project: "test", version: "0.0.0", description: "d", stack: [],
      entry_points: [
        { path: "src/cli.ts", role: "CLI" },
        { path: "src/old.ts", role: "Old entry, now deleted" },
      ],
      concepts: [], patterns: [], api_surface: [],
      doc_layers: { design: "docs/", code: "src/", public: [] }, shortcuts: {},
    }));
    await mergeLlmSpec(TMP, structuralBase);
    const parsed = yaml.load(readFileSync(join(TMP, ".sensei/llmspec.yaml"), "utf-8")) as any;
    expect(parsed.entry_points.some((e: any) => e.path === "src/old.ts")).toBe(false);
  });

  it("updates llms_txt field", async () => {
    mkdirSync(join(TMP, ".sensei"), { recursive: true });
    writeFileSync(join(TMP, ".sensei/llmspec.yaml"), yaml.dump({
      project: "test", version: "0.0.0", description: "d", stack: [],
      entry_points: [], concepts: [], patterns: [], api_surface: [],
      doc_layers: { design: "docs/", code: "src/", public: [] }, shortcuts: {},
    }));
    await mergeLlmSpec(TMP, structuralBase);
    const parsed = yaml.load(readFileSync(join(TMP, ".sensei/llmspec.yaml"), "utf-8")) as any;
    expect(Array.isArray(parsed.llms_txt)).toBe(true);
    expect(parsed.llms_txt[0].name).toBe("Entry Points");
  });
});
