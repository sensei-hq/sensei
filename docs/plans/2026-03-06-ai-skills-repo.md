# AI Skills Repo Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a universal AI skills library with a codebase indexer, content compression techniques, agentic workflow guidance, doc drift detection, context management, and an MCP server that offloads deterministic tasks so LLMs use fewer tokens.

**Architecture:** Skills are model-agnostic markdown files installed to `~/.claude/skills`. An MCP server (`repo-index-server`) scans repos, stores structured indexes in `.index/`, and serves targeted slices on demand. A `.llmspec.yaml` per repo acts as the primary orientation artifact. A benchmark system measures skill value with A/B comparisons.

**Tech Stack:** TypeScript, Bun (runtime + package manager + workspaces), `@modelcontextprotocol/sdk`, `@clack/prompts`, `vitest` (unit: `*.spec.ts`), Playwright (e2e: `e2e/*.e2e.ts`), `js-yaml`, `fast-glob`

> **Path note:** All tasks below use `packages/repo-index-server/` (not `mcp/repo-index-server/`). Replace `npm install` → `bun install`, `npm run` → `bun run`, `npx vitest` → `bunx vitest`. The root `package.json` is a bun workspace — see Task 1.

---

## Task 1: Scaffold bun workspaces monorepo

**Files:**
- Create: `package.json` (workspace root — clean, no config noise)
- Create: `config/tsconfig.base.json`
- Create: `config/vitest.config.base.ts`
- Create: `config/eslint.config.js`
- Create: `packages/sensei/package.json`
- Create: `packages/sensei/tsconfig.json`
- Create: `packages/sensei/vitest.config.ts`
- Create: `packages/sensei/playwright.config.ts`
- Create: `packages/sensei/e2e/` (directory)
- Create: `apps/` (directory, empty for now)
- Create: `skills/` subdirectories
- Create: `.gitignore`

**Step 1: Create directory tree**

```bash
mkdir -p config
mkdir -p packages/sensei/src/tools
mkdir -p packages/sensei/src/commands
mkdir -p packages/sensei/e2e
mkdir -p apps
mkdir -p skills/codebase-indexer
mkdir -p skills/content-compression
mkdir -p skills/agentic-dev-workflow
mkdir -p skills/doc-drift-detector
mkdir -p skills/context-manager
mkdir -p skills/benchmark-runner
mkdir -p tasks
mkdir -p results
```

**Step 2: Write root `package.json`**

```json
{
  "name": "sensei",
  "private": true,
  "workspaces": ["packages/*", "apps/*"],
  "scripts": {
    "build": "bun run --filter='*' build",
    "test": "bun run --filter='*' test",
    "test:e2e": "bun run --filter='*' test:e2e",
    "dev": "bun run --filter='sensei' dev"
  }
}
```

**Step 3: Write `config/tsconfig.base.json`**

```json
{
  "compilerOptions": {
    "target": "ES2022",
    "module": "ESNext",
    "moduleResolution": "bundler",
    "strict": true,
    "esModuleInterop": true,
    "skipLibCheck": true,
    "declaration": true,
    "declarationMap": true,
    "sourceMap": true
  }
}
```

**Step 4: Write `config/vitest.config.base.ts`**

```typescript
import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    include: ["src/**/*.spec.ts"],
    environment: "node",
    coverage: {
      reporter: ["text", "lcov"],
    },
  },
});
```

**Step 5: Write `config/eslint.config.js`**

```javascript
import js from "@eslint/js";
import tseslint from "typescript-eslint";

export default tseslint.config(
  js.configs.recommended,
  ...tseslint.configs.recommended,
  {
    rules: {
      "@typescript-eslint/no-unused-vars": ["error", { argsIgnorePattern: "^_" }],
      "@typescript-eslint/no-explicit-any": "warn",
    },
  }
);
```

**Step 6: Write `packages/sensei/package.json`**

```json
{
  "name": "sensei",
  "version": "0.1.0",
  "type": "module",
  "bin": { "sensei": "./dist/cli.js" },
  "scripts": {
    "dev": "bun run --watch src/index.ts",
    "build": "bun build src/index.ts src/cli.ts --outdir dist --target bun",
    "test": "bunx vitest run",
    "test:e2e": "bunx playwright test",
    "test:watch": "bunx vitest"
  },
  "dependencies": {
    "@modelcontextprotocol/sdk": "^1.0.0",
    "@clack/prompts": "^0.9.0",
    "fast-glob": "^3.3.0",
    "js-yaml": "^4.1.0",
    "zod": "^3.22.0"
  },
  "devDependencies": {
    "@eslint/js": "^9.0.0",
    "@playwright/test": "^1.48.0",
    "@types/js-yaml": "^4.0.9",
    "@types/node": "^22.0.0",
    "typescript": "^5.5.0",
    "typescript-eslint": "^8.0.0",
    "vitest": "^2.0.0"
  }
}
```

**Step 7: Write `packages/sensei/tsconfig.json`**

```json
{
  "extends": "../../config/tsconfig.base.json",
  "compilerOptions": {
    "outDir": "dist",
    "rootDir": "src"
  },
  "include": ["src/**/*"],
  "exclude": ["node_modules", "dist", "e2e"]
}
```

**Step 8: Write `packages/sensei/vitest.config.ts`**

```typescript
import { mergeConfig } from "vitest/config";
import base from "../../config/vitest.config.base.ts";

export default mergeConfig(base, {});
```

**Step 9: Write `packages/sensei/playwright.config.ts`**

```typescript
import { defineConfig } from "@playwright/test";

export default defineConfig({
  testDir: "./e2e",
  testMatch: "**/*.e2e.ts",
  timeout: 30_000,
  use: {
    trace: "on-first-retry",
  },
});
```

**Step 10: Write root `.gitignore`**

```
node_modules/
dist/
results/*.json
.index/
.env
```

**Step 11: Install dependencies**

```bash
bun install
# Expected: bun.lockb created, node_modules populated
```

**Step 12: Verify workspace resolves**

```bash
bun run test
# Expected: no test files found yet, exits cleanly
```

**Step 10: Commit**

```bash
git add .
git commit -m "chore: scaffold bun workspaces monorepo"
```

---

> **For all tasks below:** paths are relative to `packages/repo-index-server/`.
> Replace any `npm install` → `bun install`, `npm run` → `bun run`, `npx vitest` → `bunx vitest`.
> Unit tests use `*.spec.ts` co-located with source. E2E tests use `e2e/*.e2e.ts`.

---
| `context-manager` | Load/offload context efficiently |
| `benchmark-runner` | Evaluate skill impact with A/B comparisons |
```

**Step 4: Commit**

```bash
git add .
git commit -m "chore: scaffold repo structure"
```

---

## Task 2: Write `content-compression` skill

**Files:**
- Create: `skills/content-compression/SKILL.md`

This skill is foundational — other skills reference it. Write it first.

**Step 1: Write the skill**

```markdown
---
name: content-compression
description: Use when working with code in LLM context and needing to reduce token usage, choose representation levels, or compress code for agent consumption without losing reasoning ability.
---

# Content Compression

## Overview

Code has four resolution levels. Serve the minimal level that lets the LLM complete its task. Docstrings and doc-comments waste tokens — LLMs infer meaning from signatures.

## Resolution Levels

| Level | Format | ~Tokens | Use when |
|---|---|---|---|
| L0 — Signature | `processOrder(orderId: string): Promise<Order>` | 10 | Discovery, listing available functions |
| L1 — IO Pattern | `order = processOrder(orderId)` + input/output types | 30 | Understanding what a function does |
| L2 — Logic Flow | Bullet steps or pseudocode | 80 | Understanding how it works |
| L3 — Full Source | Actual code | 200–2000 | Editing, debugging, modifying |

## Task → Level Mapping

```
"list available functions in auth module"   → L0
"what does login() return?"                 → L1
"trace the auth flow"                       → L2
"fix a bug in validateToken()"              → L3
```

## Compression Rules

**Always strip:**
- Docstrings and doc-comments (repeat what signatures already say)
- Import statements at L0/L1/L2
- Type boilerplate (generics, decorators) at L0/L1
- Dead code, commented-out blocks

