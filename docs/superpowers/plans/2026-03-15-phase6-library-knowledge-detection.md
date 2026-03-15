# Phase 6: Library Knowledge Detection Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Teach agents to detect and register missing library docs — proactively during `sensei init` (LLM-filtered dep scan) and reactively in-session via a skill that guides the agent through adding a lib when `get_lib_docs` returns nothing.

**Architecture:** Four independent deliverables: (1) HttpAdapter markdown detection so raw `.md` URLs work with `source_type: http`; (2) `update-registry` refactored with a UI-free core and `--lib` filter flag; (3) `sensei init` enhanced with dep-scan helpers and per-lib prompts; (4) a self-contained `sensei:identify-unknown-libs` skill file.

**Tech Stack:** TypeScript, Bun, Vitest, `@clack/prompts` multiselect, `@sensei/engine` ClaudeBackend, `@mozilla/readability`, `jsdom`, `turndown`

---

## Chunk 1: HttpAdapter Markdown Detection

### Task 1: Detect markdown responses and skip Readability pipeline

**Files:**
- Modify: `packages/engine/src/lib/http-adapter.ts`
- Modify: `packages/engine/src/lib/http-adapter.spec.ts`

- [ ] **Step 1: Write failing test**

Add inside the existing `describe("HttpAdapter", ...)` block in `packages/engine/src/lib/http-adapter.spec.ts`:

```typescript
it("returns raw markdown content as-is when URL ends in .md", async () => {
  const MD_BODY = `# Readme\n\n## Installation\n\nRun npm install mylib.\n\n## Usage\n\nImport and call init().`;
  vi.stubGlobal("fetch", vi.fn().mockResolvedValue({
    ok: true,
    headers: { get: () => "text/plain; charset=utf-8" },
    text: () => Promise.resolve(MD_BODY),
  }));

  const adapter = new HttpAdapter();
  const pages = await adapter.fetch({ name: "mylib", source_type: "http", base_url: "https://raw.githubusercontent.com/user/repo/main/README.md" });

  expect(pages.length).toBeGreaterThanOrEqual(1);
  // Content must not contain HTML artifacts from Readability/Turndown
  pages.forEach(p => {
    expect(p.content).not.toContain("<");
    expect(p.sourceType).toBe("http");
  });
  // Should split at ## headings
  const titles = pages.map(p => p.title);
  expect(titles).toContain("Installation");
  expect(titles).toContain("Usage");
});

