# Monorepo Package Split Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Split `packages/sensei/` into four focused packages — `shared` (types/constants), `server` (inference engine), `mcp` (MCP tool server), and `cli` (thin CLI binary) — matching the architecture in `docs/design/14-server-package.md`.

**Architecture:** `packages/shared/` has no runtime deps and is the foundation. `packages/server/` and `packages/mcp/` each depend on shared. `packages/cli/` depends on all three. Each package is created first, then `packages/sensei/` is updated to import from it, then the moved code is deleted from sensei. Final step renames sensei → cli.

**Tech Stack:** Bun workspaces, TypeScript `"moduleResolution": "bundler"`, Vitest, `workspace:*` protocol for cross-package deps.

---

## Background: Current Structure

Everything lives in `packages/sensei/src/`:

```
src/
  index.ts          ← MCP server entry (19 tools, will → packages/mcp/)
  cli.ts            ← CLI entry (stays → packages/cli/)
  types.ts          ← LlmSpec, SymbolMap, etc. (will → packages/shared/)
  constants.ts      ← SENSEI_DIR, senseiPath (will → packages/shared/)
  index-reader.ts   ← reads .sensei/ files (will → packages/mcp/)
  git.ts            ← git ops (stays → packages/cli/)
  names.ts          ← benchmark run name generator (stays → packages/cli/)
  model/
    types.ts        ← FileAnalysis, ModelBackend, SetupStatus (will → packages/shared/)
    ollama-backend.ts + spec  (will → packages/server/)
    system-check.ts + spec    (will → packages/server/)
  tools/
    reindex.ts + spec    (will → packages/mcp/)
    query.ts + spec      (will → packages/mcp/)
    drift.ts + spec      (will → packages/mcp/)
    context.ts + spec    (will → packages/mcp/)
    project-memory.ts + spec (will → packages/mcp/)
  commands/
    serve.ts + spec   (will → packages/server/, thin re-export stays in cli)
    init.ts           ← imports reindexRepo + checkSystemRequirements
    add.ts            ← imports reindexRepo
    status.ts         ← imports checkDrift
    setup.ts          ← registers MCP server path
    doctor.ts, migrate.ts, benchmark-*.ts (all stay → packages/cli/)
  __stubs__/
    bun-sqlite.ts     (will → packages/server/)
    bun-globals.ts    (will → packages/server/)
```

**Dependency graph after split:**
```
@sensei/shared   ← no deps
@sensei/server   ← @sensei/shared
@sensei/mcp      ← @sensei/shared
@sensei/cli      ← @sensei/shared + @sensei/server + @sensei/mcp
```

Root `package.json` already has `"workspaces": ["packages/*", "apps/*"]` — no change needed.

---

## Task 1: Create `packages/shared/`

**Files to create:**
- `packages/shared/package.json`
- `packages/shared/tsconfig.json`
- `packages/shared/src/types.ts`
- `packages/shared/src/constants.ts`
- `packages/shared/src/index.ts`

No tests — this package contains only types and constants.

**Step 1: Create directory structure**

```bash
mkdir -p /Users/Jerry/Developer/skills/packages/shared/src
```

**Step 2: Create `packages/shared/package.json`**

```json
{
  "name": "@sensei/shared",
  "version": "0.1.0",
  "type": "module",
  "exports": {
    ".": "./src/index.ts"
  },
  "devDependencies": {
    "typescript": "^5.5.0"
  }
}
```

**Step 3: Create `packages/shared/tsconfig.json`**

```json
{
  "extends": "../../config/tsconfig.base.json",
  "compilerOptions": {
    "outDir": "dist",
    "rootDir": "src"
  },
  "include": ["src/**/*"],
  "exclude": ["node_modules", "dist"]
}
```

**Step 4: Create `packages/shared/src/types.ts`**

Consolidates `packages/sensei/src/types.ts` AND `packages/sensei/src/model/types.ts` into one file:

```typescript
// ─── Core index types ────────────────────────────────────────────────────────

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

// ─── Model / inference types ─────────────────────────────────────────────────

export interface AnalyzedSymbol {
  name: string;
  kind: "function" | "class" | "type" | "const" | "interface" | "enum" | "method" | "hook" | "component";
  signature: string;      // L0 — concise "what"
  description: string;    // L1 — plain-English explanation
  visibility: "public" | "internal";
  tags?: string[];
}

export interface Flow {
  name: string;
  steps: string[];
}

export interface Relation {
  kind: "imports" | "calls" | "implements" | "extends" | "covers";
  target: string;
}

export interface FileAnalysis {
  path: string;
  language: string;
  contentHash: string;
  analyzedAt: string;
  symbols: AnalyzedSymbol[];
  summary: string;
  role?: string;
  flows?: Flow[];
  examples?: string[];
  relations?: Relation[];
  embedding?: number[];
}

export interface ExtractionInstructions {
  filePath: string;
  language?: string;
  techContext?: string;
  focusHints?: string[];
}

export interface ModelBackend {
  name: string;
  init(): Promise<void>;
  embed(text: string): Promise<number[]>;
  extract(content: string, instructions: ExtractionInstructions): Promise<FileAnalysis>;
  isAvailable(): Promise<boolean>;
}

export interface IndexConfig {
  backend: "ollama" | "regex";
  ollamaModel?: string;
  embeddingModel?: string;
  embeddingReady: boolean;
  indexedAt?: string;
  version: number;
}

export interface SetupStatus {
  ollamaBinary: boolean;
  ollamaRunning: boolean;
  ollamaModel: boolean;
  ollamaModelName: string;
  onnxModel: boolean;
  diskFreeGB: number;
  ramTotalGB: number;
  ramAvailableGB: number;
}
```

**Step 5: Create `packages/shared/src/constants.ts`**

Copy exactly from `packages/sensei/src/constants.ts`:

```typescript
import { join } from "path";

/** The folder where sensei writes all generated artifacts. */
export const SENSEI_DIR = ".sensei";

/** Build a path inside the sensei dir relative to repoPath. */
export function senseiPath(repoPath: string, ...parts: string[]): string {
  return join(repoPath, SENSEI_DIR, ...parts);
}
```

**Step 6: Create `packages/shared/src/index.ts`**

```typescript
export * from "./types.js";
export * from "./constants.js";
```

**Step 7: Verify TypeScript compiles**

```bash
cd /Users/Jerry/Developer/skills
bun install
cd packages/shared
bunx tsc --noEmit
```

Expected: no errors.

**Step 8: Commit**

```bash
cd /Users/Jerry/Developer/skills
git add packages/shared/
git commit -m "feat: add @sensei/shared package (types + constants)"
```

---

## Task 2: Update `packages/sensei/` to import from `@sensei/shared`

**Files to modify:** several files in `packages/sensei/src/`

This is a pure refactor — no behavior changes. After this task `packages/sensei/` still owns the original source files but imports types/constants from `@sensei/shared`.

**Step 1: Add `@sensei/shared` to sensei's package.json deps**

Edit `packages/sensei/package.json` — add to `"dependencies"`:

```json
"@sensei/shared": "workspace:*"
```

**Step 2: Install**

```bash
cd /Users/Jerry/Developer/skills
bun install
```

Expected: bun installs and symlinks `@sensei/shared` into `packages/sensei/node_modules/`.

**Step 3: Find all files that import from the files being moved**

```bash
cd /Users/Jerry/Developer/skills/packages/sensei
grep -rn "from.*['\"]\..*types['\"]" src/ --include="*.ts" | grep -v spec | grep -v "\.d\.ts"
grep -rn "from.*['\"]\..*constants['\"]" src/ --include="*.ts" | grep -v spec
grep -rn "from.*['\"]\..*model/types['\"]" src/ --include="*.ts"
```

Expected output will show files like `src/tools/reindex.ts`, `src/index-reader.ts`, `src/model/ollama-backend.ts`, `src/model/system-check.ts`, etc.