**L2 logic flow notation:**
- `if/else` → indented bullets: `if valid → proceed`, `else → throw error`
- Loops → `for each item → transform and collect`
- Pipelines → `raw → parse() → validate() → transform() → output`
- Async → `await fetch → check status → parse body`

**L1 IO pattern notation:**
```
result = functionName(input)
// input: { id: string, options?: Config }
// result: Promise<{ data: T, error?: string }>
```

**State machine shorthand (any level):**
```
idle → loading → success
              ↘ error → retry → loading
```

## Common Mistakes

| Mistake | Fix |
|---|---|
| Loading L3 to understand what a function does | Use L1 instead — 98% token reduction |
| Keeping docstrings at L0/L1/L2 | Strip them — they duplicate signature info |
| Loading full file to find one function | Use `get_file_context(path, "L0")` to list, then `L3` for the specific function |
| Summarising code yourself | Call `get_file_context(path, level)` MCP tool — consistent, cached, zero tokens spent |
```

**Step 2: Check word count**

```bash
wc -w skills/content-compression/SKILL.md
# Target: under 400 words
```

**Step 3: Commit**

```bash
git add skills/content-compression/SKILL.md
git commit -m "feat: add content-compression skill"
```

---

## Task 3: Write `agentic-dev-workflow` skill

**Files:**
- Create: `skills/agentic-dev-workflow/SKILL.md`

**Step 1: Write the skill**

```markdown
---
name: agentic-dev-workflow
description: Use when starting an agentic developer session, beginning a new task in a codebase, or when an agent is spending too many tokens on orientation, broad searches, or loading full files unnecessarily.
---

# Agentic Dev Workflow

## Overview

Orient narrow, work targeted, offload deterministic tasks to MCP. Never load full files when a slice will do. The goal: minimum tokens, maximum accuracy.

## Session Protocol

```
digraph workflow {
    "Start session" -> "Load .llmspec.yaml";
    "Load .llmspec.yaml" -> "Call recommend_next(task)";
    "Call recommend_next(task)" -> "Load targeted slice";
    "Load targeted slice" -> "Work on task";
    "Work on task" -> "Need specific detail?" [label="sometimes"];
    "Need specific detail?" -> "Call query_index / get_file_context(L3)" [label="yes"];
    "Need specific detail?" -> "Work on task" [label="no"];
    "Work on task" -> "Switching task?" [label="when done"];
    "Switching task?" -> "Call checkpoint()" [label="yes"];
    "checkpoint()" -> "Call recommend_next(new task)";
    "Switching task?" -> "Commit and close" [label="no, done"];
}
```

## Rules

1. **Start with llmspec, not file tree** — `get_llmspec()` gives orientation in ~500 tokens
2. **Use L0 before L3** — list signatures first, load source only when editing
3. **Offload to MCP** — generation, validation, drift checks → MCP tools, not LLM reasoning
4. **Never grep the whole repo** — use `query_index(query)` or `find_pattern(name)`
5. **Checkpoint before switching** — `checkpoint()` preserves state, unloads dead context

## Task Entry Checklist

Before starting any task:
- [ ] Is `.llmspec.yaml` loaded? If not, call `get_llmspec()`
- [ ] Called `recommend_next(task)` to get the minimal context slice?
- [ ] Using content-compression levels correctly? (see `content-compression` skill)

## Offload to MCP (not LLM context)

| Task | MCP tool | Why |
|---|---|---|
| Generate llms.txt | `generate_llms_txt()` | Consistent output, zero context tokens |
| Check doc drift | `check_drift()` | Deterministic file comparison |
| List all exports | `list_exports(module)` | Cached index lookup |
| Find usage pattern | `find_pattern(name)` | Pre-indexed, targeted result |
| Orient new session | `onboard_agent()` | Structured briefing in one call |

## Anti-Patterns

| Anti-pattern | Fix |
|---|---|
| Reading all files in a directory | Call `list_exports(module)` at L0 |
| Writing llms.txt from scratch | Call `generate_llms_txt()` |
| Grepping whole repo for a pattern | Call `find_pattern(name)` |
| Loading full file to find one function | `get_file_context(path, "L0")` then `L3` for the target function only |
```

**Step 2: Commit**

```bash
git add skills/agentic-dev-workflow/SKILL.md
git commit -m "feat: add agentic-dev-workflow skill"
```

---

## Task 4: Write LLMSpec template and format doc

**Files:**
- Create: `skills/codebase-indexer/llmspec-template.yaml`
- Create: `skills/codebase-indexer/extractor.md`

**Step 1: Write `llmspec-template.yaml`**

```yaml
# .llmspec.yaml — LLM orientation spec for <project-name>
# Generated by codebase-indexer skill. Regenerate with: npm run index (or see shortcuts)
# Format version: 1.0

project: ""
version: ""
description: ""           # One sentence: what this project does

stack: []                 # [typescript, react, postgres, etc.]

entry_points:
  - path: ""
    role: ""              # server entry / route definitions / config / etc.

concepts: []              # Key domain terms an LLM must know
  # - name: ""
  #   definition: ""

patterns: []              # Conventions an LLM must follow when editing
  # - name: ""
  #   files: ""
  #   convention: ""

api_surface: []           # Key public functions/endpoints
  # - name: ""
  #   path: ""
  #   io: ""              # L1 notation: result = fn(input)
  #   flow: ""            # L2 notation: step1 → step2 → output

doc_layers:
  design: ""              # Path to design docs / ADRs
  code: ""                # Source root
  public: []              # [README.md, docs/guides/, etc.]

shortcuts:
  dev: ""                 # Start dev server
  test: ""                # Run tests
  index: ""               # Re-run codebase indexer
  build: ""               # Build for production
```

**Step 2: Write `extractor.md`**

```markdown
# Codebase Extractor Guide

What to extract when indexing a repo, and how.

## File Map

- Run directory tree limited to 3 levels deep
- Identify entry points: files named `index`, `main`, `app`, `server`, `router`, `config`
- Note any existing `CLAUDE.md`, `.cursorrules`, `llms.txt`, `.llmspec.yaml`

## Tech Stack

- Read `package.json`, `pyproject.toml`, `Cargo.toml`, `go.mod`, `pom.xml`
- Extract: language, framework, major deps (ignore dev tooling like eslint, prettier)
- Note package manager: npm/yarn/pnpm/bun/pip/cargo/etc.

## Code Patterns

Read 3-5 representative files. Look for:
- Naming conventions (camelCase vs snake_case, file naming)
- File organisation (feature folders vs type folders)
- Common idioms (error handling style, async patterns, DI approach)
- Test file location and naming

## Symbols (store at all 4 levels)