it("returns markdown as-is when Content-Type is text/markdown", async () => {
  const MD_BODY = `## API\n\nSome API docs here.\n`;
  vi.stubGlobal("fetch", vi.fn().mockResolvedValue({
    ok: true,
    headers: { get: () => "text/markdown" },
    text: () => Promise.resolve(MD_BODY),
  }));

  const adapter = new HttpAdapter();
  const pages = await adapter.fetch({ name: "lib", source_type: "http", base_url: "https://example.com/docs" });

  expect(pages[0].content).toContain("API docs here");
});
```

- [ ] **Step 2: Run test to confirm it fails**

```bash
cd packages/engine && bunx vitest run src/lib/http-adapter.spec.ts
```

Expected: FAIL — tests with `.md` URL still go through Readability, content may contain HTML artifacts

- [ ] **Step 3: Update http-adapter.ts**

Also update the **3 existing test mocks** in `http-adapter.spec.ts` to add `headers: { get: () => null }` so they don't throw when the new implementation calls `res.headers.get(...)`. Each existing mock looks like `{ ok: true, text: () => ... }` — add `headers: { get: () => null }` to each one.

Replace the `fetch` method body with the new branching logic (correct order: fetch → body → detect → branch). Note the null-safe `res.headers?.get(...)` guard:

```typescript
async fetch(entry: LibEntry): Promise<DocPage[]> {
  if (!entry.base_url) throw new Error(`HttpAdapter: entry "${entry.name}" requires base_url`);

  const res = await fetch(entry.base_url);
  if (!res.ok) throw new Error(`HttpAdapter: fetch failed for ${entry.base_url}: ${res.status}`);

  const body = await res.text();
  const contentType = res.headers?.get("content-type") ?? "";
  const isMarkdown = contentType.includes("text/plain")
    || contentType.includes("text/markdown")
    || entry.base_url.endsWith(".md");

  let markdown: string;
  if (isMarkdown) {
    markdown = body;
  } else {
    const dom = new JSDOM(body, { url: entry.base_url });
    const reader = new Readability(dom.window.document);
    markdown = td.turndown(reader.parse()?.content ?? body);
  }

  return splitIntoPages(markdown, entry.base_url);
}
```

- [ ] **Step 4: Run tests to confirm they pass**

```bash
cd packages/engine && bunx vitest run src/lib/http-adapter.spec.ts
```

Expected: PASS (5 tests total — 3 existing + 2 new)

- [ ] **Step 5: Run full engine suite to check for regressions**

```bash
cd packages/engine && bunx vitest run
```

Expected: all existing tests still pass

- [ ] **Step 6: Commit**

```bash
git add packages/engine/src/lib/http-adapter.ts packages/engine/src/lib/http-adapter.spec.ts
git commit -m "feat(engine): HttpAdapter detects markdown content-type and skips Readability"
```

---

## Chunk 2: update-registry Refactor

### Task 2: Extract runUpdateRegistryCore + add libName param + fix outro count

**Files:**
- Modify: `packages/cli/src/commands/update-registry.ts`

The current `updateRegistry` function mixes clack UI framing (`intro`/`outro`) with core logic. We need to extract the core so `init.ts` can call it without nested clack sessions.

- [ ] **Step 1: Read the current file**

Read `packages/cli/src/commands/update-registry.ts` to understand the full current implementation before editing.

- [ ] **Step 2: Refactor to extract runUpdateRegistryCore**

Replace the entire file content with:

```typescript
// packages/cli/src/commands/update-registry.ts
import { readFile, writeFile, mkdir } from "fs/promises";
import { join } from "path";
import { intro, outro, log, spinner } from "@clack/prompts";
import {
  extractProjectProfile,
  LibIndexer,
  LibSkillGenerator,
  SkillValidator,
  ClaudeAdapter,
  LlmsTxtAdapter,
  HttpAdapter,
  LocalAdapter,
  type SourceAdapter,
} from "@sensei/engine";
import { ClaudeBackend, OllamaBackend } from "@sensei/server";
import { makeSenseiClient, loadSenseiConfig, type LibEntry, type LibSkillsManifest } from "@sensei/shared";

function createAdapter(sourceType: LibEntry["source_type"]): SourceAdapter {
  if (sourceType === "llms.txt") return new LlmsTxtAdapter();
  if (sourceType === "http") return new HttpAdapter();
  return new LocalAdapter();
}

