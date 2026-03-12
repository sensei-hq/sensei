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

  it("writes symbol-map.json with L0 entries", async () => {
    await reindexRepo(TMP);
    const map = JSON.parse(readFileSync(join(TMP, ".sensei/symbol-map.json"), "utf-8"));
    expect(Object.keys(map).length).toBeGreaterThan(0);
  });

  it("writes stack.md with detected stack", async () => {
    await reindexRepo(TMP);
    const stack = readFileSync(join(TMP, ".sensei/stack.md"), "utf-8");
    expect(stack).toContain("express");
  });

  it("writes shortcuts.md from package.json scripts", async () => {
    await reindexRepo(TMP);
    const shortcuts = readFileSync(join(TMP, ".sensei/shortcuts.md"), "utf-8");
    expect(shortcuts).toContain("bun src/index.ts");
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

  it("writes doc-index.json with new schema (files key)", async () => {
    await reindexRepo(TMP);
    const index = JSON.parse(readFileSync(join(TMP, ".sensei/doc-index.json"), "utf-8"));
    expect(index.files["README.md"]).toHaveProperty("mtime");
    expect(index.files["README.md"]).toHaveProperty("size");
  });

  it("returns IndexSummary with correct counts on first run", async () => {
    const summary = await reindexRepo(TMP);
    expect(summary.forced).toBe(true);
    expect(summary.added).toBeGreaterThan(0);
    expect(typeof summary.unchanged).toBe("number");
    expect(typeof summary.skipped).toBe("number");
    expect(summary.total).toBe(summary.added + summary.updated + summary.unchanged + summary.skipped);
  });

  it("incremental: unchanged file not re-added when run twice", async () => {
    await reindexRepo(TMP);
    const summary2 = await reindexRepo(TMP);
    // No git in TMP, so mtime fallback: file unchanged → unchanged count > 0
    expect(summary2.forced).toBe(false);
    expect(summary2.added).toBe(0);
  });

  it("incremental: new file detected on second run", async () => {
    await reindexRepo(TMP);
    writeFileSync(join(TMP, "src/new.ts"), "export function newFn(): void {}\n");
    const summary2 = await reindexRepo(TMP);
    expect(summary2.added).toBe(1);
    const map = JSON.parse(readFileSync(join(TMP, ".sensei/symbol-map.json"), "utf-8"));
    expect(map["src/new.ts"]).toBeDefined();
  });

  it("force: re-extracts all files regardless of fingerprints", async () => {
    await reindexRepo(TMP);
    const summary2 = await reindexRepo(TMP, { force: true });
    expect(summary2.forced).toBe(true);
    expect(summary2.added + summary2.updated).toBeGreaterThan(0);
  });

  it("writes traceability.json", async () => {
    await reindexRepo(TMP);
    expect(existsSync(join(TMP, ".sensei/traceability.json"))).toBe(true);
  });

  it("traceability: auto-detects src/ references in docs", async () => {
    mkdirSync(join(TMP, "docs"), { recursive: true });
    writeFileSync(join(TMP, "docs/design.md"), "See src/index.ts for details.");
    await reindexRepo(TMP);
    const t = JSON.parse(readFileSync(join(TMP, ".sensei/traceability.json"), "utf-8"));
    expect(t["docs/design.md"]).toContain("src/index.ts");
  });

  it("includes markdown files in symbol map", async () => {
    await reindexRepo(TMP);
    const map = JSON.parse(readFileSync(join(TMP, ".sensei/symbol-map.json"), "utf-8"));
    expect(map["README.md"]).toBeDefined();
    expect(map["README.md"].L0[0]).toContain("Test App");
  });

  it("indexes markdown files not in git diff on incremental run", async () => {
    // Simulate a repo that was indexed before markdown support was added:
    // doc-index exists but symbol map has no README.md entry
    mkdirSync(join(TMP, ".sensei"), { recursive: true });
    writeFileSync(join(TMP, ".sensei/doc-index.json"), JSON.stringify({ files: { "README.md": { mtime: Date.now(), size: 100 } } }));
    writeFileSync(join(TMP, ".sensei/symbol-map.json"), JSON.stringify({}));
    const summary = await reindexRepo(TMP);
    const map = JSON.parse(readFileSync(join(TMP, ".sensei/symbol-map.json"), "utf-8"));
    expect(map["README.md"]).toBeDefined();
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