For each module/file:
- L0: extract all exported function/class signatures
- L1: extract IO patterns (parameter types + return type)
- L2: summarise logic in 3-7 bullet steps
- L3: full source (reference path only, don't copy to index)

Tools to use: Grep for `export`, `def `, `func `, `pub fn`

## Dev Shortcuts

Read: `package.json scripts`, `Makefile`, `justfile`, `taskfile.yaml`, `scripts/` directory
Extract: dev, test, build, lint, index commands

## Documentation Layers

- Design/feature: `docs/`, `ADR/`, `docs/plans/`, `docs/decisions/`
- Public: `README.md`, `docs/guides/`, `CHANGELOG.md`, `docs/api/`
- Code: source root (for drift detection, not content indexing)

Record file paths + last-modified timestamps for drift detection.

## MCP Configs

Look for: `.mcp.json`, `mcp.config.json`, `.claude/mcp.json`, any MCP server references in README.

## What NOT to Index

- `node_modules/`, `dist/`, `.git/`, `coverage/`, `.cache/`
- Lock files (`package-lock.json`, `yarn.lock`)
- Generated files (`.d.ts` files, compiled output)
- Binary files, images, fonts
```

**Step 3: Commit**

```bash
git add skills/codebase-indexer/
git commit -m "feat: add llmspec template and extractor guide"
```

---

## Task 5: Write `codebase-indexer` skill

**Files:**
- Create: `skills/codebase-indexer/SKILL.md`

**Step 1: Write the skill**

```markdown
---
name: codebase-indexer
description: Use when starting work on an unfamiliar codebase, when a codebase has changed significantly, when an agent is doing broad file searches to orient itself, or when setting up a repo for efficient AI agent use.
---

# Codebase Indexer

## Overview

Scan a repo once, produce structured artifacts so future agents orient in ~500 tokens instead of hundreds of file reads. Outputs: `.llmspec.yaml`, `CLAUDE.md`, `llms.txt`, `.index/` directory, and project-scoped skills in `~/.claude/skills/<project>/`.

**REQUIRED:** Use `content-compression` skill to understand resolution levels before indexing.

## When to Run

- First time working on a repo
- After a major refactor or feature addition
- When an agent reports spending many turns on orientation
- Before running a benchmark

## Steps

**Step 1: Check for existing index**

```bash
ls .index/ 2>/dev/null && cat .llmspec.yaml 2>/dev/null
```

If index exists and is recent (< 7 days or no major commits since), call `get_llmspec()` MCP tool and stop — no need to re-index.

**Step 2: Run the indexer MCP tool**

```
call: reindex_repo({ path: ".", output: ".index/" })
```

This scans the repo using the extractor guide (`extractor.md`) and writes all output artifacts.

**Step 3: Review and fill gaps in `.llmspec.yaml`**

Auto-generated fields will be populated. Manually review and complete:
- `concepts` — domain terms that aren't obvious from code
- `patterns` — conventions that require judgment to identify
- `description` — one-sentence project summary

**Step 4: Verify outputs**

```bash
ls .index/          # symbol-map.json, patterns.md, shortcuts.md, stack.md, doc-index.json
cat llms.txt        # LLM-friendly summary
cat CLAUDE.md       # Project context for Claude Code
```

**Step 5: Commit index artifacts**

```bash
git add .llmspec.yaml llms.txt CLAUDE.md
git add .index/patterns.md .index/shortcuts.md .index/stack.md
# symbol-map.json and doc-index.json: add to .gitignore or commit based on team preference
git commit -m "chore: add/update codebase index"
```

## Output Artifacts

| Artifact | Purpose | Where |
|---|---|---|
| `.llmspec.yaml` | Primary LLM orientation spec | Repo root |
| `llms.txt` | llmstxt.org standard summary | Repo root |
| `CLAUDE.md` | Claude Code project context | Repo root |
| `.index/symbol-map.json` | All exports at L0–L2 | Repo root |
| `.index/patterns.md` | Detected conventions | Repo root |
| `.index/shortcuts.md` | Dev commands | Repo root |
| `.index/stack.md` | Tech stack summary | Repo root |
| `.index/doc-index.json` | Doc layer fingerprints | Repo root |
| `~/.claude/skills/<project>/` | Project-scoped skills | Global |

## Re-indexing

Call `reindex_repo()` again any time. It diffs against the previous index and only re-processes changed files.
```

**Step 2: Commit**

```bash
git add skills/codebase-indexer/SKILL.md
git commit -m "feat: add codebase-indexer skill"
```

---

## Task 6: Set up MCP server project

**Files:**
- Create: `mcp/repo-index-server/package.json`
- Create: `mcp/repo-index-server/tsconfig.json`
- Create: `mcp/repo-index-server/src/index.ts`

**Step 1: Write `package.json`**

```json
{
  "name": "repo-index-server",
  "version": "0.1.0",
  "type": "module",
  "scripts": {
    "dev": "tsx watch src/index.ts",
    "build": "tsc",
    "start": "node dist/index.js",
    "test": "vitest run"
  },
  "dependencies": {
    "@modelcontextprotocol/sdk": "^1.0.0",
    "fast-glob": "^3.3.0",
    "js-yaml": "^4.1.0",
    "zod": "^3.22.0"
  },
  "devDependencies": {
    "@types/js-yaml": "^4.0.9",
    "@types/node": "^22.0.0",
    "tsx": "^4.0.0",
    "typescript": "^5.5.0",
    "vitest": "^2.0.0"
  }
}
```

**Step 2: Write `tsconfig.json`**

```json
{
  "compilerOptions": {
    "target": "ES2022",
    "module": "ESNext",
    "moduleResolution": "bundler",
    "outDir": "dist",
    "rootDir": "src",
    "strict": true,
    "esModuleInterop": true,
    "skipLibCheck": true
  },
  "include": ["src/**/*"],
  "exclude": ["node_modules", "dist"]
}
```

**Step 3: Write minimal `src/index.ts`**

```typescript
import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";

const server = new McpServer({
  name: "repo-index-server",
  version: "0.1.0",
});

// Tools registered in subsequent tasks

const transport = new StdioServerTransport();
await server.connect(transport);
```

**Step 4: Install dependencies**

```bash
cd mcp/repo-index-server && npm install
```

**Step 5: Verify it starts**

```bash
npm run dev
# Expected: server starts with no errors, waits for input
# Ctrl+C to stop
```

**Step 6: Commit**

```bash
cd ../..
git add mcp/repo-index-server/
git commit -m "chore: scaffold MCP server project"
```

---

## Task 7: MCP server — index reader module

**Files:**
- Create: `mcp/repo-index-server/src/index-reader.ts`
- Create: `mcp/repo-index-server/src/index-reader.test.ts`

The index reader loads `.llmspec.yaml` and `.index/` files. All other tools call this module.

**Step 1: Write failing tests**

```typescript
// src/index-reader.test.ts
import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { mkdirSync, writeFileSync, rmSync } from "fs";
import { join } from "path";
import { readLlmSpec, readSymbolMap, readIndexFile } from "./index-reader.js";

const TMP = "/tmp/test-repo";

beforeEach(() => {
  mkdirSync(join(TMP, ".index"), { recursive: true });
  writeFileSync(join(TMP, ".llmspec.yaml"), `
project: test-app
version: 1.0.0
description: A test project
stack: [typescript]
entry_points:
  - path: src/index.ts
    role: server entry
`);
  writeFileSync(join(TMP, ".index/symbol-map.json"), JSON.stringify({
    "src/auth.ts": {
      L0: ["login(email: string, password: string): Promise<User>"],
      L1: ["user = login(email, password)\n// returns: Promise<User | null>"],
      L2: ["validate credentials → fetch user → generate token → return user"],
    }
  }));
  writeFileSync(join(TMP, ".index/patterns.md"), "# Patterns\n\n- Use repository pattern for DB access");
});

afterEach(() => rmSync(TMP, { recursive: true, force: true }));

describe("readLlmSpec", () => {
  it("reads and parses .llmspec.yaml", async () => {
    const spec = await readLlmSpec(TMP);
    expect(spec.project).toBe("test-app");
    expect(spec.stack).toContain("typescript");
  });

  it("throws if .llmspec.yaml missing", async () => {
    await expect(readLlmSpec("/nonexistent")).rejects.toThrow("No .llmspec.yaml found");
  });
});

describe("readSymbolMap", () => {
  it("returns symbol map", async () => {
    const map = await readSymbolMap(TMP);
    expect(map["src/auth.ts"].L0).toHaveLength(1);
  });
});

describe("readIndexFile", () => {
  it("reads a named index file", async () => {
    const content = await readIndexFile(TMP, "patterns.md");
    expect(content).toContain("repository pattern");
  });

  it("returns null if file missing", async () => {
    const content = await readIndexFile(TMP, "nonexistent.md");
    expect(content).toBeNull();
  });
});
```

**Step 2: Run tests — expect failure**

```bash
cd mcp/repo-index-server && npx vitest run src/index-reader.test.ts
# Expected: FAIL — index-reader.ts does not exist
```

**Step 3: Implement `src/index-reader.ts`**

```typescript
import { readFile } from "fs/promises";
import { join } from "path";
import yaml from "js-yaml";
import type { LlmSpec, SymbolMap } from "./types.js";

export async function readLlmSpec(repoPath: string): Promise<LlmSpec> {
  const specPath = join(repoPath, ".llmspec.yaml");
  let raw: string;
  try {
    raw = await readFile(specPath, "utf-8");
  } catch {
    throw new Error(`No .llmspec.yaml found at ${specPath}. Run codebase-indexer first.`);
  }
  return yaml.load(raw) as LlmSpec;
}

export async function readSymbolMap(repoPath: string): Promise<SymbolMap> {
  const mapPath = join(repoPath, ".index/symbol-map.json");
  const raw = await readFile(mapPath, "utf-8");
  return JSON.parse(raw) as SymbolMap;
}

export async function readIndexFile(repoPath: string, filename: string): Promise<string | null> {
  const filePath = join(repoPath, ".index", filename);
  try {
    return await readFile(filePath, "utf-8");
  } catch {
    return null;
  }
}
```

**Step 4: Create `src/types.ts`**

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
}

export type ResolutionLevel = "L0" | "L1" | "L2" | "L3";

export interface SymbolEntry {
  L0: string[];
  L1: string[];
  L2: string[];
  // L3 is the raw file — never cached in symbol-map
}

export type SymbolMap = Record<string, SymbolEntry>;
```

**Step 5: Run tests — expect pass**

```bash
npx vitest run src/index-reader.test.ts
# Expected: PASS — 5 tests
```

**Step 6: Commit**

```bash
cd ../..
git add mcp/repo-index-server/src/
git commit -m "feat: add MCP index reader module"
```

---

## Task 8: MCP server — query tools

**Files:**
- Create: `mcp/repo-index-server/src/tools/query.ts`
- Create: `mcp/repo-index-server/src/tools/query.test.ts`
- Modify: `mcp/repo-index-server/src/index.ts`

Tools: `get_llmspec`, `get_file_context`, `list_exports`, `find_pattern`, `get_shortcuts`

**Step 1: Write failing tests**

```typescript
// src/tools/query.test.ts
import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { mkdirSync, writeFileSync, rmSync, readFileSync } from "fs";
import { join } from "path";
import { getLlmSpec, getFileContext, listExports, findPattern, getShortcuts } from "./query.js";

const TMP = "/tmp/test-query-repo";

beforeEach(() => {
  mkdirSync(join(TMP, ".index"), { recursive: true });
  mkdirSync(join(TMP, "src"), { recursive: true });
  writeFileSync(join(TMP, ".llmspec.yaml"), `
project: query-test
stack: [typescript]
description: test
shortcuts:
  dev: npm run dev
  test: npm test
`);
  writeFileSync(join(TMP, ".index/symbol-map.json"), JSON.stringify({
    "src/auth.ts": {
      L0: ["login(email: string): Promise<User>"],
      L1: ["user = login(email)\n// returns: Promise<User>"],
      L2: ["validate → fetch → return"],
    }
  }));
  writeFileSync(join(TMP, ".index/patterns.md"), "# Patterns\n\n## Repository Pattern\nAll DB access through repos.");
  writeFileSync(join(TMP, ".index/shortcuts.md"), "# Shortcuts\n\n- dev: npm run dev\n- test: npm test");
  writeFileSync(join(TMP, "src/auth.ts"), `export function login(email: string): Promise<User> {\n  return db.find(email);\n}`);
});

afterEach(() => rmSync(TMP, { recursive: true, force: true }));

describe("getLlmSpec", () => {
  it("returns full spec by default", async () => {
    const result = await getLlmSpec(TMP);
    expect(result).toContain("query-test");
  });

  it("returns named section only", async () => {
    const result = await getLlmSpec(TMP, "shortcuts");
    expect(result).toContain("npm run dev");
    expect(result).not.toContain("query-test");
  });
});

describe("getFileContext", () => {
  it("returns L0 signatures", async () => {
    const result = await getFileContext(TMP, "src/auth.ts", "L0");
    expect(result).toContain("login(email: string): Promise<User>");
    expect(result).not.toContain("db.find");
  });

  it("returns L3 full source", async () => {
    const result = await getFileContext(TMP, "src/auth.ts", "L3");
    expect(result).toContain("db.find(email)");
  });
});

describe("listExports", () => {
  it("lists all exports at L0", async () => {
    const result = await listExports(TMP);
    expect(result).toContain("src/auth.ts");
    expect(result).toContain("login(email: string): Promise<User>");
  });
});

describe("findPattern", () => {
  it("returns pattern content", async () => {
    const result = await findPattern(TMP, "Repository");
    expect(result).toContain("DB access through repos");
  });

  it("returns all patterns if no name given", async () => {
    const result = await findPattern(TMP);
    expect(result).toContain("Repository Pattern");
  });
});

describe("getShortcuts", () => {
  it("returns shortcuts", async () => {
    const result = await getShortcuts(TMP);
    expect(result).toContain("npm run dev");
  });
});
```

**Step 2: Run tests — expect failure**

```bash
cd mcp/repo-index-server && npx vitest run src/tools/query.test.ts
# Expected: FAIL
```

**Step 3: Implement `src/tools/query.ts`**

```typescript
import { readFile } from "fs/promises";
import { join } from "path";
import yaml from "js-yaml";
import { readLlmSpec, readSymbolMap, readIndexFile } from "../index-reader.js";
import type { ResolutionLevel } from "../types.js";

export async function getLlmSpec(repoPath: string, section?: string): Promise<string> {
  const spec = await readLlmSpec(repoPath);
  if (!section) return yaml.dump(spec);
  const value = (spec as Record<string, unknown>)[section];
  if (value === undefined) throw new Error(`Section '${section}' not found in llmspec`);
  return yaml.dump({ [section]: value });
}

export async function getFileContext(repoPath: string, filePath: string, level: ResolutionLevel): Promise<string> {
  if (level === "L3") {
    return readFile(join(repoPath, filePath), "utf-8");
  }
  const map = await readSymbolMap(repoPath);
  const entry = map[filePath];
  if (!entry) throw new Error(`File '${filePath}' not in symbol map. Run reindex_repo first.`);
  return entry[level].join("\n");
}

export async function listExports(repoPath: string, module?: string): Promise<string> {
  const map = await readSymbolMap(repoPath);
  const lines: string[] = [];
  for (const [file, entry] of Object.entries(map)) {
    if (module && !file.startsWith(module)) continue;
    lines.push(`\n### ${file}`);
    lines.push(...entry.L0);
  }
  return lines.join("\n");
}

