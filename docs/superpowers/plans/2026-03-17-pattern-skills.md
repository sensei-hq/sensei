# Pattern Skills Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add two skills (`identifying-patterns` and `pattern-based-development`) with a Supabase-backed usage tracking table and a new `record_pattern_use` MCP tool.

**Architecture:** New `sensei.pattern_usages` table tracks which patterns were applied per session. A new `record-pattern-use.ts` tool inserts rows. `checkpoint.ts` is extended to update those rows with outcome, FTR score, and changed files when the session ends. Two skill files guide agents through pattern discovery and pattern-based implementation.

**Tech Stack:** TypeScript, Supabase (PostgreSQL), Zod, MCP SDK, Vitest

---

## Chunk 1: Server-Side Changes (DB + MCP tools)

### Task 1: Supabase migration — `pattern_usages` table

**Files:**
- Create: `supabase/migrations/20260317000000_pattern_usages.sql`

- [ ] **Step 1: Write the migration file**

```sql
-- Pattern usage tracking — one row per pattern applied per session
CREATE TABLE IF NOT EXISTS sensei.pattern_usages (
  id             UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
  repo_id        UUID        REFERENCES sensei.repos(id) ON DELETE CASCADE,
  session_id     TEXT,
  pattern_name   TEXT        NOT NULL,
  applied_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
  outcome        TEXT,        -- completed | blocked | partial (filled by checkpoint())
  files_modified TEXT[],      -- filled by checkpoint() via git diff
  ftr_score      FLOAT        -- filled by checkpoint()
);

CREATE INDEX IF NOT EXISTS idx_pattern_usages_repo_pattern
  ON sensei.pattern_usages(repo_id, pattern_name);

CREATE INDEX IF NOT EXISTS idx_pattern_usages_session
  ON sensei.pattern_usages(session_id)
  WHERE session_id IS NOT NULL;
```

- [ ] **Step 2: Verify the file exists and SQL is syntactically correct**

```bash
cat supabase/migrations/20260317000000_pattern_usages.sql
```
Expected: file prints without error.

- [ ] **Step 3: Commit**

```bash
git add supabase/migrations/20260317000000_pattern_usages.sql
git commit -m "feat(db): add pattern_usages table for pattern tracking"
```

---

### Task 2: `record-pattern-use.ts` tool with tests

**Files:**
- Create: `packages/server/src/tools/record-pattern-use.ts`
- Create: `packages/server/src/tools/record-pattern-use.spec.ts`

- [ ] **Step 1: Write the failing test**

```typescript
// packages/server/src/tools/record-pattern-use.spec.ts
import { describe, it, expect, vi } from "vitest";
import { recordPatternUse } from "./record-pattern-use.js";

describe("recordPatternUse", () => {
  it("inserts a row and returns confirmation string", async () => {
    const inserted: any[] = [];
    const mockClient = {
      schema: () => ({
        from: (table: string) => {
          expect(table).toBe("pattern_usages");
          return {
            insert: (row: any) => {
              inserted.push(row);
              return { then: (r: any) => r({ error: null }) };
            },
          };
        },
      }),
    };

    const result = await recordPatternUse(mockClient as any, "repo-1", "session-1", "mcp-tool");
    expect(result).toContain("mcp-tool");
    expect(inserted).toHaveLength(1);
    expect(inserted[0]).toMatchObject({
      repo_id: "repo-1",
      session_id: "session-1",
      pattern_name: "mcp-tool",
    });
  });

  it("works without a session_id", async () => {
    const inserted: any[] = [];
    const mockClient = {
      schema: () => ({
        from: () => ({
          insert: (row: any) => {
            inserted.push(row);
            return { then: (r: any) => r({ error: null }) };
          },
        }),
      }),
    };

    await recordPatternUse(mockClient as any, "repo-1", null, "adapter");
    expect(inserted[0].session_id).toBeNull();
  });

  it("throws when insert fails", async () => {
    const mockClient = {
      schema: () => ({
        from: () => ({
          insert: () => ({ then: (r: any) => r({ error: { message: "db down" } }) }),
        }),
      }),
    };

    await expect(recordPatternUse(mockClient as any, "repo-1", null, "adapter"))
      .rejects.toThrow("db down");
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cd packages/server && bunx vitest run src/tools/record-pattern-use.spec.ts
```
Expected: FAIL — `Cannot find module './record-pattern-use.js'`

- [ ] **Step 3: Write the implementation**

