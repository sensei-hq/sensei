# llms.txt Traceability + llmspec Auto-Update — Implementation Plan

> **For agentic workers:** REQUIRED: Use `superpowers:subagent-driven-development` (if subagents available) or `superpowers:executing-plans` to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the stub `generateLlmsTxt` with a structured llmstxt.org-format generator derived from `symbol-map.json`, add per-language entry-point inference, and make `reindexRepo` perform surgical merges of `llmspec.yaml` so structural fields stay fresh while human-authored semantic fields are never touched.

**Architecture:** Three focused modules are added to `packages/tools/src/tools/`: `entry-point-adapters.ts` (language-specific entry-point discovery), `llms-txt.ts` (section building, rendering, incremental generation), and a `mergeLlmSpec` function in `reindex.ts` that replaces the create-only guard. The `LlmSpec` interface in `packages/shared` gains a `llms_txt?: LlmsTxtSection[]` field and a new `LlmsTxtSection` interface. All new code is covered by unit tests written first.

**Tech Stack:** Bun, TypeScript, Vitest, `js-yaml`, `fast-glob` (already in deps), Node.js `fs/promises`.

---

## Chunk 1: Foundation — Types, Entry-Point Adapters

### Task 1: Extend `LlmSpec` with `LlmsTxtSection`

**Files:**
- Modify: `/Users/Jerry/Developer/sensei/packages/shared/src/types.ts`

- [ ] **Step 1: Add the `LlmsTxtSection` interface and `llms_txt` field**

In `/Users/Jerry/Developer/sensei/packages/shared/src/types.ts`, after the `SymbolMap` type alias (line 25) add:

```typescript
export interface LlmsTxtSection {
  name: string;
  sources: string[]; // repo-relative paths
}
```

Then in the `LlmSpec` interface (lines 3-14), add one field after `shortcuts`:

```typescript
  llms_txt?: LlmsTxtSection[];
```

The complete modified interface should be:

```typescript
export interface LlmSpec {
  project: string;
  version: string;
  description: string;
  stack: string[];
  entry_points: Array<{ path: string; role: string }>;
  concepts: Array<{ name: string; definition: string }>;
  patterns: Array<{ name: string; files: string; convention: string }>;
  api_surface: Array<{ name: string; path: string; io: string; flow: string }>;
  doc_layers: { design: string; code: string; public: string[] };
  shortcuts: Record<string, string>;
  llms_txt?: LlmsTxtSection[];
}
```

- [ ] **Step 2: Verify types compile**

```bash
cd /Users/Jerry/Developer/sensei/packages/shared && bunx tsc --noEmit
```

Expected: no output (clean).

- [ ] **Step 3: Commit**

```bash
cd /Users/Jerry/Developer/sensei
git add packages/shared/src/types.ts
git commit -m "feat(shared): add LlmsTxtSection interface and llms_txt field to LlmSpec"
```

---

### Task 2: Create `entry-point-adapters.spec.ts` (failing tests first)

**Files:**
- Create: `/Users/Jerry/Developer/sensei/packages/tools/src/tools/entry-point-adapters.spec.ts`

- [ ] **Step 1: Write the failing test file**

Create `/Users/Jerry/Developer/sensei/packages/tools/src/tools/entry-point-adapters.spec.ts`:

```typescript
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
      `[project]\
name = "my-pkg"\
\
[project.scripts]\
my-cli = "my_package.cli:main"\
`);
    mkdirSync(join(TMP, "my_package"), { recursive: true });
    writeFileSync(join(TMP, "my_package/cli.py"), "# cli");
    const results = await inferEntryPoints(TMP);
    expect(results.some(r => r.path === "my_package/cli.py")).toBe(true);
  });

  it("uses script name in inferredRole", async () => {
    writeFileSync(join(TMP, "pyproject.toml"),
      `[project.scripts]\
my-cli = "my_package.cli:main"\
`);
    mkdirSync(join(TMP, "my_package"), { recursive: true });
    writeFileSync(join(TMP, "my_package/cli.py"), "# cli");
    const results = await inferEntryPoints(TMP);
    const entry = results.find(r => r.path === "my_package/cli.py");
    expect(entry?.inferredRole).toContain("my-cli");
  });

  it("skips script if source file does not exist", async () => {
    writeFileSync(join(TMP, "pyproject.toml"),
      `[project.scripts]\
ghost = "ghost.module:main"\
`);
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
      `[package]\
name = "my-crate"\
\
[[bin]]\
name = "my-tool"\
path = "src/bin/main.rs"\
`);
    mkdirSync(join(TMP, "src/bin"), { recursive: true });
    writeFileSync(join(TMP, "src/bin/main.rs"), "fn main() {}");
    const results = await inferEntryPoints(TMP);
    expect(results.some(r => r.path === "src/bin/main.rs")).toBe(true);
  });

  it("infers role from bin name", async () => {
    writeFileSync(join(TMP, "Cargo.toml"),
      `[[bin]]\