export async function findPattern(repoPath: string, name?: string): Promise<string> {
  const content = await readIndexFile(repoPath, "patterns.md");
  if (!content) return "No patterns indexed. Run reindex_repo first.";
  if (!name) return content;
  const lines = content.split("\n");
  const start = lines.findIndex(l => l.toLowerCase().includes(name.toLowerCase()));
  if (start === -1) return `Pattern '${name}' not found.`;
  const end = lines.findIndex((l, i) => i > start && l.startsWith("## "));
  return lines.slice(start, end === -1 ? undefined : end).join("\n");
}

export async function getShortcuts(repoPath: string): Promise<string> {
  const content = await readIndexFile(repoPath, "shortcuts.md");
  if (!content) {
    const spec = await readLlmSpec(repoPath);
    return yaml.dump(spec.shortcuts);
  }
  return content;
}
```

**Step 4: Run tests — expect pass**

```bash
npx vitest run src/tools/query.test.ts
# Expected: PASS
```

**Step 5: Register tools in `src/index.ts`**

```typescript
import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import { z } from "zod";
import { getLlmSpec, getFileContext, listExports, findPattern, getShortcuts } from "./tools/query.js";

const server = new McpServer({ name: "repo-index-server", version: "0.1.0" });

const REPO = process.env.REPO_PATH ?? process.cwd();

server.tool("get_llmspec", "Get the LLM spec for this repo, optionally a specific section",
  { section: z.string().optional() },
  async ({ section }) => ({ content: [{ type: "text", text: await getLlmSpec(REPO, section) }] })
);

server.tool("get_file_context", "Get a file at a specific resolution level (L0-L3)",
  { path: z.string(), level: z.enum(["L0", "L1", "L2", "L3"]) },
  async ({ path, level }) => ({ content: [{ type: "text", text: await getFileContext(REPO, path, level) }] })
);

server.tool("list_exports", "List all exports at L0, optionally scoped to a module path",
  { module: z.string().optional() },
  async ({ module }) => ({ content: [{ type: "text", text: await listExports(REPO, module) }] })
);

server.tool("find_pattern", "Find a named pattern or list all patterns",
  { name: z.string().optional() },
  async ({ name }) => ({ content: [{ type: "text", text: await findPattern(REPO, name) }] })
);

