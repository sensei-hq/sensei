import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { mkdirSync, writeFileSync, rmSync, readFileSync, existsSync } from "fs";
import { join } from "path";
import type { SymbolMap, LlmSpec } from "@sensei/shared";
import { buildSections, renderSection, parseLlmsTxt, generateLlmsTxt } from "./llms-txt.js";

const TMP = "/tmp/sensei-test-llms-txt";

const SAMPLE_MAP: SymbolMap = {
  "packages/tools/src/tools/reindex.ts": {
    L0: [
      "export async function reindexRepo(repoPath: string, options?: { force?: boolean }): Promise<IndexSummary>",
      "export function extractExports(content: string): { L0: string[]; L1: string[]; L2: string[] }",
      "export function extractMarkdownSymbols(content: string, filename: string): { L0: string[]; L1: string[]; L2: string[] }",
      "export function extractHeuristic(content: string, filename: string): { L0: string[]; L1: string[]; L2: string[] }",
      "export interface IndexSummary",
      "export async function mergeLlmSpec(repoPath: string, structural: object): Promise<void>",
    ],
    L1: [], L2: [],
  },
  "packages/tools/src/tools/query.ts": {
    L0: [
      "export async function getLlmSpec(repoPath: string, section?: string): Promise<string>",
      "export async function getFileContext(repoPath: string, filePath: string, level: ResolutionLevel): Promise<string>",
    ],
    L1: [], L2: [],
  },
  "packages/cli/src/cli.ts": {
    L0: ["export async function main(): Promise<void>"],
    L1: [], L2: [],
  },
};

beforeEach(() => {
  mkdirSync(join(TMP, ".sensei"), { recursive: true });
});
afterEach(() => rmSync(TMP, { recursive: true, force: true }));

describe("buildSections", () => {
  it("creates an Entry Points section with only the provided entry-point paths", () => {
    const sections = buildSections(SAMPLE_MAP, ["packages/cli/src/cli.ts"]);
    const ep = sections.find(s => s.name === "Entry Points");
    expect(ep).toBeDefined();
    expect(ep!.sources).toContain("packages/cli/src/cli.ts");
  });

  it("does not include non-entry-point files in the Entry Points section", () => {
    const sections = buildSections(SAMPLE_MAP, ["packages/cli/src/cli.ts"]);
    const ep = sections.find(s => s.name === "Entry Points")!;
    expect(ep.sources).not.toContain("packages/tools/src/tools/reindex.ts");
  });

  it("groups remaining files by parent directory", () => {
    const sections = buildSections(SAMPLE_MAP, ["packages/cli/src/cli.ts"]);
    const toolsSection = sections.find(s => s.name === "packages/tools/src/tools");
    expect(toolsSection).toBeDefined();
    expect(toolsSection!.sources).toContain("packages/tools/src/tools/reindex.ts");
    expect(toolsSection!.sources).toContain("packages/tools/src/tools/query.ts");
  });

  it("filters entry-point paths not present in symbolMap", () => {
    const sections = buildSections(SAMPLE_MAP, ["packages/cli/src/cli.ts", "nonexistent/file.ts"]);
    const ep = sections.find(s => s.name === "Entry Points")!;
    expect(ep.sources).not.toContain("nonexistent/file.ts");
  });

  it("entry point file is not also listed in its directory section", () => {
    const sections = buildSections(SAMPLE_MAP, ["packages/cli/src/cli.ts"]);
    const cliSection = sections.find(s => s.name === "packages/cli/src");
    if (cliSection) {
      expect(cliSection.sources).not.toContain("packages/cli/src/cli.ts");
    }
  });
});

describe("renderSection", () => {
  it("renders a markdown H2 heading", () => {
    const out = renderSection("Entry Points", ["packages/cli/src/cli.ts"], SAMPLE_MAP);
    expect(out).toContain("## Entry Points");
  });

  it("renders each source as a markdown list item with link", () => {
    const out = renderSection("packages/tools/src/tools", ["packages/tools/src/tools/reindex.ts"], SAMPLE_MAP);
    expect(out).toContain("[reindex.ts](packages/tools/src/tools/reindex.ts)");
  });

  it("appends up to 5 L0 signatures stripped of export keywords", () => {
    const out = renderSection("packages/tools/src/tools", ["packages/tools/src/tools/reindex.ts"], SAMPLE_MAP);
    expect(out).toContain("reindexRepo");
    expect(out).toContain("extractExports");
  });

  it("limits exported names to 5 per file", () => {
    const out = renderSection("packages/tools/src/tools", ["packages/tools/src/tools/reindex.ts"], SAMPLE_MAP);
    const namesLine = out.split("\n").find(l => l.includes("reindex.ts"));
    const afterColon = namesLine?.split(":").slice(1).join(":") ?? "";
    const commaCount = (afterColon.match(/,/g) ?? []).length;
    expect(commaCount).toBeLessThanOrEqual(4);
  });

  it("strips 'export async function' prefix from L0 entries", () => {
    const out = renderSection("Entry Points", ["packages/cli/src/cli.ts"], SAMPLE_MAP);
    expect(out).not.toContain("export async function");
    expect(out).toContain("main");
  });

  it("handles file with no L0 entries gracefully", () => {
    const emptyMap: SymbolMap = { "src/empty.ts": { L0: [], L1: [], L2: [] } };
    const out = renderSection("src", ["src/empty.ts"], emptyMap);
    expect(out).toContain("[empty.ts](src/empty.ts)");
  });
});