**Step 4: Update each file's import**

For every file that imports from `./types.js`, `../types.js`, `./model/types.js`, `../model/types.js`, `./constants.js`, `../constants.js` — change those imports to `@sensei/shared`.

Examples:

In `src/tools/reindex.ts`:
```typescript
// Before:
import type { SymbolMap } from "../types.js";
import { SENSEI_DIR, senseiPath } from "../constants.js";

// After:
import type { SymbolMap } from "@sensei/shared";
import { SENSEI_DIR, senseiPath } from "@sensei/shared";
```

In `src/model/ollama-backend.ts`:
```typescript
// Before:
import type { ModelBackend, FileAnalysis, ExtractionInstructions } from "./types.js";

// After:
import type { ModelBackend, FileAnalysis, ExtractionInstructions } from "@sensei/shared";
```

In `src/model/system-check.ts`:
```typescript
// Before:
import type { SetupStatus } from "./types.js";

// After:
import type { SetupStatus } from "@sensei/shared";
```

In `src/index-reader.ts`:
```typescript
// Before:
import type { LlmSpec, SymbolMap } from "./types.js";
import { SENSEI_DIR, senseiPath } from "./constants.js";

// After:
import type { LlmSpec, SymbolMap } from "@sensei/shared";
import { SENSEI_DIR, senseiPath } from "@sensei/shared";
```

In `src/commands/init.ts`:
```typescript
// Before:
import type { SetupStatus } from "../model/types.js";  // if present

// After:
import type { SetupStatus } from "@sensei/shared";
```

Do the same for all remaining files that import from `./types.js`, `../types.js`, `./constants.js`, `../constants.js` (but NOT from `./model/types.js` in model/ files — those are already updated above).

Also update spec files that import from these paths.

**Step 5: Run the full test suite**

```bash
cd /Users/Jerry/Developer/skills/packages/sensei
bunx vitest run
```

Expected: 118 tests pass. If any fail, it's an import path typo — fix it.

**Step 6: Commit**

```bash
cd /Users/Jerry/Developer/skills
git add packages/sensei/
git commit -m "refactor(sensei): import types and constants from @sensei/shared"
```

---

## Task 3: Create `packages/server/`

**Files to create:**
- `packages/server/package.json`
- `packages/server/tsconfig.json`
- `packages/server/vitest.config.ts`
- `packages/server/src/__stubs__/bun-sqlite.ts`
- `packages/server/src/__stubs__/bun-globals.ts`
- `packages/server/src/model/ollama-backend.ts` + spec
- `packages/server/src/model/system-check.ts` + spec
- `packages/server/src/serve.ts` + spec
- `packages/server/src/index.ts`

**Step 1: Create directories**

```bash
mkdir -p /Users/Jerry/Developer/skills/packages/server/src/model
mkdir -p /Users/Jerry/Developer/skills/packages/server/src/__stubs__
```

**Step 2: Create `packages/server/package.json`**

```json
{
  "name": "@sensei/server",
  "version": "0.1.0",
  "type": "module",
  "exports": {
    ".": "./src/index.ts"
  },
  "scripts": {
    "build": "bun build src/index.ts --outdir dist --target bun",
    "test": "bunx vitest run"
  },
  "dependencies": {
    "@sensei/shared": "workspace:*",
    "@clack/prompts": "^0.9.0"
  },
  "devDependencies": {
    "@types/node": "^22.0.0",
    "typescript": "^5.5.0",
    "vitest": "^2.0.0"
  }
}
```

**Step 3: Create `packages/server/tsconfig.json`**

```json
{
  "extends": "../../config/tsconfig.base.json",
  "compilerOptions": {
    "outDir": "dist",
    "rootDir": "src"
  },
  "include": ["src/**/*"],
  "exclude": ["node_modules", "dist"]
}
```

**Step 4: Create `packages/server/vitest.config.ts`**