name = "my-tool"\
path = "src/bin/main.rs"\
`);
    mkdirSync(join(TMP, "src/bin"), { recursive: true });
    writeFileSync(join(TMP, "src/bin/main.rs"), "fn main() {}");
    const results = await inferEntryPoints(TMP);
    const entry = results.find(r => r.path === "src/bin/main.rs");
    expect(entry?.inferredRole).toContain("my-tool");
  });
});

describe("inferEntryPoints — deduplication", () => {
  it("deduplicates candidates with the same path", async () => {
    // root package.json bin + workspace package.json bin both pointing to same source
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
```

- [ ] **Step 2: Run tests to confirm they fail (module not found)**

```bash
cd /Users/Jerry/Developer/sensei/packages/tools && bunx vitest run src/tools/entry-point-adapters.spec.ts 2>&1 | tail -15
```

Expected: FAIL with `Cannot find module './entry-point-adapters.js'` or similar import error.

---

### Task 3: Implement `entry-point-adapters.ts`

**Files:**
- Create: `/Users/Jerry/Developer/sensei/packages/tools/src/tools/entry-point-adapters.ts`

- [ ] **Step 1: Create the implementation**

Create `/Users/Jerry/Developer/sensei/packages/tools/src/tools/entry-point-adapters.ts`:

```typescript
import { readFile } from "fs/promises";
import { existsSync } from "fs";
import { join, dirname } from "path";
import fg from "fast-glob";

export interface EntryPointCandidate {
  path: string;        // repo-relative source path
  inferredRole: string;
}

// ─── Public API ──────────────────────────────────────────────────────────────

export async function inferEntryPoints(repoPath: string): Promise<EntryPointCandidate[]> {
  const [fromJs, fromPy, fromGo, fromRust] = await Promise.all([
    inferFromPackageJson(repoPath),
    inferFromPyprojectToml(repoPath),
    inferFromGoConvention(repoPath),
    inferFromCargoToml(repoPath),
  ]);

  const all = [...fromJs, ...fromPy, ...fromGo, ...fromRust];
  // Deduplicate by path — first occurrence wins
  const seen = new Set<string>();
  return all.filter(c => {
    if (seen.has(c.path)) return false;
    seen.add(c.path);
    return true;
  });
}

// ─── TypeScript / JavaScript ─────────────────────────────────────────────────

async function inferFromPackageJson(repoPath: string): Promise<EntryPointCandidate[]> {
  const results: EntryPointCandidate[] = [];

  // Root package
  const rootPkg = await readJsonFile(join(repoPath, "package.json"));
  if (rootPkg) results.push(...extractFromPkg(repoPath, rootPkg, ""));

  // Workspace packages
  const workspaceGlobs: string[] = [];
  if (Array.isArray(rootPkg?.workspaces)) {
    for (const ws of rootPkg.workspaces as string[]) {
      workspaceGlobs.push(`${ws}/package.json`);
    }
  }
  if (workspaceGlobs.length > 0) {
    const wsPkgPaths = await fg(workspaceGlobs, { cwd: repoPath, ignore: ["**/node_modules/**"] });
    for (const relPkgPath of wsPkgPaths) {
      const pkg = await readJsonFile(join(repoPath, relPkgPath));
      if (pkg) {
        const pkgDir = dirname(relPkgPath); // e.g. "packages/cli"
        results.push(...extractFromPkg(repoPath, pkg, pkgDir));
      }
    }
  }

  return results;
}

function extractFromPkg(
  repoPath: string,
  pkg: Record<string, unknown>,
  pkgDir: string,
): EntryPointCandidate[] {
  const results: EntryPointCandidate[] = [];

  // bin field
  const bin = pkg.bin;
  if (bin && typeof bin === "object") {
    for (const [key, distPath] of Object.entries(bin as Record<string, string>)) {
      const resolved = resolveDistToSrc(repoPath, pkgDir, distPath);
      results.push({ path: resolved, inferredRole: `${key} CLI binary` });
    }
  }

  // main field
  if (typeof pkg.main === "string") {
    const resolved = resolveDistToSrc(repoPath, pkgDir, pkg.main);
    results.push({ path: resolved, inferredRole: "main entry" });
  }

  return results;
}

/**
 * Heuristic: dist/foo.js → src/foo.ts (if source exists), else keep dist path.
 * Handles both package-relative and repo-relative dist paths.
 */
function resolveDistToSrc(repoPath: string, pkgDir: string, distRelPath: string): string {
  // Strip leading ./
  const stripped = distRelPath.replace(/^\.\//, "");
  // Build repo-relative dist path
  const repoRelDist = pkgDir ? `${pkgDir}/${stripped}` : stripped;

  // Try to map dist/X.js → src/X.ts
  const srcCandidate = repoRelDist.replace(/\bdist\//, "src/").replace(/\.js$/, ".ts");
  if (existsSync(join(repoPath, srcCandidate))) return srcCandidate;

  // Also try with .tsx
  const tsxCandidate = repoRelDist.replace(/\bdist\//, "src/").replace(/\.js$/, ".tsx");
  if (existsSync(join(repoPath, tsxCandidate))) return tsxCandidate;

  return repoRelDist;
}

// ─── Python ──────────────────────────────────────────────────────────────────

async function inferFromPyprojectToml(repoPath: string): Promise<EntryPointCandidate[]> {
  const tomlPath = join(repoPath, "pyproject.toml");
  if (!existsSync(tomlPath)) return [];

  const content = await readFile(tomlPath, "utf-8");
  const results: EntryPointCandidate[] = [];

  let inScripts = false;
  for (const line of content.split("\
")) {
    const trimmed = line.trim();
    if (trimmed === "[project.scripts]") { inScripts = true; continue; }
    if (inScripts && trimmed.startsWith("[")) { inScripts = false; continue; }
    if (!inScripts) continue;

    // e.g.  my-cli = "my_package.cli:main"
    const m = /^([\w-]+)\s*=\s*"([\w.]+):([\w]+)"/.exec(trimmed);
    if (!m) continue;
    const [, scriptName, modulePath] = m;
    // "my_package.cli" → "my_package/cli.py"
    const filePath = modulePath.replace(/\./g, "/") + ".py";
    if (!existsSync(join(repoPath, filePath))) continue;
    results.push({ path: filePath, inferredRole: `${scriptName} entry point` });
  }

  return results;
}

// ─── Go ──────────────────────────────────────────────────────────────────────

async function inferFromGoConvention(repoPath: string): Promise<EntryPointCandidate[]> {
  if (!existsSync(join(repoPath, "go.mod"))) return [];

  const mains = await fg(["cmd/*/main.go"], { cwd: repoPath });
  return mains.map(p => {
    // "cmd/server/main.go" → role "server binary"
    const parts = p.split("/");
    const binName = parts[1] ?? "main";
    return { path: p, inferredRole: `${binName} binary` };
  });
}

// ─── Rust ────────────────────────────────────────────────────────────────────

async function inferFromCargoToml(repoPath: string): Promise<EntryPointCandidate[]> {
  const cargoPath = join(repoPath, "Cargo.toml");
  if (!existsSync(cargoPath)) return [];

  const content = await readFile(cargoPath, "utf-8");
  const results: EntryPointCandidate[] = [];

  let inBin = false;
  let name: string | null = null;
  let path: string | null = null;

  const flush = () => {
    if (path && existsSync(join(repoPath, path))) {
      results.push({ path, inferredRole: `${name ?? "binary"} binary` });
    }
    name = null;
    path = null;
  };

  for (const line of content.split("\
")) {
    const trimmed = line.trim();
    if (trimmed === "[[bin]]") {
      flush();
      inBin = true;
      continue;
    }
    if (inBin && trimmed.startsWith("[[")) {
      flush();
      inBin = false;
      continue;
    }
    if (!inBin) continue;

    const namM = /^name\s*=\s*"([^"]+)"/.exec(trimmed);
    if (namM) { name = namM[1]; continue; }

    const pathM = /^path\s*=\s*"([^"]+)"/.exec(trimmed);
    if (pathM) { path = pathM[1]; }
  }
  flush();

  return results;
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

async function readJsonFile(absPath: string): Promise<Record<string, unknown> | null> {
  if (!existsSync(absPath)) return null;
  try {
    return JSON.parse(await readFile(absPath, "utf-8"));
  } catch {
    return null;
  }
}
```

- [ ] **Step 2: Run the adapter tests**

```bash
cd /Users/Jerry/Developer/sensei/packages/tools && bunx vitest run src/tools/entry-point-adapters.spec.ts 2>&1 | tail -20
```

Expected: all tests pass, output contains `✓ entry-point-adapters.spec.ts`.

- [ ] **Step 3: Commit**

```bash
cd /Users/Jerry/Developer/sensei
git add packages/tools/src/tools/entry-point-adapters.ts packages/tools/src/tools/entry-point-adapters.spec.ts
git commit -m "feat(tools): add per-language entry-point inference adapters"
```

---

## Chunk 2: Generator — `llms-txt.ts`

### Task 4: Create `llms-txt.spec.ts` (failing tests first)

**Files:**
- Create: `/Users/Jerry/Developer/sensei/packages/tools/src/tools/llms-txt.spec.ts`

- [ ] **Step 1: Write the failing test file**

Create `/Users/Jerry/Developer/sensei/packages/tools/src/tools/llms-txt.spec.ts`:

```typescript
import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { mkdirSync, writeFileSync, rmSync, readFileSync, existsSync } from "fs";
import { join } from "path";
import type { SymbolMap } from "@sensei/shared";
import type { LlmSpec } from "@sensei/shared";
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

// ─── buildSections ────────────────────────────────────────────────────────────

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
    // cli.ts is in entry points — it should not appear in a directory section
    if (cliSection) {
      expect(cliSection.sources).not.toContain("packages/cli/src/cli.ts");
    }
  });
});

// ─── renderSection ────────────────────────────────────────────────────────────

describe("renderSection", () => {
  it("renders a markdown H2 heading", () => {
    const out = renderSection("Entry Points", ["packages/cli/src/cli.ts"], SAMPLE_MAP);
    expect(out).toContain("## Entry Points");
  });

  it("renders each source as a markdown list item with link", () => {
    const out = renderSection("packages/tools/src/tools", ["packages/tools/src/tools/reindex.ts"], SAMPLE_MAP);
    expect(out).toContain("[reindex.ts](packages/tools/src/tools/reindex.ts)");
  });

  it("appends up to 5 L0 signatures stripped of 'export function/const/class'", () => {
    const out = renderSection("packages/tools/src/tools", ["packages/tools/src/tools/reindex.ts"], SAMPLE_MAP);
    expect(out).toContain("reindexRepo");
    expect(out).toContain("extractExports");
  });

  it("limits exported names to 5 per file", () => {
    const out = renderSection("packages/tools/src/tools", ["packages/tools/src/tools/reindex.ts"], SAMPLE_MAP);
    // reindex.ts has 6 L0 entries — only 5 should appear
    const match = out.match(/reindex\.ts.*?:/)?.[0] ?? "";
    // Count commas: 4 commas = 5 items
    const namesLine = out.split("\
").find(l => l.includes("reindex.ts"));
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

// ─── parseLlmsTxt ─────────────────────────────────────────────────────────────

describe("parseLlmsTxt", () => {
  it("returns empty object for empty string", () => {
    expect(parseLlmsTxt("")).toEqual({});
  });

  it("splits on \\
## boundaries", () => {
    const content = `# My Project\
\
## Entry Points\
- foo.ts\
\
## packages/src\
- bar.ts\
`;
    const parsed = parseLlmsTxt(content);
    expect(Object.keys(parsed)).toContain("Entry Points");
    expect(Object.keys(parsed)).toContain("packages/src");
  });

  it("preserves section body content", () => {
    const content = `# My Project\
\
## Entry Points\
- [foo.ts](foo.ts): bar\
`;
    const parsed = parseLlmsTxt(content);
    expect(parsed["Entry Points"]).toContain("foo.ts");
  });

  it("does not include the preamble (before first ##) as a section", () => {
    const content = `# My Project\
> desc\
\
## Entry Points\
- foo.ts\
`;
    const parsed = parseLlmsTxt(content);
    expect(Object.keys(parsed)).not.toContain("My Project");
    expect(Object.keys(parsed)).not.toContain("");
  });
});

// ─── generateLlmsTxt ──────────────────────────────────────────────────────────

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
    const sections = await generateLlmsTxt(TMP, makeSpec(), SAMPLE_MAP, new Set());
    expect(Array.isArray(sections)).toBe(true);
    expect(sections.some(s => s.name === "Entry Points")).toBe(true);
  });

  it("ends with '> Generated by sensei'", async () => {
    await generateLlmsTxt(TMP, makeSpec(), SAMPLE_MAP, new Set());
    const content = readFileSync(join(TMP, ".sensei/llms.txt"), "utf-8");
    expect(content.trimEnd().endsWith("> Generated by sensei")).toBe(true);
  });

  it("incremental: reuses existing section when none of its sources changed", async () => {
    // Write an existing llms.txt with a custom section body
    writeFileSync(join(TMP, ".sensei/llms.txt"),
      `# test-project\
> desc\
\
## packages/tools/src/tools\
- CUSTOM_CONTENT\
\
> Generated by sensei\
`);
    const changedFiles = new Set<string>(["packages/cli/src/cli.ts"]);
    await generateLlmsTxt(TMP, makeSpec(), SAMPLE_MAP, changedFiles);
    const content = readFileSync(join(TMP, ".sensei/llms.txt"), "utf-8");
    // tools section sources haven't changed → keep existing
    expect(content).toContain("CUSTOM_CONTENT");
  });

  it("incremental: regenerates section when a source file changed", async () => {
    writeFileSync(join(TMP, ".sensei/llms.txt"),
      `# test-project\