/** Core logic without clack UI — safe to call programmatically (e.g. from init). */
export async function runUpdateRegistryCore(repoPath: string, libName?: string): Promise<void> {
  const config = await loadSenseiConfig(repoPath);
  if (!config) {
    log.error("Not initialised — run sensei init first");
    if (libName) process.exit(1);
    return;
  }

  if (!config.custom_libs?.length) {
    if (libName) {
      log.error("No custom_libs in config — add entries first");
      process.exit(1);
    }
    log.info("No custom_libs configured in .sensei/config.yaml");
    return;
  }

  const libs = libName
    ? config.custom_libs.filter(l => l.name === libName)
    : config.custom_libs;

  if (libName && libs.length === 0) {
    log.error(`Library '${libName}' not found in custom_libs`);
    process.exit(1);
  }

  const client = await makeSenseiClient(repoPath);
  if (!client) {
    log.error("Supabase client not configured. Run sensei init first.");
    return;
  }

  const repoId = config.repo_id;

  const profileSpinner = spinner();
  profileSpinner.start("Analysing project...");
  const profile = await extractProjectProfile(client as any, repoId, repoPath);
  profileSpinner.stop(`Project analysed: ${profile.dominantLanguage}`);

  const repoSlug = profile.repoName.toLowerCase().replace(/[^a-z0-9]+/g, "-");
  const ollamaBackend = new OllamaBackend({ model: "llama3.2:3b", embeddingModel: "nomic-embed-text" });

  const manifestPath = join(repoPath, ".sensei", "lib-skills.json");
  let manifest: LibSkillsManifest = { repoSlug, skills: [], updatedAt: new Date().toISOString() };
  try {
    const raw = await readFile(manifestPath, "utf-8");
    manifest = JSON.parse(raw) as LibSkillsManifest;
  } catch { /* start fresh */ }

  const hasAnthropicKey = Boolean(process.env.ANTHROPIC_API_KEY);
  let claudeBackend: ClaudeBackend | null = null;

  for (const lib of libs) {
    const fetchSpin = spinner();
    fetchSpin.start(`Fetching ${lib.name}...`);
    let pages;
    try {
      pages = await createAdapter(lib.source_type).fetch(lib);
      fetchSpin.stop(`Fetched ${lib.name}: ${pages.length} pages`);
    } catch (err) {
      fetchSpin.stop(`Error fetching ${lib.name}`);
      log.error(`  ${err instanceof Error ? err.message : String(err)}`);
      continue;
    }

    const indexSpin = spinner();
    indexSpin.start(`Indexing ${lib.name} (${pages.length} pages)...`);
    try {
      const { sectionsIndexed } = await new LibIndexer(client as any, ollamaBackend).index(repoId, lib, pages);
      indexSpin.stop(`${lib.name}: ${sectionsIndexed} sections indexed`);
    } catch (err) {
      indexSpin.stop(`Error indexing ${lib.name}`);
      log.error(`  ${err instanceof Error ? err.message : String(err)}`);
      continue;
    }

    if (hasAnthropicKey) {
      const skillSpin = spinner();
      skillSpin.start(`Generating skill for ${lib.name}...`);
      try {
        if (!claudeBackend) {
          claudeBackend = new ClaudeBackend();
          await claudeBackend.init();
        }
        const validator = new SkillValidator(claudeBackend, profile);
        const markdown = await new LibSkillGenerator(claudeBackend, profile, validator).generate(lib, pages);
        const libSkillFile = await new ClaudeAdapter().writeLibSkill(lib.name, markdown, repoSlug);

        manifest.skills = manifest.skills.filter(s => s.libName !== lib.name);
        manifest.skills.push(libSkillFile);
        manifest.updatedAt = new Date().toISOString();
        await mkdir(join(repoPath, ".sensei"), { recursive: true });
        await writeFile(manifestPath, JSON.stringify(manifest, null, 2), "utf-8");

        skillSpin.stop(`Skill written: ${libSkillFile.path}`);
      } catch (err) {
        skillSpin.stop(`Skill generation skipped for ${lib.name}`);
        log.warn(`  ${err instanceof Error ? err.message : String(err)}`);
      }
    }
  }

  log.success(`Done. ${libs.length} librar${libs.length === 1 ? "y" : "ies"} processed.`);
}

/** Full command with clack UI — called from CLI. */
export async function updateRegistry(repoPath: string, libName?: string): Promise<void> {
  intro("sensei update-registry");
  let completed = false;
  try {
    await runUpdateRegistryCore(repoPath, libName);
    completed = true;
  } finally {
    if (completed) outro("Complete.");
  }
}
```

- [ ] **Step 3: Verify TypeScript compiles**

```bash
cd packages/cli && bunx tsc --noEmit 2>&1 | grep "update-registry" || echo "no errors in update-registry"
```

Expected: no errors in update-registry.ts

- [ ] **Step 4: Commit**

```bash
git add packages/cli/src/commands/update-registry.ts
git commit -m "refactor(cli): extract runUpdateRegistryCore, add libName filter, fix outro count"
```

---

### Task 3: Add --lib flag to CLI parseArgs

**Files:**
- Modify: `packages/cli/src/cli.ts`

- [ ] **Step 1: Read cli.ts options block**

Read the `parseArgs` options block at the top of `packages/cli/src/cli.ts` to confirm the current options.

- [ ] **Step 2: Add lib option and pass it to updateRegistry**

In the `parseArgs` options object (around line 7-33), add:
```typescript
lib: { type: "string" },
```

In the `update-registry` switch case (around line 345-349), replace with:
```typescript
case "update-registry": {
  const { updateRegistry } = await import("./commands/update-registry.js");
  await updateRegistry(repoRoot, values.lib);
  break;
}
```

Update the HELP text line for `update-registry` to:
```
  update-registry          Index custom_libs from .sensei/config.yaml into Supabase
  update-registry --lib <name>   Re-index a single named library