```typescript
import { mergeConfig } from "vitest/config";
import base from "../../config/vitest.config.base.ts";
import { resolve } from "path";

const __dirname = new URL(".", import.meta.url).pathname;

export default mergeConfig(base, {
  resolve: {
    alias: {
      "bun:sqlite": resolve(__dirname, "src/__stubs__/bun-sqlite.ts"),
    },
  },
  test: {
    setupFiles: [resolve(__dirname, "src/__stubs__/bun-globals.ts")],
  },
});
```

**Step 5: Copy stub files**

Copy (do not move yet) `packages/sensei/src/__stubs__/bun-sqlite.ts` → `packages/server/src/__stubs__/bun-sqlite.ts`

Copy `packages/sensei/src/__stubs__/bun-globals.ts` → `packages/server/src/__stubs__/bun-globals.ts`

Read each stub first, then write it to the new location with identical content.

**Step 6: Copy model files**

Copy (do not move yet — sensei still needs them until Task 4):

- `packages/sensei/src/model/ollama-backend.ts` → `packages/server/src/model/ollama-backend.ts`
- `packages/sensei/src/model/ollama-backend.spec.ts` → `packages/server/src/model/ollama-backend.spec.ts`
- `packages/sensei/src/model/system-check.ts` → `packages/server/src/model/system-check.ts`
- `packages/sensei/src/model/system-check.spec.ts` → `packages/server/src/model/system-check.spec.ts`

All imports in these files already use `@sensei/shared` (from Task 2), so no changes needed.

**Step 7: Copy serve files**

Copy (do not move yet):

- `packages/sensei/src/commands/serve.ts` → `packages/server/src/serve.ts`
- `packages/sensei/src/commands/serve.spec.ts` → `packages/server/src/serve.spec.ts`

In `packages/server/src/serve.ts`, update imports:
- `../constants.js` → `@sensei/shared`  (if any remain — check after Task 2)
- `../model/system-check.js` → `./model/system-check.js`  (relative, now a sibling)
- `../model/ollama-backend.js` → `./model/ollama-backend.js`  (relative, now a sibling)