server.tool("get_shortcuts", "Get dev shortcuts and commands",
  {},
  async () => ({ content: [{ type: "text", text: await getShortcuts(REPO) }] })
);

const transport = new StdioServerTransport();
await server.connect(transport);
```

**Step 6: Commit**

```bash
cd ../..
git add mcp/repo-index-server/src/
git commit -m "feat: add MCP query tools (get_llmspec, get_file_context, list_exports, find_pattern, get_shortcuts)"
```

---

## Task 9: MCP server — reindex tool (core indexer)

**Files:**
- Create: `mcp/repo-index-server/src/tools/reindex.ts`
- Create: `mcp/repo-index-server/src/tools/reindex.test.ts`
- Modify: `mcp/repo-index-server/src/index.ts`

The reindex tool scans a repo and writes all `.index/` artifacts + `.llmspec.yaml` template if missing.

**Step 1: Write failing tests**

```typescript
// src/tools/reindex.test.ts
import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { mkdirSync, writeFileSync, rmSync, existsSync } from "fs";
import { join } from "path";
import { reindexRepo } from "./reindex.js";

const TMP = "/tmp/test-reindex-repo";

beforeEach(() => {
  mkdirSync(join(TMP, "src"), { recursive: true });
  writeFileSync(join(TMP, "package.json"), JSON.stringify({
    name: "test-app", version: "1.0.0",
    scripts: { dev: "tsx src/index.ts", test: "vitest" },
    dependencies: { express: "^4.0.0" }
  }));
  writeFileSync(join(TMP, "src/index.ts"), `
import express from 'express';
export const app = express();
export function startServer(port: number): void { app.listen(port); }
`);
  writeFileSync(join(TMP, "README.md"), "# Test App\nA test application.");
});

afterEach(() => rmSync(TMP, { recursive: true, force: true }));

describe("reindexRepo", () => {
  it("creates .index directory", async () => {
    await reindexRepo(TMP);
    expect(existsSync(join(TMP, ".index"))).toBe(true);
  });

  it("writes symbol-map.json with L0 entries", async () => {
    await reindexRepo(TMP);
    const map = JSON.parse(require("fs").readFileSync(join(TMP, ".index/symbol-map.json"), "utf-8"));
    expect(Object.keys(map).length).toBeGreaterThan(0);
  });

  it("writes stack.md with detected stack", async () => {
    await reindexRepo(TMP);
    const stack = require("fs").readFileSync(join(TMP, ".index/stack.md"), "utf-8");
    expect(stack).toContain("express");
  });

  it("writes shortcuts.md from package.json scripts", async () => {
    await reindexRepo(TMP);
    const shortcuts = require("fs").readFileSync(join(TMP, ".index/shortcuts.md"), "utf-8");
    expect(shortcuts).toContain("tsx src/index.ts");
  });

  it("creates .llmspec.yaml template if missing", async () => {
    await reindexRepo(TMP);
    expect(existsSync(join(TMP, ".llmspec.yaml"))).toBe(true);
  });

  it("does not overwrite existing .llmspec.yaml", async () => {
    writeFileSync(join(TMP, ".llmspec.yaml"), "project: my-custom-project\n");
    await reindexRepo(TMP);
    const content = require("fs").readFileSync(join(TMP, ".llmspec.yaml"), "utf-8");
    expect(content).toContain("my-custom-project");
  });
});
```

**Step 2: Run tests — expect failure**

```bash
cd mcp/repo-index-server && npx vitest run src/tools/reindex.test.ts
# Expected: FAIL
```

**Step 3: Implement `src/tools/reindex.ts`**

```typescript
import { readFile, writeFile, mkdir, readdir, stat } from "fs/promises";
import { join, relative, extname } from "path";
import { existsSync } from "fs";
import yaml from "js-yaml";
import fg from "fast-glob";
import type { SymbolMap, LlmSpec } from "../types.js";

const IGNORE = ["node_modules", "dist", ".git", "coverage", ".cache", ".index"];
const CODE_EXTS = [".ts", ".tsx", ".js", ".jsx", ".py", ".go", ".rs"];

export async function reindexRepo(repoPath: string): Promise<void> {
  await mkdir(join(repoPath, ".index"), { recursive: true });

  const [stack, shortcuts, symbolMap, docFingerprints] = await Promise.all([
    detectStack(repoPath),
    detectShortcuts(repoPath),
    buildSymbolMap(repoPath),
    buildDocIndex(repoPath),
  ]);

  await Promise.all([
    writeFile(join(repoPath, ".index/stack.md"), formatStack(stack)),
    writeFile(join(repoPath, ".index/shortcuts.md"), formatShortcuts(shortcuts)),
    writeFile(join(repoPath, ".index/symbol-map.json"), JSON.stringify(symbolMap, null, 2)),
    writeFile(join(repoPath, ".index/doc-index.json"), JSON.stringify(docFingerprints, null, 2)),
    writeFile(join(repoPath, ".index/patterns.md"), "# Patterns\n\n<!-- Auto-generated: review and expand -->\n"),
  ]);

  if (!existsSync(join(repoPath, ".llmspec.yaml"))) {
    await writeFile(join(repoPath, ".llmspec.yaml"), generateLlmSpecTemplate(repoPath, stack, shortcuts));
  }

  await generateLlmsTxt(repoPath);
  await generateClaudeMd(repoPath);
}

async function detectStack(repoPath: string): Promise<Record<string, string[]>> {
  const stack: Record<string, string[]> = { languages: [], frameworks: [], tools: [] };
  const pkgPath = join(repoPath, "package.json");
  if (existsSync(pkgPath)) {
    const pkg = JSON.parse(await readFile(pkgPath, "utf-8"));
    stack.languages.push("typescript/javascript");
    const deps = { ...pkg.dependencies, ...pkg.devDependencies };
    const frameworks = ["react", "next", "vue", "svelte", "express", "fastify", "hono", "nest"];
    frameworks.forEach(f => { if (deps[f] || deps[`@${f}`]) stack.frameworks.push(f); });
    if (deps["tsx"] || deps["ts-node"]) stack.tools.push("tsx");
    if (deps["vitest"]) stack.tools.push("vitest");
    if (deps["jest"]) stack.tools.push("jest");
  }
  if (existsSync(join(repoPath, "pyproject.toml")) || existsSync(join(repoPath, "requirements.txt"))) {
    stack.languages.push("python");
  }
  return stack;
}

async function detectShortcuts(repoPath: string): Promise<Record<string, string>> {
  const shortcuts: Record<string, string> = {};
  const pkgPath = join(repoPath, "package.json");
  if (existsSync(pkgPath)) {
    const pkg = JSON.parse(await readFile(pkgPath, "utf-8"));
    return pkg.scripts ?? {};
  }
  return shortcuts;
}

async function buildSymbolMap(repoPath: string): Promise<SymbolMap> {
  const files = await fg(CODE_EXTS.map(e => `**/*${e}`), {
    cwd: repoPath, ignore: IGNORE, absolute: false
  });
  const map: SymbolMap = {};
  await Promise.all(files.map(async (file) => {
    const content = await readFile(join(repoPath, file), "utf-8");
    const exports = extractExports(content, file);
    if (exports.L0.length > 0) map[file] = exports;
  }));
  return map;
}

