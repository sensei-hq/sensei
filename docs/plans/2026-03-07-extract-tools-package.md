# Extract @sensei/tools Package Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Extract the tool implementations from `packages/mcp/` into a new `packages/tools/` package so that both `@sensei/mcp` (MCP adapter) and `@sensei/cli` (CLI adapter) are thin presentation layers over shared core logic.

**Architecture:** Create `packages/tools/` containing all tool logic (`reindex`, `query`, `drift`, `context`, `project-memory`, `index-reader`). `@sensei/mcp` becomes a thin MCP protocol adapter that imports from `@sensei/tools`. `@sensei/cli` drops its dependency on `@sensei/mcp` and imports from `@sensei/tools` directly. Final dep graph: `shared` ← `tools` ← `mcp` / `cli` (neither mcp nor cli depends on the other).

**Tech Stack:** Bun workspaces, `workspace:*` protocol, TypeScript `moduleResolution: bundler`, Vitest.

---

## Background: Current Structure

```
packages/
  shared/   @sensei/shared  — types + constants (no deps)
  server/   @sensei/server  — inference engine (← shared)
  mcp/      @sensei/mcp     — MCP server + tool impls (← shared)
  cli/      @sensei/cli     — CLI binary (← shared, server, mcp)
```

Problem: `packages/mcp/src/tools/` contains pure logic that both the MCP server and the CLI need. The CLI imports `reindexRepo`, `checkDrift`, `checkpoint`, `addDecision`, `addPattern` directly from `@sensei/mcp` — using the MCP package as a tools library rather than an MCP adapter.

**Target structure:**

```
packages/
  shared/   @sensei/shared  — types + constants (no deps)
  tools/    @sensei/tools   — pure tool logic (← shared)
  server/   @sensei/server  — inference engine (← shared)
  mcp/      @sensei/mcp     — thin MCP adapter (← shared, tools)
  cli/      @sensei/cli     — thin CLI adapter (← shared, server, tools)
```

**Files to move from `packages/mcp/src/` to `packages/tools/src/`:**
- `index-reader.ts` + `index-reader.spec.ts`
- `tools/context.ts` + `context.spec.ts`
- `tools/drift.ts` + `drift.spec.ts`
- `tools/project-memory.ts` + `project-memory.spec.ts`
- `tools/query.ts` + `query.spec.ts`
- `tools/reindex.ts` + `reindex.spec.ts`

**`packages/mcp/src/tools.ts` barrel is deleted** — it was the band-aid that let CLI import from MCP. After the split, `@sensei/tools` is the library surface.

**`packages/mcp/src/index.ts`** becomes a thin adapter: imports from `@sensei/tools` instead of local `./tools/`.

---

## Task 1: Create `packages/tools/`

**Files to create:**
- `packages/tools/package.json`
- `packages/tools/tsconfig.json`
- `packages/tools/vitest.config.ts`
- `packages/tools/src/index-reader.ts` (copy from mcp)
- `packages/tools/src/index-reader.spec.ts` (copy from mcp)
- `packages/tools/src/tools/context.ts` + `context.spec.ts` (copy from mcp)
- `packages/tools/src/tools/drift.ts` + `drift.spec.ts` (copy from mcp)
- `packages/tools/src/tools/project-memory.ts` + `project-memory.spec.ts` (copy from mcp)
- `packages/tools/src/tools/query.ts` + `query.spec.ts` (copy from mcp)
- `packages/tools/src/tools/reindex.ts` + `reindex.spec.ts` (copy from mcp)
- `packages/tools/src/index.ts` (barrel)

**Step 1: Create directories**

```bash
mkdir -p /Users/Jerry/Developer/skills/packages/tools/src/tools
```

**Step 2: Create `packages/tools/package.json`**

```json
{
  "name": "@sensei/tools",
  "version": "0.1.0",
  "type": "module",
  "exports": {
    ".": "./src/index.ts",
    "./package.json": "./package.json"
  },
  "scripts": {
    "build": "bun build src/index.ts --outdir dist --target bun",
    "test": "bunx vitest run"
  },
  "dependencies": {
    "@sensei/shared": "workspace:*",
    "fast-glob": "^3.3.0",
    "js-yaml": "^4.1.0",
    "zod": "^3.22.0"
  },
  "devDependencies": {
    "@types/js-yaml": "^4.0.9",
    "@types/node": "^22.0.0",
    "typescript": "^5.5.0",
    "vitest": "^2.0.0"
  }
}
```

Note: `zod` is included because `reindex.ts` uses it for schema validation. Verify by checking the actual `reindex.ts` — if zod is not imported there, remove it.

**Step 3: Create `packages/tools/tsconfig.json`**

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