```typescript
// packages/server/src/tools/record-pattern-use.ts
import type { SupabaseClient } from "@supabase/supabase-js";

export async function recordPatternUse(
  client: SupabaseClient,
  repoId: string,
  sessionId: string | null,
  patternName: string,
): Promise<string> {
  const { error } = await (client as any)
    .schema("sensei")
    .from("pattern_usages")
    .insert({ repo_id: repoId, session_id: sessionId, pattern_name: patternName });

  if (error) throw new Error(error.message ?? "Failed to record pattern use");
  return `Pattern use recorded: ${patternName}`;
}
```

- [ ] **Step 4: Run test to verify it passes**

```bash
cd packages/server && bunx vitest run src/tools/record-pattern-use.spec.ts
```
Expected: 3 tests PASS

- [ ] **Step 5: Commit**

```bash
git add packages/server/src/tools/record-pattern-use.ts packages/server/src/tools/record-pattern-use.spec.ts
git commit -m "feat(server): add record-pattern-use tool"
```

---

### Task 3: Register `record_pattern_use` in `mcp-server.ts`

**Files:**
- Modify: `packages/server/src/mcp-server.ts`
- Modify: `packages/server/src/mcp-server.spec.ts`

- [ ] **Step 1: Write the failing test**

Open `packages/server/src/mcp-server.spec.ts`. Add this `it()` block inside the existing `describe("createSenseiMcpServer", ...)` block, after the existing test:

```typescript
  it("exposes record_pattern_use as a registered tool", () => {
    const server = createSenseiMcpServer({ repoId: "test", repoPath: "/tmp" });
    // MCP server stores tools in _registeredTools
    const tools = (server as any)._registeredTools;
    expect(tools).toBeDefined();
    expect(tools["record_pattern_use"]).toBeDefined();
  });
```

Do NOT replace the existing file — only add this new `it()` case.

- [ ] **Step 2: Run test to verify it fails**

```bash
cd packages/server && bunx vitest run src/mcp-server.spec.ts
```
Expected: second test FAIL — `record_pattern_use` is undefined

- [ ] **Step 3: Add import and tool registration to `mcp-server.ts`**

Add import at top (with other tool imports):
```typescript
import { recordPatternUse } from "./tools/record-pattern-use.js";
```

Add tool registration (after `install_skills` tool, before `return server`):
```typescript
  server.tool(
    "record_pattern_use",
    "Record that a named pattern from PATTERNS.md is being applied in this session — call at the start of any pattern-guided implementation",
    {
      pattern_name: z.string().describe("Pattern name exactly as it appears in PATTERNS.md"),
    },
    async ({ pattern_name }) => {
      try {
        const client = await getClient();
        if (!client) return { content: [{ type: "text", text: "Error: Supabase client not configured." }] };
        const result = await recordPatternUse(client as any, opts.repoId, sessionId, pattern_name);
        beat(client, "record_pattern_use", true);
        return { content: [{ type: "text", text: result }] };
      } catch (err) {
        return { content: [{ type: "text", text: `Error: ${err instanceof Error ? err.message : String(err)}` }], isError: true };
      }
    }
  );
```

- [ ] **Step 4: Run test to verify it passes**

```bash
cd packages/server && bunx vitest run src/mcp-server.spec.ts
```
Expected: 2 tests PASS

- [ ] **Step 5: Commit**

```bash
git add packages/server/src/mcp-server.ts packages/server/src/mcp-server.spec.ts
git commit -m "feat(server): register record_pattern_use MCP tool"
```

---

### Task 4: Extend `checkpoint.ts` to close out pattern usage rows

**Files:**
- Modify: `packages/server/src/tools/checkpoint.ts`
- Create: `packages/server/src/tools/checkpoint.spec.ts`

- [ ] **Step 1: Write the failing test**