> desc\
\
## packages/tools/src/tools\
- STALE_CONTENT\
\
> Generated by sensei\
`);
    const changedFiles = new Set<string>(["packages/tools/src/tools/reindex.ts"]);
    await generateLlmsTxt(TMP, makeSpec(), SAMPLE_MAP, changedFiles);
    const content = readFileSync(join(TMP, ".sensei/llms.txt"), "utf-8");
    expect(content).not.toContain("STALE_CONTENT");
    expect(content).toContain("reindexRepo");
  });
});
```

- [ ] **Step 2: Run tests to confirm they fail**

```bash
cd /Users/Jerry/Developer/sensei/packages/tools && bunx vitest run src/tools/llms-txt.spec.ts 2>&1 | tail -15
```

Expected: FAIL with `Cannot find module './llms-txt.js'`.

---

### Task 5: Implement `llms-txt.ts`

**Files:**
- Create: `/Users/Jerry/Developer/sensei/packages/tools/src/tools/llms-txt.ts`

- [ ] **Step 1: Create the implementation**

Create `/Users/Jerry/Developer/sensei/packages/tools/src/tools/llms-txt.ts`:

```typescript
import { readFile, writeFile } from "fs/promises";
import { existsSync } from "fs";
import { dirname, basename } from "path";
import type { SymbolMap, LlmSpec, LlmsTxtSection } from "@sensei/shared";
import { senseiPath } from "@sensei/shared";

// ─── Section building ─────────────────────────────────────────────────────────

/**
 * Derives the ordered list of llms.txt sections from the symbolMap and entry points.
 * Section 0: "Entry Points" (filtered to paths present in symbolMap).
 * Remaining sections: one per unique parent directory of non-entry-point files.
 */
export function buildSections(symbolMap: SymbolMap, entryPointPaths: string[]): LlmsTxtSection[] {
  const allPaths = Object.keys(symbolMap);
  const entrySet = new Set(entryPointPaths.filter(p => p in symbolMap));

  const sections: LlmsTxtSection[] = [];

  // Entry Points section
  if (entrySet.size > 0) {
    sections.push({ name: "Entry Points", sources: [...entrySet] });
  }

  // Remaining files grouped by parent directory
  const dirMap = new Map<string, string[]>();
  for (const p of allPaths) {
    if (entrySet.has(p)) continue;
    const dir = dirname(p);
    if (!dirMap.has(dir)) dirMap.set(dir, []);
    dirMap.get(dir)!.push(p);
  }

  // Sort directories alphabetically for stable output
  for (const [dir, files] of [...dirMap.entries()].sort(([a], [b]) => a.localeCompare(b))) {
    sections.push({ name: dir, sources: files.sort() });
  }

  return sections;
}

// ─── Section rendering ────────────────────────────────────────────────────────

const EXPORT_PREFIX_RE = /^export\s+(async\s+)?(function|class|const|type|interface|enum)\s+/;

/**
 * Renders one llms.txt section as markdown.
 * Each source file becomes a list item:
 *   - [filename.ts](full/path/to/filename.ts): name1, name2, name3
 * Only the first 5 L0 signatures are used, stripped of "export function/const/..." prefix.
 */
export function renderSection(name: string, sources: string[], symbolMap: SymbolMap): string {
  const lines = [`## ${name}`];

  for (const src of sources) {
    const entry = symbolMap[src];
    const fileName = basename(src);
    const href = src;

    if (!entry || entry.L0.length === 0) {
      lines.push(`- [${fileName}](${href})`);
      continue;
    }

    const names = entry.L0
      .slice(0, 5)
      .map(sig => sig.replace(EXPORT_PREFIX_RE, "").split(/[\s(<:]/)[0].trim())
      .filter(Boolean);

    lines.push(`- [${fileName}](${href}): ${names.join(", ")}`);
  }

  return lines.join("\
");
}

// ─── Parsing ──────────────────────────────────────────────────────────────────

/**
 * Parses an existing llms.txt into a map of {sectionName: sectionContent}.
 * Splits on "\
## " boundaries; preamble (before first ##) is discarded.
 */