**Step 4: Create `packages/tools/vitest.config.ts`**

```typescript
import { mergeConfig } from "vitest/config";
import base from "../../config/vitest.config.base.ts";

export default mergeConfig(base, {});
```

**Step 5: Copy source files from `packages/mcp/src/` (do NOT delete originals yet)**

```bash
cp /Users/Jerry/Developer/skills/packages/mcp/src/index-reader.ts \
   /Users/Jerry/Developer/skills/packages/tools/src/index-reader.ts

cp /Users/Jerry/Developer/skills/packages/mcp/src/index-reader.spec.ts \
   /Users/Jerry/Developer/skills/packages/tools/src/index-reader.spec.ts

cp /Users/Jerry/Developer/skills/packages/mcp/src/tools/context.ts \
   /Users/Jerry/Developer/skills/packages/tools/src/tools/context.ts

cp /Users/Jerry/Developer/skills/packages/mcp/src/tools/context.spec.ts \
   /Users/Jerry/Developer/skills/packages/tools/src/tools/context.spec.ts

cp /Users/Jerry/Developer/skills/packages/mcp/src/tools/drift.ts \
   /Users/Jerry/Developer/skills/packages/tools/src/tools/drift.ts

cp /Users/Jerry/Developer/skills/packages/mcp/src/tools/drift.spec.ts \
   /Users/Jerry/Developer/skills/packages/tools/src/tools/drift.spec.ts

cp /Users/Jerry/Developer/skills/packages/mcp/src/tools/project-memory.ts \
   /Users/Jerry/Developer/skills/packages/tools/src/tools/project-memory.ts

cp /Users/Jerry/Developer/skills/packages/mcp/src/tools/project-memory.spec.ts \
   /Users/Jerry/Developer/skills/packages/tools/src/tools/project-memory.spec.ts

cp /Users/Jerry/Developer/skills/packages/mcp/src/tools/query.ts \
   /Users/Jerry/Developer/skills/packages/tools/src/tools/query.ts

cp /Users/Jerry/Developer/skills/packages/mcp/src/tools/query.spec.ts \
   /Users/Jerry/Developer/skills/packages/tools/src/tools/query.spec.ts

cp /Users/Jerry/Developer/skills/packages/mcp/src/tools/reindex.ts \
   /Users/Jerry/Developer/skills/packages/tools/src/tools/reindex.ts

cp /Users/Jerry/Developer/skills/packages/mcp/src/tools/reindex.spec.ts \
   /Users/Jerry/Developer/skills/packages/tools/src/tools/reindex.spec.ts
```

**Step 6: Verify/fix imports in copied files**

The files already import from `@sensei/shared` for types/constants. The only internal relative import to check is in `query.ts` and `context.ts`:

```typescript
// In tools/query.ts and tools/context.ts:
import { ... } from "../index-reader.js";
// This is correct — tools/ is one level below src/, index-reader.ts is at src/ level
```

No changes should be needed. Verify by reading each copied `.ts` file and checking all import paths resolve correctly within `packages/tools/src/`.

**Step 7: Create `packages/tools/src/index.ts` barrel**