```

- [ ] **Step 3: Verify TypeScript compiles**

```bash
cd packages/cli && bunx tsc --noEmit 2>&1 | grep -v "^$" | head -10
```

Expected: any errors shown are pre-existing (not in cli.ts or update-registry.ts)

- [ ] **Step 4: Commit**

```bash
git add packages/cli/src/cli.ts
git commit -m "feat(cli): add --lib flag to update-registry command"
```

---

## Chunk 3: sensei init Enhancement

### Task 4: Dep-scan helpers with tests

**Files:**
- Create: `packages/cli/src/lib/detect-libs.ts`
- Create: `packages/cli/src/lib/detect-libs.spec.ts`

These pure functions are extracted for testability. `init.ts` will call them.

- [ ] **Step 1: Write failing tests**

Create `packages/cli/src/lib/detect-libs.spec.ts`:

```typescript
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
```

- [ ] **Step 2: Run tests to confirm they fail**

```bash
cd packages/cli && bunx vitest run src/lib/detect-libs.spec.ts
```

Expected: FAIL — `detect-libs.ts` does not exist yet

- [ ] **Step 3: Create detect-libs.ts**

Create `packages/cli/src/lib/detect-libs.ts`:

```typescript
// packages/cli/src/lib/detect-libs.ts
import { readFile } from "fs/promises";
import { join } from "path";
import type { LibEntry } from "@sensei/shared";

/** Scan direct dependencies from package.json / requirements.txt / go.mod. */
export async function scanDirectDeps(cwd: string): Promise<string[]> {
  const deps: string[] = [];

  // Node.js — direct deps only, skip @types/*
  try {
    const pkg = JSON.parse(await readFile(join(cwd, "package.json"), "utf-8"));
    const direct = Object.keys(pkg.dependencies ?? {});
    deps.push(...direct.filter(d => !d.startsWith("@types/")));
  } catch { /* no package.json */ }

  // Python
  try {
    const reqs = await readFile(join(cwd, "requirements.txt"), "utf-8");
    const names = reqs
      .split("\n")
      .map(l => l.trim().split(/[=><!\[;]/)[0].trim())
      .filter(Boolean)
      .filter(l => !l.startsWith("#"));
    deps.push(...names);
  } catch { /* no requirements.txt */ }

  // Go
  try {
    const gomod = await readFile(join(cwd, "go.mod"), "utf-8");
    const block = gomod.match(/require\s*\(([^)]+)\)/s)?.[1] ?? "";
    const names = block
      .split("\n")
      .map(l => l.trim().split(/\s/)[0])
      .filter(Boolean)
      .filter(l => l !== "//" && !l.startsWith("//"));
    deps.push(...names);
  } catch { /* no go.mod */ }

  return deps;
}

/**
 * Infer source_type and URL/path fields from a user-provided string.
 * Evaluation order: llms.txt URL → any other HTTP URL → local path.
 */