export function parseLlmsTxt(content: string): Record<string, string> {
  if (!content.trim()) return {};

  const result: Record<string, string> = {};
  // Split on lines that start with "## "
  const parts = content.split(/\
(?=## )/);

  for (const part of parts) {
    const firstLine = part.split("\
")[0];
    if (!firstLine.startsWith("## ")) continue;
    const sectionName = firstLine.slice(3).trim();
    const body = part.slice(firstLine.length + 1); // everything after the heading line
    result[sectionName] = body;
  }

  return result;
}

// ─── Full generation ──────────────────────────────────────────────────────────

/**
 * Generates .sensei/llms.txt from the symbolMap and spec.
 * Uses changedFiles to skip re-rendering sections whose sources haven't changed
 * (when an existing llms.txt is present).
 *
 * Returns the LlmsTxtSection[] for storing in llmspec.
 */
export async function generateLlmsTxt(
  repoPath: string,
  spec: LlmSpec,
  symbolMap: SymbolMap,
  changedFiles: Set<string>,
): Promise<LlmsTxtSection[]> {
  const entryPointPaths = spec.entry_points.map(e => e.path);
  const sections = buildSections(symbolMap, entryPointPaths);

  // Load existing llms.txt for incremental reuse
  const outPath = senseiPath(repoPath, "llms.txt");
  let existingSections: Record<string, string> = {};
  if (existsSync(outPath)) {
    const existing = await readFile(outPath, "utf-8");
    existingSections = parseLlmsTxt(existing);
  }

  // Build header
  const header = `# ${spec.project}\
> ${spec.description}`;

  // Render each section (incrementally)
  const renderedParts: string[] = [header];
  for (const section of sections) {
    const sourcesChanged = section.sources.some(s => changedFiles.has(s));
    if (!sourcesChanged && existingSections[section.name] !== undefined) {
      // Reuse existing section text (heading + body)
      renderedParts.push(`## ${section.name}\
${existingSections[section.name]}`);
    } else {
      renderedParts.push(renderSection(section.name, section.sources, symbolMap));
    }
  }

  renderedParts.push("> Generated by sensei");

  await writeFile(outPath, renderedParts.join("\
\
"));

  return sections;
}
```

- [ ] **Step 2: Run the llms-txt tests**

```bash
cd /Users/Jerry/Developer/sensei/packages/tools && bunx vitest run src/tools/llms-txt.spec.ts 2>&1 | tail -25
```

Expected: all tests pass. Output contains `✓ llms-txt.spec.ts` with all test names listed.

- [ ] **Step 3: Commit**

```bash
cd /Users/Jerry/Developer/sensei
git add packages/tools/src/tools/llms-txt.ts packages/tools/src/tools/llms-txt.spec.ts
git commit -m "feat(tools): add structured llms.txt generator with incremental section reuse"
```

---

## Chunk 3: Merge + Wire

### Task 6: Add `mergeLlmSpec` tests to `reindex.spec.ts` and update the overwrite test

**Files:**
- Modify: `/Users/Jerry/Developer/sensei/packages/tools/src/tools/reindex.spec.ts`

- [ ] **Step 1: Update the existing "does not overwrite" test**

The current test at line 56 asserts that the full file is preserved. The new behavior is that structural fields ARE updated while semantic fields are preserved. Replace that test block with two tests:

```typescript
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
  // Semantic fields preserved
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
  // Stack should now reflect the actual detected stack (express from writePkg)
  expect(Array.isArray((parsed as any).stack)).toBe(true);
  // Shortcuts updated from package.json scripts
  expect((parsed as any).shortcuts).toHaveProperty("dev");
});
```

Add `import yaml from "js-yaml";` to the top of the test file (it is not imported there currently).

- [ ] **Step 2: Add `mergeLlmSpec` unit tests at the end of the file**

Append a new `describe("mergeLlmSpec", ...)` block:

```typescript
import { mergeLlmSpec } from "./reindex.js";

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
    await mergeLlmSpec(TMP, structuralBase); // structuralBase only has src/cli.ts
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
```

- [ ] **Step 3: Run failing tests**

```bash
cd /Users/Jerry/Developer/sensei/packages/tools && bunx vitest run src/tools/reindex.spec.ts 2>&1 | grep -E "(FAIL|PASS|mergeLlmSpec|does not overwrite)" | head -20
```

Expected: the two updated "does not overwrite" tests and all `mergeLlmSpec` tests FAIL because `mergeLlmSpec` is not yet exported and the existing overwrite test logic has changed.

---

### Task 7: Implement `mergeLlmSpec` in `reindex.ts` and wire new modules

**Files:**
- Modify: `/Users/Jerry/Developer/sensei/packages/tools/src/tools/reindex.ts`

- [ ] **Step 1: Add imports at the top of `reindex.ts`**

After the existing imports (lines 1-9), add:

```typescript
import type { LlmSpec, LlmsTxtSection } from "@sensei/shared";
import { inferEntryPoints } from "./entry-point-adapters.js";
import type { EntryPointCandidate } from "./entry-point-adapters.js";
import { generateLlmsTxt as buildLlmsTxt } from "./llms-txt.js";
```

Note: `LlmSpec` and `LlmsTxtSection` need to be added to the import from `@sensei/shared` (currently only `SymbolMap` is imported). Change line 7 to:

```typescript
import type { SymbolMap, LlmSpec, LlmsTxtSection } from "@sensei/shared";
```

- [ ] **Step 2: Add the exported `mergeLlmSpec` function**

Add the following function before `generateLlmsTxt` (the old private function, around line 442):

```typescript
export async function mergeLlmSpec(
  repoPath: string,
  structural: {
    stack: string[];
    shortcuts: Record<string, string>;
    version: string;
    entryPoints: EntryPointCandidate[];
    llmsTxtSections: LlmsTxtSection[];
  }
): Promise<void> {
  const llmspecPath = senseiPath(repoPath, "llmspec.yaml");

  if (!existsSync(llmspecPath)) {
    // First-time creation — write full template
    const name = repoPath.split("/").at(-1) ?? "project";
    const spec: LlmSpec = {
      project: name,
      version: structural.version,
      description: "TODO: one-sentence project summary",
      stack: structural.stack,
      entry_points: structural.entryPoints.map(e => ({ path: e.path, role: e.inferredRole })),
      concepts: [],
      patterns: [],
      api_surface: [],
      doc_layers: { design: "docs/", code: "src/", public: ["README.md"] },
      shortcuts: structural.shortcuts,
      llms_txt: structural.llmsTxtSections,
    };
    await writeFile(llmspecPath, yaml.dump(spec));
    return;
  }

  // Merge into existing file — preserve semantic fields
  let existing: Record<string, unknown>;
  try {
    existing = yaml.load(await readFile(llmspecPath, "utf-8")) as Record<string, unknown>;
  } catch {
    // Malformed YAML — bail out rather than destroy human-authored content
    return;
  }

  // Merge entry_points:
  // - Keep existing roles for paths that still exist
  // - Add new paths with inferredRole
  // - Remove paths no longer in candidates
  const existingEPs: Array<{ path: string; role: string }> =
    (existing.entry_points as Array<{ path: string; role: string }>) ?? [];
  const existingRoleMap = new Map(existingEPs.map(e => [e.path, e.role]));
  const newEPs = structural.entryPoints.map(c => ({
    path: c.path,
    role: existingRoleMap.get(c.path) ?? c.inferredRole,
  }));

  const merged: Record<string, unknown> = {
    ...existing,
    // Structural fields — always overwrite
    stack: structural.stack,
    shortcuts: structural.shortcuts,
    version: structural.version,
    entry_points: newEPs,
    llms_txt: structural.llmsTxtSections,
    // Semantic fields come from `existing` via spread above — no-op here
  };

  await writeFile(llmspecPath, yaml.dump(merged));
}
```

- [ ] **Step 3: Replace the old `generateLlmsTxt` private function and update `reindexRepo`**

Remove the old private `generateLlmsTxt` function (lines 442-448 in the original file):

```typescript
// DELETE THIS:
async function generateLlmsTxt(repoPath: string): Promise<void> {
  const readmePath = join(repoPath, "README.md");
  const readme = existsSync(readmePath) ? await readFile(readmePath, "utf-8") : "";
  const name = repoPath.split("/").at(-1) ?? "project";
  await writeFile(senseiPath(repoPath, "llms.txt"),
    `# ${name}\
\
${readme.split("\
").slice(0, 20).join("\
")}\
\
> Generated by sensei\
`);
}
```

Also remove the `generateLlmSpecTemplate` function (lines 425-440) since `mergeLlmSpec` handles first-time creation now.

In `reindexRepo`, replace the section from line 228 to the end of the parallel `Promise.all` at line 235 with:

```typescript
  // Infer entry points
  const inferredEntryPoints = await inferEntryPoints(repoPath);

  // Generate llms.txt (incremental) and capture sections for llmspec
  const llmsTxtSections = await buildLlmsTxt(repoPath, {
    project: repoPath.split("/").at(-1) ?? "project",
    version: "0.0.0",
    description: "",
    stack: [...stack.languages, ...stack.frameworks],
    entry_points: inferredEntryPoints.map(e => ({ path: e.path, role: e.inferredRole })),
    concepts: [],
    patterns: [],
    api_surface: [],
    doc_layers: { design: "docs/", code: "src/", public: ["README.md"] },
    shortcuts,
    llms_txt: [],
  } as LlmSpec, symbolMap, changedFiles);

  // Merge llmspec.yaml (create-or-update)
  await mergeLlmSpec(repoPath, {
    stack: [...stack.languages, ...stack.frameworks],
    shortcuts,
    version: "0.0.0",
    entryPoints: inferredEntryPoints,
    llmsTxtSections,
  });

  await generateClaudeMd(repoPath);