```typescript
// Index reader
export { readLlmSpec, readSymbolMap, readIndexFile } from "./index-reader.js";

// Query tools
export { getLlmSpec, getFileContext, listExports, findPattern, getShortcuts } from "./tools/query.js";

// Reindex
export { reindexRepo } from "./tools/reindex.js";
export type { IndexSummary } from "./tools/reindex.js";

// Context
export { loadContext, recommendNext } from "./tools/context.js";
export type { ContextSlice } from "./tools/context.js";

// Drift
export { checkDrift } from "./tools/drift.js";
export type { DriftEntry, DriftResult } from "./tools/drift.js";

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

Before writing this barrel, read each source file to verify the exported names are correct.

**Step 8: Install and run tests**

```bash
cd /Users/Jerry/Developer/skills
bun install
cd packages/tools
bunx vitest run
```

Expected: 63 tests pass (same tests as currently in `packages/mcp/`).

If any test fails due to a missing import, fix the import in `packages/tools/src/` — do NOT modify `packages/mcp/`.

**Step 9: Commit**

```bash
cd /Users/Jerry/Developer/skills
git add packages/tools/
git commit -m "feat: add @sensei/tools package (extracted from @sensei/mcp)"
```

---

## Task 2: Wire `packages/mcp/` → `@sensei/tools`, delete moved code

**Files to modify:**
- `packages/mcp/package.json` — add `@sensei/tools: workspace:*`, remove `fast-glob`, `js-yaml`, `zod`
- `packages/mcp/src/index.ts` — update imports from local `./tools/` to `@sensei/tools`
- Delete: `packages/mcp/src/tools/` (entire directory)
- Delete: `packages/mcp/src/index-reader.ts`
- Delete: `packages/mcp/src/index-reader.spec.ts`
- Delete: `packages/mcp/src/tools.ts` (the library barrel — no longer needed, CLI will use @sensei/tools directly)

**Step 1: Add `@sensei/tools` to mcp's package.json, remove tool deps**

Read `packages/mcp/package.json`, then:
- Add `"@sensei/tools": "workspace:*"` to `dependencies`
- Remove `"fast-glob"`, `"js-yaml"`, `"zod"` (these move to @sensei/tools)
- Remove `"@types/js-yaml"` from devDependencies (no longer needed)
- Keep `"@modelcontextprotocol/sdk"` (still needed for the MCP server)
- Keep `"@sensei/shared"` (still used in index.ts for `SENSEI_DIR`)
- Update `exports` — remove `"."` entry (tools.ts is being deleted), keep `"./server"` and `"./package.json"`:

```json
{
  "name": "@sensei/mcp",
  "version": "0.1.0",
  "type": "module",
  "exports": {
    "./server": "./src/index.ts",
    "./package.json": "./package.json"
  },
  "scripts": {
    "build": "bun build src/index.ts --outdir dist --target bun",
    "test": "bunx vitest run"
  },
  "dependencies": {
    "@sensei/shared": "workspace:*",
    "@sensei/tools": "workspace:*",
    "@modelcontextprotocol/sdk": "^1.0.0"
  },
  "devDependencies": {
    "@types/node": "^22.0.0",
    "typescript": "^5.5.0",
    "vitest": "^2.0.0"
  }
}
```

**Step 2: Install**

```bash
cd /Users/Jerry/Developer/skills
bun install
```

**Step 3: Update `packages/mcp/src/index.ts` imports**

Read the file. It currently imports from local `./tools/query.js`, `./tools/reindex.js`, etc. Change all tool imports to `@sensei/tools`:

```typescript
// Before:
import { getLlmSpec, getFileContext, listExports, findPattern, getShortcuts } from "./tools/query.js";
import { reindexRepo } from "./tools/reindex.js";
import { loadContext, recommendNext } from "./tools/context.js";
import { checkDrift } from "./tools/drift.js";
import { checkpoint, getSessionContext, addDecision, addPattern, askQuestion, getOpenItems, closeItem } from "./tools/project-memory.js";
import { SENSEI_DIR } from "@sensei/shared";

// After:
import { getLlmSpec, getFileContext, listExports, findPattern, getShortcuts,
         reindexRepo, loadContext, recommendNext, checkDrift,
         checkpoint, getSessionContext, addDecision, addPattern,
         askQuestion, getOpenItems, closeItem } from "@sensei/tools";
import { SENSEI_DIR } from "@sensei/shared";
```

**Step 4: Delete moved files from mcp**

```bash
rm -rf /Users/Jerry/Developer/skills/packages/mcp/src/tools
rm -f /Users/Jerry/Developer/skills/packages/mcp/src/index-reader.ts
rm -f /Users/Jerry/Developer/skills/packages/mcp/src/index-reader.spec.ts
rm -f /Users/Jerry/Developer/skills/packages/mcp/src/tools.ts
```

**Step 5: Run mcp tests**

```bash
cd /Users/Jerry/Developer/skills/packages/mcp
bunx vitest run
```

Expected: 0 test files (all tests moved to packages/tools). If vitest reports "no test files found", that is correct. If it errors on missing imports in index.ts, fix those imports first.

**Step 6: Verify mcp build**

```bash
cd /Users/Jerry/Developer/skills/packages/mcp
bun run build
```

Expected: `dist/index.js` builds successfully (no tools test bundle needed).

**Step 7: Commit**

```bash
cd /Users/Jerry/Developer/skills
git add packages/mcp/
git commit -m "refactor(mcp): wire to @sensei/tools, delete tool impls"
```

---

## Task 3: Wire `packages/cli/` → `@sensei/tools`, drop `@sensei/mcp` dep

**Files to modify:**
- `packages/cli/package.json` — add `@sensei/tools: workspace:*`, remove `@sensei/mcp`
- `packages/cli/src/commands/add.ts` — update import
- `packages/cli/src/commands/init.ts` — update import
- `packages/cli/src/commands/status.ts` — update import
- `packages/cli/src/commands/migrate.ts` — update import
- `packages/cli/src/cli.ts` — update dynamic imports

**Step 1: Update `packages/cli/package.json`**

Read the file, then:
- Replace `"@sensei/mcp": "workspace:*"` with `"@sensei/tools": "workspace:*"`
- All other deps stay the same (`@sensei/server`, `@sensei/shared`, `@anthropic-ai/sdk`, `@clack/prompts`)

**Step 2: Install**

```bash
cd /Users/Jerry/Developer/skills
bun install
```

**Step 3: Update command imports**

`packages/cli/src/commands/add.ts`:
```typescript
// Before:
import { reindexRepo } from "@sensei/mcp";
// After:
import { reindexRepo } from "@sensei/tools";
```

`packages/cli/src/commands/init.ts`:
```typescript
// Before:
import { reindexRepo } from "@sensei/mcp";
// After:
import { reindexRepo } from "@sensei/tools";
```

`packages/cli/src/commands/status.ts`:
```typescript
// Before:
import { checkDrift } from "@sensei/mcp";
// After:
import { checkDrift } from "@sensei/tools";
```

`packages/cli/src/commands/migrate.ts`:
```typescript
// Before:
import { addDecision, addPattern, checkpoint } from "@sensei/mcp";
// After:
import { addDecision, addPattern, checkpoint } from "@sensei/tools";
```

**Step 4: Update dynamic imports in `packages/cli/src/cli.ts`**

Read the file. Find the dynamic imports:

```typescript
// Before (case "index"):
const { reindexRepo } = await import("@sensei/mcp");