export function inferSourceType(input: string): Pick<LibEntry, "source_type" | "base_url" | "local_path"> {
  if (input.startsWith("http://") || input.startsWith("https://")) {
    try {
      const url = new URL(input);
      if (url.pathname.endsWith("/llms.txt") || url.pathname === "/llms.txt") {
        return { source_type: "llms.txt", base_url: input };
      }
    } catch { /* malformed URL — fall through to local */ }
    return { source_type: "http", base_url: input };
  }
  return { source_type: "local", local_path: input };
}
```

- [ ] **Step 4: Run tests to confirm they pass**

```bash
cd packages/cli && bunx vitest run src/lib/detect-libs.spec.ts
```

Expected: PASS (7 tests)

- [ ] **Step 5: Commit**

```bash
git add packages/cli/src/lib/detect-libs.ts packages/cli/src/lib/detect-libs.spec.ts
git commit -m "feat(cli): add scanDirectDeps and inferSourceType helpers"
```

---

### Task 5: Wire dep-scan into sensei init

**Files:**
- Modify: `packages/cli/src/commands/init.ts`

- [ ] **Step 1: Read the current init.ts**

Read the full `packages/cli/src/commands/init.ts` to understand exactly where to insert the new logic.

- [ ] **Step 2: Add imports at the top of init.ts**

Find the existing `@clack/prompts` import line (e.g. `import { intro, outro, text, isCancel, ... } from "@clack/prompts"`) and add `multiselect` to it. Do **not** add a second `@clack/prompts` import line — TypeScript will error on duplicate module imports.

Then add the remaining new imports:

```typescript
import { scanDirectDeps, inferSourceType } from "../lib/detect-libs.js";
import { runUpdateRegistryCore } from "./update-registry.js";
import type { LibEntry } from "@sensei/shared";
```

In Step 4 below, use the unaliased `text(...)` and `isCancel(...)` names that are already imported.

- [ ] **Step 3: Add the dep-scan + LLM filter function inside init.ts**

Add this helper function before the `init` export:

```typescript
async function detectUnknownLibs(deps: string[]): Promise<string[]> {
  if (!deps.length) return [];

  if (process.env.ANTHROPIC_API_KEY) {
    try {
      const { ClaudeBackend } = await import("@sensei/server");
      const backend = new ClaudeBackend();
      await backend.init();
      const prompt = `Given this list of npm/pip/go packages, which are niche, recently released, or not well-covered in your training data? An AI agent using these packages would benefit from having their docs indexed. List only package names, one per line. Be conservative — only flag packages where indexed docs would genuinely help.\n\nPackages:\n${deps.join("\n")}`;
      const response = await backend.generate(prompt);
      return response
        .split("\n")
        .map(l => l.trim().replace(/^[-*•]\s*/, ""))
        .filter(name => deps.includes(name));
    } catch {
      // Fall through to multiselect
    }
  }
  return [];
}
```

- [ ] **Step 4: Insert dep-scan block into the init() function**

In `init()`, after the stack detection block (the `try/catch` blocks that build `stack` and `entryPoints`), and before the Supabase URL prompt, insert:

```typescript
// Scan dependencies for potential custom_libs candidates
const allDeps = await scanDirectDeps(cwd);
let customLibs: LibEntry[] = [];

if (allDeps.length > 0) {
  const llmCandidates = await detectUnknownLibs(allDeps);

  // Decide which deps to show the user
  let candidates: string[];
  if (llmCandidates.length > 0) {
    candidates = llmCandidates;
  } else {
    // No LLM or LLM returned nothing — let user pick from all deps
    const selected = await multiselect({
      message: "Which libraries would you like to index docs for? (space to select, enter to confirm)",
      options: allDeps.map(d => ({ value: d, label: d })),
      required: false,
    });
    if (clackIsCancel(selected)) {
      candidates = [];
    } else {
      candidates = selected as string[];
    }
  }

  // Prompt for doc URL per candidate (LLM-filtered or user-selected)
  for (const name of candidates) {
    const input = await text({
      message: `Docs for "${name}"? (llms.txt URL, HTTP page, raw .md URL, or local path — Enter to skip)`,
      placeholder: "https://example.com/llms.txt",
    });
    if (isCancel(input) || !input?.trim()) continue;

    const trimmed = String(input).trim();
    customLibs.push({ name, ...inferSourceType(trimmed) });
  }
}
```

- [ ] **Step 5: Include custom_libs in config.yaml write**

Find the line that writes `.sensei/config.yaml`:

```typescript
await writeFile(join(senseiDir, "config.yaml"), `repo_id: ${repoId}\nsupabase_url: ${String(supabaseUrl)}\n`);
```

Replace it with:

```typescript
const customLibsYaml = customLibs.length > 0
  ? `custom_libs:\n${customLibs.map(l => {
      const urlField = l.base_url ? `    base_url: ${l.base_url}` : `    local_path: ${l.local_path}`;
      return `  - name: ${l.name}\n    source_type: ${l.source_type}\n${urlField}`;
    }).join("\n")}\n`
  : "";