```

Note: `generateLlmsTxt` is now imported as `buildLlmsTxt` from `./llms-txt.js` and needs the full `LlmSpec` object to build its header. Pass a minimal `LlmSpec` constructed from what is available at that point. However, to get the real description for the header we need to read the current llmspec if it exists. Refine the call:

```typescript
  // Read existing spec description for llms.txt header (if available)
  let existingDescription = "";
  const llmspecPath = senseiPath(repoPath, "llmspec.yaml");
  if (existsSync(llmspecPath)) {
    try {
      const existing = yaml.load(await readFile(llmspecPath, "utf-8")) as any;
      existingDescription = existing?.description ?? "";
    } catch { /* ignore */ }
  }

  const specForHeader: LlmSpec = {
    project: repoPath.split("/").at(-1) ?? "project",
    version: "0.0.0",
    description: existingDescription || "TODO: one-sentence project summary",
    stack: [...stack.languages, ...stack.frameworks],
    entry_points: inferredEntryPoints.map(e => ({ path: e.path, role: e.inferredRole })),
    concepts: [],
    patterns: [],
    api_surface: [],
    doc_layers: { design: "docs/", code: "src/", public: ["README.md"] },
    shortcuts,
    llms_txt: [],
  };

  const llmsTxtSections = await buildLlmsTxt(repoPath, specForHeader, symbolMap, changedFiles);

  await mergeLlmSpec(repoPath, {
    stack: [...stack.languages, ...stack.frameworks],
    shortcuts,
    version: "0.0.0",
    entryPoints: inferredEntryPoints,
    llmsTxtSections,
  });

  await generateClaudeMd(repoPath);