function extractExports(content: string, file: string): { L0: string[]; L1: string[]; L2: string[] } {
  const L0: string[] = [];
  const lines = content.split("\n");
  // Match: export function/class/const/type/interface
  const exportRe = /^export\s+(async\s+)?(function|class|const|type|interface|enum)\s+(\w+)([^{;]*)/;
  for (const line of lines) {
    const m = line.match(exportRe);
    if (m) {
      const signature = line.replace(/\{.*/, "").replace(/=\s*$/, "").trim();
      L0.push(signature);
    }
  }
  return { L0, L1: L0.map(s => `// ${s}`), L2: [] };
}

async function buildDocIndex(repoPath: string): Promise<Record<string, { mtime: number; size: number }>> {
  const docExts = [".md", ".mdx", ".txt", ".yaml", ".yml"];
  const files = await fg(docExts.map(e => `**/*${e}`), {
    cwd: repoPath, ignore: IGNORE, absolute: false
  });
  const index: Record<string, { mtime: number; size: number }> = {};
  await Promise.all(files.map(async (file) => {
    const s = await stat(join(repoPath, file));
    index[file] = { mtime: s.mtimeMs, size: s.size };
  }));
  return index;
}

function formatStack(stack: Record<string, string[]>): string {
  return `# Tech Stack\n\n` +
    Object.entries(stack).filter(([, v]) => v.length).map(([k, v]) => `## ${k}\n${v.map(x => `- ${x}`).join("\n")}`).join("\n\n");
}

function formatShortcuts(shortcuts: Record<string, string>): string {
  return `# Shortcuts\n\n` +
    Object.entries(shortcuts).map(([k, v]) => `- **${k}**: \`${v}\``).join("\n");
}

function generateLlmSpecTemplate(repoPath: string, stack: Record<string, string[]>, shortcuts: Record<string, string>): string {
  const name = repoPath.split("/").at(-1) ?? "project";
  return yaml.dump({
    project: name, version: "0.0.0", description: "TODO: one-sentence project summary",
    stack: [...stack.languages, ...stack.frameworks],
    entry_points: [{ path: "src/index.ts", role: "TODO: describe role" }],
    concepts: [], patterns: [], api_surface: [],
    doc_layers: { design: "docs/", code: "src/", public: ["README.md"] },
    shortcuts,
  });
}

async function generateLlmsTxt(repoPath: string): Promise<void> {
  const readmePath = join(repoPath, "README.md");
  const readme = existsSync(readmePath) ? await readFile(readmePath, "utf-8") : "";
  const llmsTxt = `# ${repoPath.split("/").at(-1)}\n\n${readme.split("\n").slice(0, 20).join("\n")}\n\n> Generated by codebase-indexer\n`;
  await writeFile(join(repoPath, "llms.txt"), llmsTxt);
}

async function generateClaudeMd(repoPath: string): Promise<void> {
  const claudeMdPath = join(repoPath, "CLAUDE.md");
  if (existsSync(claudeMdPath)) return; // Never overwrite
  const content = `# Project Context\n\n> Auto-generated by codebase-indexer. Update as needed.\n\n## Stack\nSee \`.index/stack.md\`\n\n## Shortcuts\nSee \`.index/shortcuts.md\`\n\n## Patterns\nSee \`.index/patterns.md\`\n`;
  await writeFile(claudeMdPath, content);
}
```

**Step 4: Run tests — expect pass**

```bash
npx vitest run src/tools/reindex.test.ts
# Expected: PASS
```

**Step 5: Register `reindex_repo` tool in `src/index.ts`**

Add after existing tools:
```typescript
import { reindexRepo } from "./tools/reindex.js";

server.tool("reindex_repo", "Scan a repo and build/update the index artifacts",
  { path: z.string().optional() },
  async ({ path }) => {
    const target = path ?? REPO;
    await reindexRepo(target);
    return { content: [{ type: "text", text: `Indexed ${target}. Artifacts written to .index/` }] };
  }
);
```

**Step 6: Commit**

```bash
cd ../..
git add mcp/repo-index-server/src/
git commit -m "feat: add reindex_repo MCP tool with symbol extraction"
```

---

## Task 10: MCP server — context and drift tools

**Files:**
- Create: `mcp/repo-index-server/src/tools/context.ts`
- Create: `mcp/repo-index-server/src/tools/drift.ts`
- Create: `mcp/repo-index-server/src/tools/context.test.ts`
- Create: `mcp/repo-index-server/src/tools/drift.test.ts`
- Modify: `mcp/repo-index-server/src/index.ts`

**Step 1: Write failing drift test**

```typescript
// src/tools/drift.test.ts
import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { mkdirSync, writeFileSync, rmSync } from "fs";
import { join } from "path";
import { checkDrift } from "./drift.js";

const TMP = "/tmp/test-drift-repo";

beforeEach(() => {
  mkdirSync(join(TMP, ".index"), { recursive: true });
  mkdirSync(join(TMP, "docs"), { recursive: true });
  writeFileSync(join(TMP, "README.md"), "# App");
  writeFileSync(join(TMP, "docs/design.md"), "# Design");
  // Fingerprint from 10 seconds ago
  const past = Date.now() - 10_000;
  writeFileSync(join(TMP, ".index/doc-index.json"), JSON.stringify({
    "README.md": { mtime: past, size: 5 },
    "docs/design.md": { mtime: past, size: 8 },
  }));
});

afterEach(() => rmSync(TMP, { recursive: true, force: true }));

describe("checkDrift", () => {
  it("reports no drift when files unchanged", async () => {
    // Write index with current mtimes
    const { statSync } = await import("fs");
    const readmeS = statSync(join(TMP, "README.md"));
    const designS = statSync(join(TMP, "docs/design.md"));
    writeFileSync(join(TMP, ".index/doc-index.json"), JSON.stringify({
      "README.md": { mtime: readmeS.mtimeMs, size: readmeS.size },
      "docs/design.md": { mtime: designS.mtimeMs, size: designS.size },
    }));
    const result = await checkDrift(TMP);
    expect(result.drifted).toHaveLength(0);
  });

  it("reports drift when a file has been modified", async () => {
    const result = await checkDrift(TMP);
    expect(result.drifted.length).toBeGreaterThan(0);
    expect(result.drifted[0]).toContain("modified");
  });

  it("reports missing files", async () => {
    writeFileSync(join(TMP, ".index/doc-index.json"), JSON.stringify({
      "README.md": { mtime: Date.now(), size: 5 },
      "docs/deleted.md": { mtime: Date.now(), size: 100 },
    }));
    const result = await checkDrift(TMP);
    expect(result.drifted.some(d => d.includes("deleted.md"))).toBe(true);
  });
});
```

**Step 2: Run drift test — expect failure**

```bash
cd mcp/repo-index-server && npx vitest run src/tools/drift.test.ts
# Expected: FAIL
```

**Step 3: Implement `src/tools/drift.ts`**

```typescript
import { stat, readFile } from "fs/promises";
import { join } from "path";
import { existsSync } from "fs";
import fg from "fast-glob";

interface DriftResult {
  drifted: string[];
  summary: string;
}

export async function checkDrift(repoPath: string): Promise<DriftResult> {
  const indexPath = join(repoPath, ".index/doc-index.json");
  if (!existsSync(indexPath)) {
    return { drifted: [], summary: "No doc-index.json found. Run reindex_repo first." };
  }

  const stored: Record<string, { mtime: number; size: number }> = JSON.parse(
    await readFile(indexPath, "utf-8")
  );

  const drifted: string[] = [];

  for (const [file, fingerprint] of Object.entries(stored)) {
    const fullPath = join(repoPath, file);
    if (!existsSync(fullPath)) {
      drifted.push(`${file}: deleted (was in index)`);
      continue;
    }
    const current = await stat(fullPath);
    if (Math.abs(current.mtimeMs - fingerprint.mtime) > 1000 || current.size !== fingerprint.size) {
      drifted.push(`${file}: modified since last index`);
    }
  }

  const summary = drifted.length === 0
    ? "No drift detected. All indexed docs match current state."
    : `${drifted.length} file(s) drifted since last index:\n${drifted.join("\n")}`;

  return { drifted, summary };
}
```

**Step 4: Run drift test — expect pass**

```bash
npx vitest run src/tools/drift.test.ts
# Expected: PASS
```

**Step 5: Implement `src/tools/context.ts`**

```typescript
import { readFile, writeFile, mkdir } from "fs/promises";
import { join } from "path";
import { existsSync } from "fs";
import { readLlmSpec, readSymbolMap } from "../index-reader.js";

interface ContextSlice {
  scope: string;
  content: string;
  tokenEstimate: number;
}

const CHECKPOINTS_DIR = ".index/checkpoints";

export async function loadContext(repoPath: string, scope: string): Promise<ContextSlice> {
  const spec = await readLlmSpec(repoPath);
  let content = "";

  if (scope === "orientation") {
    content = `# ${spec.project}\n${spec.description}\n\nStack: ${spec.stack.join(", ")}\n\nEntry points:\n` +
      spec.entry_points.map(e => `- ${e.path}: ${e.role}`).join("\n");
  } else if (scope === "patterns") {
    const patterns = await readFile(join(repoPath, ".index/patterns.md"), "utf-8").catch(() => "No patterns indexed");
    content = patterns;
  } else {
    // Scope is a module path prefix
    const map = await readSymbolMap(repoPath);
    const relevant = Object.entries(map).filter(([f]) => f.startsWith(scope));
    content = relevant.map(([f, e]) => `### ${f}\n${e.L0.join("\n")}`).join("\n\n");
  }

  return { scope, content, tokenEstimate: Math.ceil(content.length / 4) };
}

export async function checkpoint(repoPath: string, name?: string): Promise<string> {
  const checkpointPath = join(repoPath, CHECKPOINTS_DIR);
  await mkdir(checkpointPath, { recursive: true });
  const id = name ?? `checkpoint-${Date.now()}`;
  await writeFile(join(checkpointPath, `${id}.json`), JSON.stringify({ id, timestamp: Date.now() }));
  return `Checkpoint '${id}' saved.`;
}

export async function recommendNext(repoPath: string, task: string): Promise<string> {
  const lower = task.toLowerCase();
  if (lower.includes("bug") || lower.includes("fix") || lower.includes("error")) {
    return "Recommended: load L2 for the relevant module, then L3 for the specific function. Use get_file_context(path, 'L2') first.";
  }
  if (lower.includes("understand") || lower.includes("explain") || lower.includes("how")) {
    return "Recommended: load orientation slice first (load_context('orientation')), then L1 or L2 for relevant modules.";
  }
  if (lower.includes("list") || lower.includes("find") || lower.includes("search")) {
    return "Recommended: use list_exports() at L0. No full file loads needed.";
  }
  if (lower.includes("add") || lower.includes("implement") || lower.includes("create")) {
    return "Recommended: load patterns (load_context('patterns')), then L0 for related modules, then L3 only for the specific files you will edit.";
  }
  return "Recommended: start with get_llmspec() for orientation (~500 tokens), then load targeted slices as needed.";
}
```

**Step 6: Register tools in `src/index.ts`**

```typescript
import { loadContext, checkpoint, recommendNext } from "./tools/context.js";
import { checkDrift } from "./tools/drift.js";

server.tool("check_drift", "Check if indexed docs have drifted from the current state",
  {},
  async () => {
    const result = await checkDrift(REPO);
    return { content: [{ type: "text", text: result.summary }] };
  }
);

server.tool("load_context", "Load a targeted context slice by scope (orientation, patterns, or module path)",
  { scope: z.string() },
  async ({ scope }) => {
    const slice = await loadContext(REPO, scope);
    return { content: [{ type: "text", text: `# Context: ${slice.scope}\n~${slice.tokenEstimate} tokens\n\n${slice.content}` }] };
  }
);

server.tool("checkpoint", "Save current context state",
  { name: z.string().optional() },
  async ({ name }) => {
    const msg = await checkpoint(REPO, name);
    return { content: [{ type: "text", text: msg }] };
  }
);

server.tool("recommend_next", "Get recommended minimal context for a given task",
  { task: z.string() },
  async ({ task }) => {
    const rec = await recommendNext(REPO, task);
    return { content: [{ type: "text", text: rec }] };
  }
);
```

**Step 7: Run all tests**

```bash
cd mcp/repo-index-server && npx vitest run
# Expected: all PASS
```

**Step 8: Commit**

```bash
cd ../..
git add mcp/repo-index-server/src/
git commit -m "feat: add context and drift detection MCP tools"
```

---

## Task 11: Write `doc-drift-detector` and `context-manager` skills

**Files:**
- Create: `skills/doc-drift-detector/SKILL.md`
- Create: `skills/context-manager/SKILL.md`

**Step 1: Write `doc-drift-detector/SKILL.md`**

```markdown
---
name: doc-drift-detector
description: Use when documentation and code may be out of sync, before releasing or merging, when onboarding to a codebase and unsure if docs are current, or when setting up CI checks for documentation freshness.
---

# Doc Drift Detector

## Overview

Tracks three doc layers — design docs, code, and public docs — against a stored fingerprint index. Reports files that have changed since the last index run.

## Three Doc Layers

| Layer | Contents | Examples |
|---|---|---|
| Design/feature | Architecture decisions, feature specs, plans | `docs/plans/`, `ADR/`, `docs/decisions/` |
| Code | Source files (for API surface tracking) | `src/` |
| Public | User-facing documentation | `README.md`, `docs/guides/`, `CHANGELOG.md` |

**Drift happens when:** one layer changes without the others updating to match.

## Detecting Drift

**On demand:**
```
call: check_drift()
```
Returns a list of files modified since last index + a summary.

**As a pre-commit hook** — add to `.git/hooks/pre-commit`:
```bash
#!/bin/sh
npx repo-index-server check-drift --fail-on-drift
```

**In CI** — add to pipeline:
```bash
npx repo-index-server check-drift --output results/drift-$(date +%Y%m%d).json
```

## Resolving Drift

1. Call `check_drift()` to see what drifted
2. Review each drifted file
3. Update the appropriate other doc layers to match
4. Call `reindex_repo()` to update fingerprints
5. Commit updated docs + new index

## Common Mistakes

| Mistake | Fix |
|---|---|
| Updating code without updating design docs | Check drift before merging PRs |
| Updating public docs without updating design docs | Design docs are source of truth — update them first |
| Ignoring drift on "small" changes | Small undocumented changes accumulate into large drift |
```

**Step 2: Write `context-manager/SKILL.md`**

```markdown
---
name: context-manager
description: Use when an agent is loading too many files, using too many tokens for orientation, switching between tasks without clearing context, or when context window is filling up with stale content.
---

# Context Manager

## Overview

Load narrow, offload often. The context window is a working memory — keep only what the current task needs. Everything else lives in the index.

**REQUIRED:** MCP server (`repo-index-server`) must be running and indexed.

## Session Protocol

**Start of session:**
```
1. call: get_llmspec()          → ~500 tokens, full orientation
2. call: recommend_next(task)   → get minimal context prescription
3. call: load_context(scope)    → load only the prescribed slice
```

**During work:**
```
- Need a function signature?  → get_file_context(path, "L0")
- Need to understand logic?   → get_file_context(path, "L2")
- Need to edit code?          → get_file_context(path, "L3")
- Need to find something?     → query_index(query) or find_pattern(name)
- Never: read full files to browse
```

**Switching tasks:**
```
1. call: checkpoint()           → save state, clear mental model
2. call: recommend_next(task)   → new prescription
3. call: load_context(scope)    → fresh targeted slice
```

## Token Budget Guidelines

| Operation | Token cost | When justified |
|---|---|---|
| `get_llmspec()` | ~500 | Once per session |
| `load_context("orientation")` | ~200 | First task only |
| `get_file_context(path, "L0")` | ~50 | Any discovery |
| `get_file_context(path, "L2")` | ~150 | Understanding flow |
| `get_file_context(path, "L3")` | ~500–2000 | Editing only |
| Reading full repo file tree | ~2000+ | Never — use `list_exports()` |

## Anti-Patterns

| Anti-pattern | Fix |
|---|---|
| Loading full files to find a function | `list_exports(module)` at L0, then `L3` for target only |
| Keeping previous task's files in context | `checkpoint()` before switching |
| Using grep/glob to explore | `query_index()` or `find_pattern()` |
| Loading files "just in case" | Load on demand only — YAGNI for context |
```

**Step 3: Commit**

```bash
git add skills/doc-drift-detector/ skills/context-manager/
git commit -m "feat: add doc-drift-detector and context-manager skills"
```

---

## Task 12: Write `benchmark-runner` skill and task corpus

**Files:**
- Create: `skills/benchmark-runner/SKILL.md`
- Create: `tasks/sample.yaml`

**Step 1: Write `benchmark-runner/SKILL.md`**

```markdown
---
name: benchmark-runner
description: Use when evaluating the impact of skills on agent efficiency, comparing token usage with and without skills, measuring interaction counts, or validating improvements after updating skills.
---

# Benchmark Runner

## Overview

Run a representative task corpus against two configurations (with-skills vs without-skills) and compare token usage, interaction counts, and task completion.

## Setup

**Two branches required:**
```bash
git checkout -b benchmark/with-skills
# ensure skills installed, MCP running, repo indexed

git checkout -b benchmark/without-skills
# uninstall skills, stop MCP, remove .index/
```

## Running a Benchmark

```bash
cd mcp/repo-index-server
npx tsx src/benchmark.ts \
  --corpus ../../tasks/sample.yaml \
  --config-a "with-skills" \
  --config-b "without-skills" \
  --output ../../results/$(date +%Y-%m-%d)-benchmark.json
```

## Metrics Captured

| Metric | What it measures |
|---|---|
| `tokens_in` | Prompt tokens consumed |
| `tokens_out` | Completion tokens generated |
| `interactions` | Turns taken to complete task |
| `tool_calls` | File reads, searches, MCP calls |
| `success` | Task completed correctly (boolean) |
| `drift_score` | Doc sync state after task (0–1) |

## Task Categories

Tasks in `tasks/sample.yaml` cover:
- **orientation** — "explain the auth flow"
- **bug-fix** — "fix the failing test in X"
- **feature-add** — "add a Y field to the Z type"
- **doc-update** — "update README to reflect new CLI flags"
- **refactor** — "extract validation logic from processOrder"

## Interpreting Results

Good skill impact shows:
- 40–80% reduction in `tokens_in` for orientation tasks
- 50%+ reduction in `tool_calls` (fewer file reads)
- Same or better `success` rate
- Fewer `interactions` to complete

## Improvement Loop

```
run benchmark → identify weak task categories →
improve relevant skill → re-run benchmark →
confirm improvement → commit
```
```

**Step 2: Write `tasks/sample.yaml`**

```yaml
# Benchmark task corpus
# Representative developer tasks for evaluating skill impact
version: "1.0"

tasks:
  - id: orient-01
    category: orientation
    prompt: "Explain the overall architecture of this codebase. What are the main components and how do they interact?"
    success_criteria: "Mentions entry points, key modules, and data flow"

  - id: orient-02
    category: orientation
    prompt: "What tech stack does this project use? What are the main dependencies?"
    success_criteria: "Lists correct stack and major dependencies"

  - id: orient-03
    category: orientation
    prompt: "What commands do I need to run to start development and run tests?"
    success_criteria: "Provides correct dev and test commands"

  - id: find-01
    category: discovery
    prompt: "List all exported functions in the auth module."
    success_criteria: "Returns correct function signatures without loading full files"

  - id: find-02
    category: discovery
    prompt: "Find where the database connection is configured."
    success_criteria: "Identifies correct file and line range"

  - id: understand-01
    category: understanding
    prompt: "Walk me through the login flow step by step."
    success_criteria: "Describes correct sequence of operations"

  - id: understand-02
    category: understanding
    prompt: "What does the processOrder function return and what does it expect as input?"
    success_criteria: "Correct IO description without loading full source"

  - id: edit-01
    category: feature-add
    prompt: "Add a 'discount' field (number, optional) to the Order type and update the processOrder function to apply it."
    success_criteria: "Correct type update, correct function update, no regressions"

  - id: edit-02
    category: refactor
    prompt: "Extract the input validation logic from processOrder into a separate validateOrderInput function."
    success_criteria: "Correct extraction, same behaviour, passing tests"

  - id: doc-01
    category: doc-update
    prompt: "Update the README to document the new CLI flags added in the latest release."
    success_criteria: "README updated, content matches code"

  - id: drift-01
    category: drift-check
    prompt: "Check if the documentation is in sync with the current code."
    success_criteria: "Drift report produced, identifies any mismatches"
```

**Step 3: Commit**

```bash
git add skills/benchmark-runner/ tasks/
git commit -m "feat: add benchmark-runner skill and sample task corpus"
```

---

## Task 13: Write install script

**Files:**
- Create: `install.sh`

**Step 1: Write `install.sh`**

```bash
#!/usr/bin/env bash
set -e

SKILLS_DIR="$(cd "$(dirname "$0")/skills" && pwd)"
CLAUDE_SKILLS_DIR="$HOME/.claude/skills"
CLAUDE_MCP_CONFIG="$HOME/.claude/mcp.json"
MCP_SERVER_DIR="$(cd "$(dirname "$0")/mcp/repo-index-server" && pwd)"

usage() {
  echo "Usage: ./install.sh [--claude] [--all] [--uninstall]"
  echo ""
  echo "  --claude     Install skills for Claude Code + register MCP server"
  echo "  --all        Install for all supported agents"
  echo "  --uninstall  Remove installed skills and MCP registration"
  exit 1
}

install_skills() {
  local target="$1"
  echo "Installing skills to $target..."
  mkdir -p "$target"
  for skill_dir in "$SKILLS_DIR"/*/; do
    skill_name=$(basename "$skill_dir")
    link_path="$target/$skill_name"
    if [ -L "$link_path" ]; then
      rm "$link_path"
    fi
    ln -s "$skill_dir" "$link_path"
    echo "  Linked: $skill_name"
  done
}

install_mcp() {
  echo "Building MCP server..."
  cd "$MCP_SERVER_DIR"
  npm install --silent
  npm run build
  cd - > /dev/null

  echo "Registering MCP server in $CLAUDE_MCP_CONFIG..."
  mkdir -p "$(dirname "$CLAUDE_MCP_CONFIG")"

  if [ ! -f "$CLAUDE_MCP_CONFIG" ]; then
    echo '{"mcpServers":{}}' > "$CLAUDE_MCP_CONFIG"
  fi

  # Use node to safely merge JSON
  node -e "
    const fs = require('fs');
    const config = JSON.parse(fs.readFileSync('$CLAUDE_MCP_CONFIG', 'utf-8'));
    config.mcpServers = config.mcpServers ?? {};
    config.mcpServers['repo-index-server'] = {
      command: 'node',
      args: ['$MCP_SERVER_DIR/dist/index.js'],
      env: { REPO_PATH: process.cwd() }
    };
    fs.writeFileSync('$CLAUDE_MCP_CONFIG', JSON.stringify(config, null, 2));
  "
  echo "  MCP server registered."
}

uninstall() {
  echo "Uninstalling..."
  for skill_dir in "$SKILLS_DIR"/*/; do
    skill_name=$(basename "$skill_dir")
    rm -f "$CLAUDE_SKILLS_DIR/$skill_name"
  done
  echo "  Skills removed."
}

if [ $# -eq 0 ]; then usage; fi

case "$1" in
  --claude|--all)
    install_skills "$CLAUDE_SKILLS_DIR"
    install_mcp
    echo ""
    echo "Done. Restart Claude Code to pick up new skills and MCP server."
    ;;
  --uninstall)
    uninstall
    ;;
  *)
    usage
    ;;