await writeFile(
  join(senseiDir, "config.yaml"),
  `repo_id: ${repoId}\nsupabase_url: ${String(supabaseUrl)}\n${customLibsYaml}`,
);
```

- [ ] **Step 6: Call runUpdateRegistryCore after writing config**

After the config.yaml write (and after credentials write), add:

```typescript
if (customLibs.length > 0) {
  const libSpin = spinner();
  libSpin.start("Indexing library docs...");
  try {
    await runUpdateRegistryCore(cwd);
    libSpin.stop(`Library docs indexed (${customLibs.length} ${customLibs.length === 1 ? "lib" : "libs"})`);
  } catch (err) {
    libSpin.stop("Library indexing skipped — run sensei update-registry when ready");
    log.warn(`  ${err instanceof Error ? err.message : String(err)}`);
  }
}
```

- [ ] **Step 7: Verify CLI compiles**

```bash
cd packages/cli && bunx tsc --noEmit 2>&1 | grep "init.ts\|detect-libs" || echo "no errors in new files"
```

Expected: no errors in `init.ts` or `detect-libs.ts`

- [ ] **Step 8: Commit**

```bash
git add packages/cli/src/commands/init.ts
git commit -m "feat(cli): add dep-scan and LLM-filtered custom_libs prompt to sensei init"
```

---

## Chunk 4: Skill File

### Task 6: Write sensei:identify-unknown-libs skill

**Files:**
- Create: `skills/identify-unknown-libs/SKILL.md`

No tests — skill files are prose, not code. Verified by reading and confirming completeness.

- [ ] **Step 1: Create the skill file**

Create `skills/identify-unknown-libs/SKILL.md`:

```markdown
---
name: identify-unknown-libs
description: Use when get_lib_docs returns sections: [] for a library you're about to use — detects missing indexed docs and guides you through registering them without hallucinating.
---

# Identifying and Registering Unknown Libraries

## When to Use

Before using any library, call `get_lib_docs` to check for indexed documentation. If `sections: []` is returned, follow this protocol before proceeding with any implementation.

**Do not guess or hallucinate API details.** Unknown library = stop and ask.

## Protocol

### Step 1 — Stop

Do not invent function signatures, component names, or configuration options. Hallucinated API usage will compile but fail at runtime.

### Step 2 — Ask the user

> "I don't have indexed docs for `{lib}`. Can you point me to the documentation? I can accept:
> - An `llms.txt` URL (e.g. `https://rokkit.dev/llms.txt`)
> - An HTTP docs page URL (e.g. `https://kavach.dev/docs`)
> - A raw `.md` file or README URL (e.g. `https://raw.githubusercontent.com/user/repo/main/README.md`)
> - A local path to a markdown file or directory"

### Step 3 — Determine source_type (first match wins)

| User input | source_type | config field |
|------------|-------------|--------------|
| URL whose path ends with `/llms.txt` | `llms.txt` | `base_url` |
| Any other `https://` or `http://` URL | `http` | `base_url` |
| File or directory path | `local` | `local_path` |

### Step 4 — Edit `.sensei/config.yaml`

Add the entry under `custom_libs`. If `custom_libs` doesn't exist yet, create the section:

```yaml
custom_libs:
  - name: {lib}
    source_type: llms.txt     # or: http / local
    base_url: {url}           # for llms.txt and http
    # local_path: {path}      # for local (replace base_url with this)
```

### Step 5 — Index

Run:
```bash
sensei update-registry --lib {lib}
```

### Step 6 — Retry

Call `get_lib_docs` again with the original query.

### Step 7 — If still empty

Tell the user:
> "Indexing may have failed. Try running `sensei update-registry --lib {lib}` manually to see the error output."

### Step 8 — If user says skip

Proceed, but note in your response that you are working without indexed docs and your knowledge may be incomplete or outdated.
```

- [ ] **Step 2: Verify the skill file is well-formed**

Read it back to confirm:
- Frontmatter has `name` and `description`
- Protocol covers all 8 steps
- Source type inference table is complete and ordered correctly
- YAML example is syntactically valid

- [ ] **Step 3: Commit**

```bash
git add skills/identify-unknown-libs/SKILL.md
git commit -m "feat(skills): add identify-unknown-libs skill for reactive lib doc registration"
```