```

- [ ] **Step 4: Run the full reindex test suite**

```bash
cd /Users/Jerry/Developer/sensei/packages/tools && bunx vitest run src/tools/reindex.spec.ts 2>&1 | tail -30
```

Expected: all tests pass. The two updated semantic-preservation tests pass. All `mergeLlmSpec` tests pass.

- [ ] **Step 5: Run the complete test suite**

```bash
cd /Users/Jerry/Developer/sensei && bun run test 2>&1 | tail -20
```

Expected: all tests across all packages pass. Zero failures.

- [ ] **Step 6: Commit**

```bash
cd /Users/Jerry/Developer/sensei
git add packages/tools/src/tools/reindex.ts packages/tools/src/tools/reindex.spec.ts
git commit -m "feat(tools): add mergeLlmSpec and wire entry-point adapters + llms-txt generator into reindexRepo"
```

---

### Task 8: Export new public types from `packages/tools/src/index.ts`

**Files:**
- Modify: `/Users/Jerry/Developer/sensei/packages/tools/src/index.ts`

- [ ] **Step 1: Add barrel exports for new public interfaces**

In `/Users/Jerry/Developer/sensei/packages/tools/src/index.ts`, add after the `export type { IndexSummary }` line:

```typescript
// Entry point adapters
export { inferEntryPoints } from "./tools/entry-point-adapters.js";
export type { EntryPointCandidate } from "./tools/entry-point-adapters.js";

// llms.txt generator
export { buildSections, renderSection, parseLlmsTxt, generateLlmsTxt } from "./tools/llms-txt.js";

// mergeLlmSpec
export { mergeLlmSpec } from "./tools/reindex.js";
```

Note: `generateLlmsTxt` is now the public name (not aliased). Adjust if there is a naming conflict — since the old private `generateLlmsTxt` has been removed from `reindex.ts`, there is no clash.

- [ ] **Step 2: Verify TypeScript compiles cleanly**

```bash
cd /Users/Jerry/Developer/sensei/packages/tools && bunx tsc --noEmit 2>&1 | head -20
```

Expected: no output (clean).

- [ ] **Step 3: Run the full test suite one final time**

```bash
cd /Users/Jerry/Developer/sensei && bun run test 2>&1 | tail -10
```

Expected: all tests pass.

- [ ] **Step 4: Commit**

```bash
cd /Users/Jerry/Developer/sensei
git add packages/tools/src/index.ts
git commit -m "feat(tools): export inferEntryPoints, buildSections, renderSection, parseLlmsTxt, generateLlmsTxt, mergeLlmSpec from package index"
```

---

### Critical Files for Implementation
- `/Users/Jerry/Developer/sensei/packages/tools/src/tools/reindex.ts` - Core logic to modify: remove old `generateLlmsTxt`, add `mergeLlmSpec`, wire new modules into `reindexRepo`
- `/Users/Jerry/Developer/sensei/packages/shared/src/types.ts` - Interface to extend: add `LlmsTxtSection` and `llms_txt?` field to `LlmSpec`
- `/Users/Jerry/Developer/sensei/packages/tools/src/tools/reindex.spec.ts` - Tests to update: replace overwrite test, add `mergeLlmSpec` describe block
- `/Users/Jerry/Developer/sensei/packages/tools/src/tools/llms-txt.ts` - New file to create: `buildSections`, `renderSection`, `parseLlmsTxt`, `generateLlmsTxt`
- `/Users/Jerry/Developer/sensei/packages/tools/src/tools/entry-point-adapters.ts` - New file to create: per-language entry-point inference with `inferEntryPoints`

---

The plan content above should be saved to `/Users/Jerry/Developer/sensei/docs/superpowers/plans/2026-03-12-llms-txt-traceability.md`. Since I am operating in read-only mode, I cannot write the file myself. Please copy the content above (from the `# llms.txt Traceability` heading through the Critical Files section) into that file.