In `packages/server/src/serve.spec.ts`, update imports:
- `./serve.js` stays as-is (it's in the same directory now)

**Step 8: Create `packages/server/src/index.ts`**

This is the public API of the server package:

```typescript
// Server HTTP API
export { createReportServer, serve } from "./serve.js";
export type { ServeOptions } from "./serve.js";

// Model / inference
export { OllamaBackend, makeFallbackAnalysis, extractJson } from "./model/ollama-backend.js";
export {
  checkSystemRequirements,
  getDiskFreeGB,
  getRamGB,
  OLLAMA_BASE_URL,
  OLLAMA_MODEL,
  OLLAMA_MODEL_SIZE_GB,
  ONNX_MODEL_ID,
  ONNX_MODEL_SIZE_MB,
} from "./model/system-check.js";
```

**Step 9: Install and run server tests**

```bash
cd /Users/Jerry/Developer/skills
bun install
cd packages/server
bunx vitest run
```

Expected: 26 tests pass (11 ollama-backend + 8 system-check + 7 serve).

**Step 10: Commit**

```bash
cd /Users/Jerry/Developer/skills
git add packages/server/
git commit -m "feat: add @sensei/server package (inference engine + telemetry server)"
```

---

## Task 4: Wire `packages/sensei/` → `@sensei/server`, delete moved code

**Step 1: Add `@sensei/server` to sensei's package.json deps**

Edit `packages/sensei/package.json` — add to `"dependencies"`:

```json
"@sensei/server": "workspace:*"
```

**Step 2: Install**

```bash
cd /Users/Jerry/Developer/skills
bun install
```

**Step 3: Update `packages/sensei/src/commands/serve.ts`**

Replace the entire file with a thin re-export:

```typescript
// Thin re-export — implementation lives in @sensei/server
export { serve } from "@sensei/server";
```

**Step 4: Update `packages/sensei/src/commands/init.ts`**

Change imports from `../model/system-check.js` to `@sensei/server`:

```typescript
// Before:
import { checkSystemRequirements, OLLAMA_MODEL, OLLAMA_MODEL_SIZE_GB, OLLAMA_BASE_URL } from "../model/system-check.js";

// After:
import { checkSystemRequirements, OLLAMA_MODEL, OLLAMA_MODEL_SIZE_GB, OLLAMA_BASE_URL } from "@sensei/server";
```

Also update the `SetupStatus` type import if present:
```typescript
// Before:
import type { SetupStatus } from "../model/types.js";  // or @sensei/shared already

// After (if not already @sensei/shared):
import type { SetupStatus } from "@sensei/shared";
```

**Step 5: Update `packages/sensei/src/cli.ts`**

The `case "server":` block in cli.ts hits server HTTP endpoints via fetch — no import changes needed there.

Check if cli.ts has any direct imports from `./model/` or `./tools/reindex.js`. If so, update them:
- `./model/system-check.js` → `@sensei/server`

**Step 6: Delete `packages/sensei/src/model/`**

```bash
rm -rf /Users/Jerry/Developer/skills/packages/sensei/src/model
```

**Step 7: Run sensei tests**

```bash
cd /Users/Jerry/Developer/skills/packages/sensei
bunx vitest run
```

Expected: tests that previously used model/ now pass via @sensei/server workspace dep. The serve.spec.ts should still pass since it imports from `./serve.js` (the thin re-export) which re-exports from @sensei/server.

If serve.spec.ts fails because the Bun stubs are no longer in the sensei package, update `packages/sensei/vitest.config.ts` to remove the bun stubs (they are now only needed in packages/server). The serve.spec.ts will also need to move — actually: the serve.spec.ts in sensei should be deleted since the real spec now lives in packages/server/.

Delete `packages/sensei/src/commands/serve.spec.ts` (it was copied to server in Task 3).

Re-run:
```bash
bunx vitest run
```

Expected: remaining tests pass (benchmark-*, init would need reindexRepo — still coming in Task 6).

**Step 8: Commit**

```bash
cd /Users/Jerry/Developer/skills
git add packages/sensei/
git commit -m "refactor(sensei): wire to @sensei/server, delete model/"
```

---

## Task 5: Create `packages/mcp/`

**Files to create:**
- `packages/mcp/package.json`
- `packages/mcp/tsconfig.json`
- `packages/mcp/vitest.config.ts`
- `packages/mcp/src/index.ts` ← the MCP server entry (copy of sensei's index.ts, updated imports)
- `packages/mcp/src/index-reader.ts` + spec
- `packages/mcp/src/tools/` (5 tools + 5 specs)
- `packages/mcp/src/tools.ts` ← barrel re-export of all tool functions (for @sensei/cli to import)

**Step 1: Create directories**

```bash
mkdir -p /Users/Jerry/Developer/skills/packages/mcp/src/tools
```

**Step 2: Create `packages/mcp/package.json`**

```json
{
  "name": "@sensei/mcp",
  "version": "0.1.0",
  "type": "module",
  "exports": {
    ".": "./src/tools.ts",
    "./server": "./src/index.ts"
  },
  "scripts": {
    "build": "bun build src/index.ts src/tools.ts --outdir dist --target bun",
    "test": "bunx vitest run"
  },
  "dependencies": {
    "@sensei/shared": "workspace:*",
    "@modelcontextprotocol/sdk": "^1.0.0",
    "fast-glob": "^3.3.0",
    "js-yaml": "^4.1.0",
    "zod": "^3.22.0"
  },
  "devDependencies": {
    "@types/node": "^22.0.0",
    "typescript": "^5.5.0",
    "vitest": "^2.0.0"
  }
}
```

**Step 3: Create `packages/mcp/tsconfig.json`**

```json
{
  "extends": "../../config/tsconfig.base.json",
  "compilerOptions": {
    "outDir": "dist",
    "rootDir": "src"
  },
  "include": ["src/**/*"],
  "exclude": ["node_modules", "dist"]
}
```

**Step 4: Create `packages/mcp/vitest.config.ts`**

```typescript
import { mergeConfig } from "vitest/config";
import base from "../../config/vitest.config.base.ts";

export default mergeConfig(base, {});
```

**Step 5: Copy `index-reader.ts` and its spec**

Copy (do not move yet):
- `packages/sensei/src/index-reader.ts` → `packages/mcp/src/index-reader.ts`
- `packages/sensei/src/index-reader.spec.ts` → `packages/mcp/src/index-reader.spec.ts`

Imports in the copied `index-reader.ts` already use `@sensei/shared` (from Task 2) — no changes needed.

**Step 6: Copy all tools**

Copy these files from `packages/sensei/src/tools/` to `packages/mcp/src/tools/`:

- `context.ts` + `context.spec.ts`
- `drift.ts` + `drift.spec.ts`
- `project-memory.ts` + `project-memory.spec.ts`
- `query.ts` + `query.spec.ts`
- `reindex.ts` + `reindex.spec.ts`

In each copied `.ts` file, update any remaining relative imports:
- `../types.js` or `../constants.js` → `@sensei/shared`  (should already be done from Task 2)
- `../index-reader.js` → `./index-reader.js` is WRONG — it's now a sibling of tools/, so: `../index-reader.js` stays as `../index-reader.js` (correct relative path from tools/ to src/)

Double-check: `packages/mcp/src/tools/query.ts` imports `../index-reader.js` — this resolves to `packages/mcp/src/index-reader.ts`. ✓

**Step 7: Copy `src/index.ts` (MCP server entry)**

Copy `packages/sensei/src/index.ts` → `packages/mcp/src/index.ts`

Update imports in the copied file:
- `./tools/query.js`, `./tools/reindex.js`, etc. → stay as-is (relative, correct)
- `./constants.js` → `@sensei/shared`
- Any `./index-reader.js` refs → `./index-reader.js` (stays correct)

**Step 8: Create `packages/mcp/src/tools.ts`**

This is the library surface that `@sensei/cli` imports from:

```typescript
// Query tools
export { getLlmSpec, getFileContext, listExports, findPattern, getShortcuts } from "./tools/query.js";

// Reindex
export { reindexRepo } from "./tools/reindex.js";
export type { IndexSummary } from "./tools/reindex.js";

// Context
export { loadContext, recommendNext } from "./tools/context.js";

// Drift
export { checkDrift } from "./tools/drift.js";

// Project memory
export {
  checkpoint,
  getSessionContext,
  addDecision,
  addPattern,
  askQuestion,
  getOpenItems,
  closeItem,
} from "./tools/project-memory.js";
```

**Step 9: Install and run mcp tests**

```bash
cd /Users/Jerry/Developer/skills
bun install
cd packages/mcp
bunx vitest run
```

Expected: all tool specs pass (reindex ~26, query ~9, context ~7, project-memory ~12, drift ~4, index-reader ~5 = ~63 tests).

**Step 10: Commit**

```bash
cd /Users/Jerry/Developer/skills
git add packages/mcp/
git commit -m "feat: add @sensei/mcp package (MCP server + tool implementations)"
```

---

## Task 6: Wire `packages/sensei/` → `@sensei/mcp`, delete moved code

**Step 1: Add `@sensei/mcp` to sensei's package.json deps**

Edit `packages/sensei/package.json` — add to `"dependencies"`:

```json
"@sensei/mcp": "workspace:*"
```

**Step 2: Install**

```bash
cd /Users/Jerry/Developer/skills
bun install
```

**Step 3: Update `packages/sensei/src/commands/init.ts`**

```typescript
// Before:
import { reindexRepo } from "../tools/reindex.js";

// After:
import { reindexRepo } from "@sensei/mcp";
```

**Step 4: Update `packages/sensei/src/commands/add.ts`**

```typescript
// Before:
import { reindexRepo } from "../tools/reindex.js";

// After:
import { reindexRepo } from "@sensei/mcp";
```

**Step 5: Update `packages/sensei/src/commands/status.ts`**

```typescript
// Before:
import { checkDrift } from "../tools/drift.js";

// After:
import { checkDrift } from "@sensei/mcp";
```

**Step 6: Update `packages/sensei/src/cli.ts`**

```typescript
// Before (in case "index"):
const { reindexRepo } = await import("./tools/reindex.js");

// After:
const { reindexRepo } = await import("@sensei/mcp");

// Before (in case "drift"):
const { checkDrift } = await import("./tools/drift.js");

// After:
const { checkDrift } = await import("@sensei/mcp");
```

**Step 7: Update `packages/sensei/src/commands/setup.ts`**

Currently `setup.ts` derives the MCP server path by replacing `cli.js` with `index.js` in the same dist folder. After the split, the MCP server lives in `packages/mcp/dist/index.js`, not alongside the CLI.

Read `packages/sensei/src/commands/setup.ts` first, then update the path resolution:

```typescript
// Before (approximate — read the actual file):
const cliPath = new URL(import.meta.url).pathname;
const indexJsPath = cliPath.replace(/cli\.js$/, "index.js");
await setupMcp(repoRoot, indexJsPath);

// After:
import { createRequire } from "module";
import { dirname, join } from "path";
const _require = createRequire(import.meta.url);
// Resolve the @sensei/mcp package location via workspace symlink
const mcpPkgPath = _require.resolve("@sensei/mcp/package.json");
const mcpIndexJs = join(dirname(mcpPkgPath), "dist", "index.js");
await setupMcp(repoRoot, mcpIndexJs);
```

Note: `setup --mcp` registers the **built** MCP server (`dist/index.js`). Users must run `bun run build` in `packages/mcp/` before running `sensei setup --mcp`.

**Step 8: Delete moved files from sensei**

```bash
rm -rf /Users/Jerry/Developer/skills/packages/sensei/src/tools
rm -f /Users/Jerry/Developer/skills/packages/sensei/src/index.ts
rm -f /Users/Jerry/Developer/skills/packages/sensei/src/index-reader.ts
rm -f /Users/Jerry/Developer/skills/packages/sensei/src/index-reader.spec.ts
```

**Step 9: Update `packages/sensei/vitest.config.ts`**

Remove the bun stubs since serve.ts and its spec no longer live in sensei:

```typescript
import { mergeConfig } from "vitest/config";
import base from "../../config/vitest.config.base.ts";

export default mergeConfig(base, {});
```

Also delete the stubs from sensei (they live in server now):

```bash
rm -rf /Users/Jerry/Developer/skills/packages/sensei/src/__stubs__
```

**Step 10: Run sensei tests**

```bash
cd /Users/Jerry/Developer/skills/packages/sensei
bunx vitest run
```

Expected: benchmark-*, names, git specs pass. Any remaining failures are missing import updates — fix them.

**Step 11: Commit**

```bash
cd /Users/Jerry/Developer/skills
git add packages/sensei/
git commit -m "refactor(sensei): wire to @sensei/mcp, delete tools/ and index.ts"
```

---

## Task 7: Rename `packages/sensei/` → `packages/cli/`

**Step 1: Rename with git**

```bash
cd /Users/Jerry/Developer/skills
git mv packages/sensei packages/cli
```

**Step 2: Update `packages/cli/package.json`**

Change:
- `"name": "sensei"` → `"name": "@sensei/cli"`
- Keep `"bin": { "sensei": "./dist/cli.js" }` unchanged — the binary name stays `sensei`
- Keep all existing deps
- Update tsconfig extends path (it's now `../../config/tsconfig.base.json` — verify it's still correct after the rename, it should be since depth is the same)

**Step 3: Update `packages/cli/tsconfig.json`**

Verify extends path is still `"../../config/tsconfig.base.json"` — it should be, since the depth (`packages/cli/`) is the same as before (`packages/sensei/`).

**Step 4: Run bun install**

```bash
cd /Users/Jerry/Developer/skills
bun install
```

This recreates symlinks and updates the workspace registry.

**Step 5: Build all packages**

```bash
cd /Users/Jerry/Developer/skills

# Build in dependency order
cd packages/shared && bunx tsc --noEmit && cd ../..
cd packages/server && bun run build && cd ../..
cd packages/mcp && bun run build && cd ../..
cd packages/cli && bun run build && cd ../..
```

Expected: all build without errors. If mcp build fails because `@sensei/shared` exports TypeScript source (no dist/), that's fine — Bun's bundler resolves workspace source directly.

**Step 6: Smoke test the CLI**

```bash
/Users/Jerry/Developer/skills/packages/cli/dist/cli.js --help
/Users/Jerry/Developer/skills/packages/cli/dist/cli.js server status
```

Expected: help text prints, server status shows "not running" or running status.

**Step 7: Update README.md**

In `README.md`, update the Getting Started section:

```bash
# Install sensei globally (from packages/cli/)
cd packages/cli
bun install && bun run build
bun link   # or: bun i -g .
```

Also update the Repo Structure section to show the new layout:

```
packages/
  cli/    @sensei/cli    — CLI binary (sensei command)
  server/ @sensei/server — inference engine + telemetry server
  mcp/    @sensei/mcp    — MCP tool server (served to Claude)
  shared/ @sensei/shared — types, constants, API contracts
```

**Step 8: Commit**

```bash
cd /Users/Jerry/Developer/skills
git add packages/cli/ README.md
git commit -m "refactor: rename packages/sensei → packages/cli, package name → @sensei/cli"
```

---

## Task 8: Full Validation

**Step 1: Run tests in all packages**

```bash
cd /Users/Jerry/Developer/skills

cd packages/shared && bunx tsc --noEmit
echo "shared: ✓"

cd ../server && bunx vitest run
echo "server tests: ✓"

cd ../mcp && bunx vitest run
echo "mcp tests: ✓"

cd ../cli && bunx vitest run
echo "cli tests: ✓"
```

Expected: all packages type-check and test-suites pass.

**Step 2: Build all packages in dependency order**

```bash
cd /Users/Jerry/Developer/skills/packages/server && bun run build
cd /Users/Jerry/Developer/skills/packages/mcp && bun run build
cd /Users/Jerry/Developer/skills/packages/cli && bun run build
```

Expected: `dist/` created in server, mcp, and cli.

**Step 3: End-to-end smoke test**

```bash
# CLI help
/Users/Jerry/Developer/skills/packages/cli/dist/cli.js --help

# Server status (server not running = expected output)
/Users/Jerry/Developer/skills/packages/cli/dist/cli.js server status

# MCP server starts and connects (ctrl+C to exit)
timeout 3 bun run /Users/Jerry/Developer/skills/packages/mcp/dist/index.js || true
echo "MCP server started and exited cleanly"
```

**Step 4: Commit any fixes**

If any test fixes or import corrections were needed:

```bash
cd /Users/Jerry/Developer/skills
git add -p
git commit -m "fix: final package split corrections"
```

**Step 5: Update `docs/design/README.md`**

Mark `14-server-package.md` phases 0-3 as complete in the document (optional, only if there's a status field — skip if not).

---

## Validation Checklist

After all 8 tasks, verify:

```
packages/
  shared/   ✓ @sensei/shared — src/types.ts, src/constants.ts, src/index.ts
  server/   ✓ @sensei/server — src/model/, src/serve.ts, src/index.ts
  mcp/      ✓ @sensei/mcp   — src/tools/, src/index-reader.ts, src/index.ts, src/tools.ts
  cli/      ✓ @sensei/cli   — src/cli.ts, src/commands/, src/git.ts, src/names.ts
```

Cross-package dep graph:
- `@sensei/shared` has no workspace deps
- `@sensei/server` depends on `@sensei/shared`
- `@sensei/mcp` depends on `@sensei/shared`
- `@sensei/cli` depends on `@sensei/shared` + `@sensei/server` + `@sensei/mcp`

No circular dependencies.