```typescript
// packages/server/src/tools/checkpoint.spec.ts
import { describe, it, expect, vi } from "vitest";

// Mock @sensei/engine before importing checkpoint
vi.mock("@sensei/engine", () => ({
  takeSnapshot: vi.fn().mockResolvedValue({
    id: "snap-1",
    kind: "checkpoint",
    progressSummary: "done",
    createdAt: new Date().toISOString(),
  }),
}));

// Mock child_process exec for git diff
vi.mock("child_process", () => ({
  exec: vi.fn((_cmd: string, cb: (err: null, out: { stdout: string }) => void) =>
    cb(null, { stdout: "src/foo.ts\nsrc/bar.ts\n" })
  ),
}));

import { checkpointTool } from "./checkpoint.js";

function makeClient(patternUsageRows: any[]) {
  const updates: any[] = [];
  return {
    // Top-level .from() — used only for "sessions" table
    from: (table: string) => {
      if (table === "sessions") {
        return {
          update: () => ({ eq: () => ({ then: (r: any) => r({ error: null }) }) }),
        };
      }
      return {};
    },
    // .schema("sensei").from() — used for pattern_usages
    schema: () => ({
      from: (table: string) => {
        if (table === "pattern_usages") {
          return {
            update: (vals: any) => ({
              eq: (_c1: string, _v1: any) => ({
                in: (_c: string, _v: any[]) => {
                  updates.push(vals);
                  return { then: (r: any) => r({ error: null }) };
                },
              }),
            }),
            select: (_cols: string) => ({
              eq: (_c1: string, _v1: any) => ({
                is: (_c2: string, _v2: any) => ({
                  then: (r: any) => r({ data: patternUsageRows, error: null }),
                }),
              }),
            }),
          };
        }
        return {};
      },
    }),
    updates,
  };
}

describe("checkpointTool", () => {
  it("updates open pattern_usages rows with outcome and files_modified", async () => {
    const rows = [{ id: "pu-1", session_id: "sess-1", outcome: null }];
    const client = makeClient(rows);

    await checkpointTool(client as any, "sess-1", "repo-1", {
      task_summary: "done",
    });

    expect(client.updates.length).toBeGreaterThan(0);
    const update = client.updates[0];
    expect(update).toHaveProperty("outcome", "completed");
    expect(update).toHaveProperty("files_modified");
    // Note: ftr_score propagation is deferred — requires engine to expose score
  });

  it("skips pattern_usages update when no open rows exist", async () => {
    const client = makeClient([]);
    await expect(
      checkpointTool(client as any, "sess-1", "repo-1", { task_summary: "done" })
    ).resolves.toBeDefined();
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cd packages/server && bunx vitest run src/tools/checkpoint.spec.ts
```
Expected: FAIL — snapshot mock may pass but pattern_usages update is not called

- [ ] **Step 3: Extend `checkpoint.ts`**

> **Note on `ftr_score`:** The spec lists `ftr_score` as a field to populate at checkpoint. The current engine does not expose a computed FTR score from `takeSnapshot`. Populating `ftr_score` is deferred to a future task that wires the FTR computation result through to this call site.

> **Note on `git diff` range:** The spec calls for `git diff --name-only <session_start_commit>..HEAD`. The current system does not store the session start commit hash. We use `HEAD~1..HEAD` as a best-effort approximation (files changed in the last commit). A future improvement would store the HEAD commit at session creation and pass it here.

```typescript
// packages/server/src/tools/checkpoint.ts
import type { SupabaseClient } from "@supabase/supabase-js";
import { takeSnapshot } from "@sensei/engine";
import { exec } from "child_process";
import { promisify } from "util";

const execAsync = promisify(exec);

interface CheckpointParams {
  task_summary: string;
  completed_steps?: string[];
}

async function getChangedFiles(repoPath: string): Promise<string[]> {
  try {
    // Best-effort: files changed in the most recent commit.
    // Full spec requires session_start_commit..HEAD — deferred until start commit is stored.
    const { stdout } = await execAsync("git diff --name-only HEAD~1..HEAD", { cwd: repoPath });
    return stdout.trim().split("\n").filter(Boolean);
  } catch {
    return [];
  }
}

export async function checkpointTool(
  client: SupabaseClient,
  sessionId: string,
  repoId: string,
  params: CheckpointParams,
  repoPath?: string,
) {
  // Write checkpoint snapshot
  const snapshot = await takeSnapshot(client, sessionId, repoId, {
    kind: "checkpoint",
    progressSummary: params.task_summary,
    completedSteps: params.completed_steps,
  });

  // Mark session completed
  const { error: updateError } = await client
    .from("sessions")
    .update({ status: "completed" })
    .eq("id", sessionId);
  if (updateError) throw new Error(updateError.message ?? "Failed to mark session completed");

  // Close out any open pattern_usages rows for this session
  const { data: openRows } = await (client as any)
    .schema("sensei")
    .from("pattern_usages")
    .select("id")
    .eq("session_id", sessionId)
    .is("outcome", null);

  if (openRows && openRows.length > 0) {
    const filesModified = repoPath ? await getChangedFiles(repoPath) : [];
    const ids = openRows.map((r: any) => r.id);
    await (client as any)
      .schema("sensei")
      .from("pattern_usages")
      .update({
        outcome: "completed",
        files_modified: filesModified,
        // ftr_score: deferred — requires engine to expose computed score
      })
      .eq("session_id", sessionId)
      .in("id", ids);
  }

  return {
    id: snapshot.id,
    kind: snapshot.kind,
    progressSummary: snapshot.progressSummary,
    createdAt: snapshot.createdAt,
  };
}
```