// After:
const { reindexRepo } = await import("@sensei/tools");

// Before (case "drift"):
const { checkDrift } = await import("@sensei/mcp");

// After:
const { checkDrift } = await import("@sensei/tools");
```

Also check if `cli.ts` has any other references to `@sensei/mcp`. The `setup` case uses `_require.resolve("@sensei/mcp/package.json")` to locate the MCP server binary — this is intentional and must NOT be changed (the CLI still needs to know where the MCP server dist lives for `sensei setup --mcp`).

**Step 5: Verify no stale @sensei/mcp imports**

```bash
grep -rn "@sensei/mcp" /Users/Jerry/Developer/skills/packages/cli/src/ --include="*.ts"
```

Expected output: only the `_require.resolve("@sensei/mcp/package.json")` line in `cli.ts`. No import statements should reference `@sensei/mcp`.

**Step 6: Run cli tests**

```bash
cd /Users/Jerry/Developer/skills/packages/cli
bunx vitest run
```

Expected: 29 tests pass.

**Step 7: Commit**

```bash
cd /Users/Jerry/Developer/skills
git add packages/cli/
git commit -m "refactor(cli): import tools from @sensei/tools, drop @sensei/mcp dep"
```

---

## Task 4: Full validation

**Step 1: Run all tests**

```bash
cd /Users/Jerry/Developer/skills/packages/tools && bunx vitest run
echo "tools: ✓"

cd /Users/Jerry/Developer/skills/packages/mcp && bunx vitest run || echo "mcp: no tests (expected)"

cd /Users/Jerry/Developer/skills/packages/server && bunx vitest run
echo "server: ✓"

cd /Users/Jerry/Developer/skills/packages/cli && bunx vitest run
echo "cli: ✓"
```

Expected totals: tools=63, mcp=0, server=26, cli=29.

**Step 2: Build all packages**

```bash
cd /Users/Jerry/Developer/skills/packages/tools && bun run build
cd /Users/Jerry/Developer/skills/packages/mcp && bun run build
cd /Users/Jerry/Developer/skills/packages/server && bun run build
cd /Users/Jerry/Developer/skills/packages/cli && bun run build
```

Expected: all build without errors.

**Step 3: Smoke test**

```bash
/Users/Jerry/Developer/skills/packages/cli/dist/cli.js --help
```

Expected: help text prints.

**Step 4: Verify final dependency graph**

```bash
# shared has no workspace deps
grep "workspace:" /Users/Jerry/Developer/skills/packages/shared/package.json || echo "shared: no workspace deps ✓"

# tools depends only on shared
grep "workspace:" /Users/Jerry/Developer/skills/packages/tools/package.json

# server depends only on shared
grep "workspace:" /Users/Jerry/Developer/skills/packages/server/package.json

# mcp depends on shared + tools (NOT cli)
grep "workspace:" /Users/Jerry/Developer/skills/packages/mcp/package.json

# cli depends on shared + server + tools (NOT mcp)
grep "workspace:" /Users/Jerry/Developer/skills/packages/cli/package.json
```

Expected final dep graph:
```
@sensei/shared   ← no workspace deps
@sensei/tools    ← @sensei/shared
@sensei/server   ← @sensei/shared
@sensei/mcp      ← @sensei/shared, @sensei/tools
@sensei/cli      ← @sensei/shared, @sensei/server, @sensei/tools
```

**Step 5: Commit**

```bash
cd /Users/Jerry/Developer/skills
git add -A
git commit -m "chore: full validation — @sensei/tools package split complete"
```