describe("parseLlmsTxt", () => {
  it("returns empty object for empty string", () => {
    expect(parseLlmsTxt("")).toEqual({});
  });

  it("splits on ## boundaries", () => {
    const content = `# My Project\n\n## Entry Points\n- foo.ts\n\n## packages/src\n- bar.ts\n`;
    const parsed = parseLlmsTxt(content);
    expect(Object.keys(parsed)).toContain("Entry Points");
    expect(Object.keys(parsed)).toContain("packages/src");
  });

  it("preserves section body content", () => {
    const content = `# My Project\n\n## Entry Points\n- [foo.ts](foo.ts): bar\n`;
    const parsed = parseLlmsTxt(content);
    expect(parsed["Entry Points"]).toContain("foo.ts");
  });

  it("does not include the preamble as a section", () => {
    const content = `# My Project\n> desc\n\n## Entry Points\n- foo.ts\n`;
    const parsed = parseLlmsTxt(content);
    expect(Object.keys(parsed)).not.toContain("My Project");
    expect(Object.keys(parsed)).not.toContain("");
  });
});

describe("generateLlmsTxt", () => {
  const makeSpec = (overrides: Partial<LlmSpec> = {}): LlmSpec => ({
    project: "test-project",
    version: "0.0.1",
    description: "A test project for AI agents.",
    stack: ["typescript"],
    entry_points: [{ path: "packages/cli/src/cli.ts", role: "CLI binary" }],
    concepts: [],
    patterns: [],
    api_surface: [],
    doc_layers: { design: "docs/", code: "src/", public: ["README.md"] },
    shortcuts: {},
    ...overrides,
  });

  it("writes .sensei/llms.txt", async () => {
    await generateLlmsTxt(TMP, makeSpec(), SAMPLE_MAP, new Set());
    expect(existsSync(join(TMP, ".sensei/llms.txt"))).toBe(true);
  });

  it("generated llms.txt starts with # project name", async () => {
    await generateLlmsTxt(TMP, makeSpec(), SAMPLE_MAP, new Set());
    const content = readFileSync(join(TMP, ".sensei/llms.txt"), "utf-8");
    expect(content.startsWith("# test-project")).toBe(true);
  });

  it("generated llms.txt includes project description as blockquote", async () => {
    await generateLlmsTxt(TMP, makeSpec(), SAMPLE_MAP, new Set());
    const content = readFileSync(join(TMP, ".sensei/llms.txt"), "utf-8");
    expect(content).toContain("> A test project for AI agents.");
  });

  it("returns LlmsTxtSection[] matching sections in the file", async () => {
    const result = await generateLlmsTxt(TMP, makeSpec(), SAMPLE_MAP, new Set());
    expect(Array.isArray(result.sections)).toBe(true);
    expect(result.sections.some(s => s.name === "Entry Points")).toBe(true);
  });

  it("ends with '> Generated by sensei'", async () => {
    await generateLlmsTxt(TMP, makeSpec(), SAMPLE_MAP, new Set());
    const content = readFileSync(join(TMP, ".sensei/llms.txt"), "utf-8");
    expect(content.trimEnd().endsWith("> Generated by sensei")).toBe(true);
  });

  it("incremental: reuses existing section when none of its sources changed", async () => {
    writeFileSync(join(TMP, ".sensei/llms.txt"),
      `# test-project\n> desc\n\n## packages/tools/src/tools\n- CUSTOM_CONTENT\n\n> Generated by sensei\n`);
    const changedFiles = new Set<string>(["packages/cli/src/cli.ts"]);
    await generateLlmsTxt(TMP, makeSpec(), SAMPLE_MAP, changedFiles);
    const content = readFileSync(join(TMP, ".sensei/llms.txt"), "utf-8");
    expect(content).toContain("CUSTOM_CONTENT");
  });

  it("incremental: regenerates section when a source file changed", async () => {
    writeFileSync(join(TMP, ".sensei/llms.txt"),
      `# test-project\n> desc\n\n## packages/tools/src/tools\n- STALE_CONTENT\n\n> Generated by sensei\n`);
    const changedFiles = new Set<string>(["packages/tools/src/tools/reindex.ts"]);
    await generateLlmsTxt(TMP, makeSpec(), SAMPLE_MAP, changedFiles);
    const content = readFileSync(join(TMP, ".sensei/llms.txt"), "utf-8");
    expect(content).not.toContain("STALE_CONTENT");
    expect(content).toContain("reindexRepo");
  });
});