- [ ] **Step 4: Update `mcp-server.ts` to pass `repoPath` to `checkpointTool`**

In `packages/server/src/mcp-server.ts`, find line 224 (the `checkpointTool` call):

```typescript
// Current (line 224):
const result = await checkpointTool(client as any, sessionId, opts.repoId, params);
// Change to:
const result = await checkpointTool(client as any, sessionId, opts.repoId, params, opts.repoPath);
```

- [ ] **Step 5: Run tests**

```bash
cd packages/server && bunx vitest run src/tools/checkpoint.spec.ts src/mcp-server.spec.ts
```
Expected: all pass

- [ ] **Step 6: Run full server test suite**

```bash
bun run --filter '@sensei/server' test
```
Expected: zero failures

- [ ] **Step 7: TypeScript check**

```bash
bunx tsc --noEmit -p packages/server/tsconfig.json
```
Expected: zero errors

- [ ] **Step 8: Commit**

```bash
git add packages/server/src/tools/checkpoint.ts packages/server/src/tools/checkpoint.spec.ts packages/server/src/mcp-server.ts
git commit -m "feat(server): extend checkpoint to close pattern_usages rows on session end"
```

---

## Chunk 2: Skill Files and Catalog Update

### Task 5: `skills/identifying-patterns/SKILL.md`

**Files:**
- Create: `skills/identifying-patterns/SKILL.md`

- [ ] **Step 1: Write the skill file**

```markdown
---
name: identifying-patterns
description: Use before implementing any feature that might follow a recurring structure
in this codebase — discovers existing patterns (adapters, CLI commands, MCP tools,
dashboard routes) and writes a reusable skill recipe so future implementations follow
the same shape. Also use when adding a new structural pattern to document it permanently.
---

# Identifying Patterns

## Overview

This skill finds recurring structural patterns in the codebase, extracts the shared interface and invariants, writes a dedicated skill file for the pattern, and registers it in `PATTERNS.md`.

## When to Use

- Before implementing something that looks like it might already exist elsewhere in the codebase (e.g., "I need to add an MCP tool" — there are already 12 of them)
- When a user names a domain ("adapters", "CLI commands", "dashboard routes") and asks to add to it
- Periodically to refresh the pattern catalog after significant code changes

## Procedure

### First Run (Guided — seed a new pattern)

1. User names a domain or structural pattern (e.g., "MCP tools", "CLI commands", "adapters")
2. Search codebase for 2+ existing implementations using `search()` + `context_pack()`
3. Read 2-3 canonical examples; extract:
   - **Interface** — types/signatures every implementation shares
   - **Required exports** — what must be exported and consumed
   - **Registration step** — where implementations are registered (e.g., `mcp-server.ts`, `cli.ts`)
   - **Invariants** — what must never be omitted or renamed
4. Write `skills/<pattern-name>/SKILL.md` using the template below
5. Append an entry to `PATTERNS.md`

### Incremental Run (automated refresh)

1. Read existing `PATTERNS.md` catalog
2. For each entry, load its canonical example file
3. Compare current file content against the recipe in the pattern skill
4. If diverged: flag as stale, propose updated recipe
5. If new patterns detected (2+ implementations of same structure not yet cataloged): offer to document them

## Pattern Skill Template

```markdown
---
name: <pattern-name>
description: Use when implementing <X> in this codebase — provides the established
recipe so new implementations match the existing structure.
---

# <Pattern Name> Pattern

## What It Solves
<1-2 sentences on why this pattern exists in this codebase>

## Interface
<The types/signatures every implementation of this pattern must produce>

## Canonical Example
File: `<path/to/example.ts>`
<Key excerpt — the critical lines that show the pattern in use>

## Recipe
1. Create `<path>` with this structure: ...
2. Implement these exports: ...
3. Register in `<path>`: ...

## Invariants
- <What must never be omitted>
- <What naming convention is mandatory>
- <What must be tested>
```

## Updating PATTERNS.md

Each entry in `PATTERNS.md` follows this format:

```markdown
## <Pattern Name>