esac
```

**Step 2: Make it executable**

```bash
chmod +x install.sh
```

**Step 3: Dry-run verify**

```bash
bash -n install.sh
# Expected: no syntax errors
```

**Step 4: Commit**

```bash
git add install.sh
git commit -m "feat: add install script for skills and MCP server"
```

---

## Task 14: Final verification

**Step 1: Run all MCP server tests**

```bash
cd mcp/repo-index-server && npx vitest run
# Expected: all tests PASS
```

**Step 2: Test MCP server starts**

```bash
npm run build && node dist/index.js &
sleep 1 && kill %1
# Expected: starts without errors
```

**Step 3: Verify skill files exist and have valid frontmatter**

```bash
for skill in ../../skills/*/SKILL.md; do
  echo "=== $skill ==="
  head -5 "$skill"
  echo ""
done
```

**Step 4: Verify install script is valid**

```bash
cd ../.. && bash -n install.sh
```

**Step 5: Final commit**

```bash
git add -A
git commit -m "chore: final verification pass"
```

**Step 6: Summary**

After all tasks complete, the repo contains:
- 6 skills: `codebase-indexer`, `content-compression`, `agentic-dev-workflow`, `doc-drift-detector`, `context-manager`, `benchmark-runner`
- 1 MCP server with 12 tools across query, reindex, context, drift, and generation categories
- Benchmark task corpus (`tasks/sample.yaml`)
- Install script (`install.sh`)
- Design doc (`docs/plans/2026-03-06-ai-skills-repo-design.md`)