**What it solves:** <one line>
**Canonical example:** `<file path>`
**Skill:** `skills/<pattern-name>/SKILL.md`
```
```

- [ ] **Step 2: Verify the file was created**

```bash
cat skills/identifying-patterns/SKILL.md
```
Expected: file prints the full skill content.

- [ ] **Step 3: Commit**

```bash
git add skills/identifying-patterns/SKILL.md
git commit -m "feat(skills): add identifying-patterns skill"
```

---

### Task 6: `skills/pattern-based-development/SKILL.md`

**Files:**
- Create: `skills/pattern-based-development/SKILL.md`

- [ ] **Step 1: Write the skill file**

```markdown
---
name: pattern-based-development
description: Use before implementing any new feature, component, module, or integration
— checks PATTERNS.md for an applicable recipe before writing new code. Prevents
re-inventing structure that already exists in this codebase.
---

# Pattern-Based Development

## Overview

Before writing new code, check whether a structural pattern already exists for what you're about to implement. If it does, load the pattern's skill file and follow its recipe. Record the pattern use for tracking.

## Procedure

1. Read `PATTERNS.md` (repo root) — scan for patterns relevant to the current task
2. Match task description to pattern entries:
   - "Add an MCP tool" → `mcp-tool` pattern
   - "Add a CLI command" → `cli-command` pattern
   - "Add a new adapter" → `adapter` pattern
3. **If pattern found:**
   a. Load `skills/<pattern-name>/SKILL.md`
   b. Call `record_pattern_use("<pattern-name>")` MCP tool to start tracking
   c. Present the recipe to the agent before implementation
   d. Implement following the recipe exactly — file structure, exports, registration
4. **If no matching pattern:**
   - Proceed with standard implementation
   - Consider running `identifying-patterns` skill to document this new pattern afterwards
5. At session end, `checkpoint()` automatically links outcome and changed files to the pattern usage row

## Why This Matters

When every implementation of a pattern follows the same recipe:
- Reviewers know exactly what to look for
- New contributors can follow the same steps
- The codebase stays internally consistent

## Example

Task: "Add a `record_pattern_use` MCP tool"

1. Read `PATTERNS.md` → find `mcp-tool` pattern entry
2. Load `skills/mcp-tool/SKILL.md`
3. Call `record_pattern_use("mcp-tool")`
4. Follow recipe:
   - Create `packages/server/src/tools/record-pattern-use.ts`
   - Export `recordPatternUse(client, repoId, sessionId, patternName)`
   - Import and register in `mcp-server.ts` with `server.tool(...)`
   - Write tests in `record-pattern-use.spec.ts`
```

- [ ] **Step 2: Verify the file was created**

```bash
cat skills/pattern-based-development/SKILL.md
```
Expected: file prints the full skill content.

- [ ] **Step 3: Commit**

```bash
git add skills/pattern-based-development/SKILL.md
git commit -m "feat(skills): add pattern-based-development skill"
```

---

### Task 7: Update `PATTERNS.md` with catalog structure and seed first pattern

**Files:**
- Modify: `PATTERNS.md`

- [ ] **Step 1: Update PATTERNS.md with catalog header and MCP tool pattern entry**

Replace the current content:

```markdown
# Patterns

<!-- Catalog of recurring structural patterns in this codebase.
     Each entry links to a skill file with the full implementation recipe.
     Generated and maintained by the identifying-patterns skill. -->

---

## MCP Tool

**What it solves:** Adding new capabilities to the sensei MCP server
**Canonical example:** `packages/server/src/tools/record-pattern-use.ts`
**Skill:** `skills/mcp-tool/SKILL.md` *(pending — run `identifying-patterns` to generate)*
```

- [ ] **Step 2: Verify**

```bash
cat PATTERNS.md
```
Expected: header comment + MCP Tool entry printed.

- [ ] **Step 3: Commit**

```bash
git add PATTERNS.md
git commit -m "docs: add pattern catalog structure to PATTERNS.md"
```

---

## Final Verification

- [ ] **Run full test suite**

```bash
bun run --filter '*' test
```
Expected: zero failures

- [ ] **TypeScript check**

```bash
bunx tsc --noEmit
```
Expected: zero errors

- [ ] **Verify new skills appear in skills/ directory**

```bash
ls skills/
```
Expected: `identifying-patterns/` and `pattern-based-development/` listed alongside existing skills.
