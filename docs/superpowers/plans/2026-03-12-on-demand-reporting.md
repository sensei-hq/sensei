# On-Demand Drift Reporting Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Return structured JSON from the `check_drift` MCP tool and add a `--drift` flag to `sensei watch` that prints drift summaries after reindex.

**Architecture:** Two independent changes — one line in the MCP server to serialize the full `DriftResult` as JSON, and a new `drift` option in `watch.ts` that calls `checkDrift` after each successful reindex and prints only when drift is found.

**Tech Stack:** TypeScript, Bun, Vitest, `@sensei/tools` (`checkDrift`), `@modelcontextprotocol/sdk`

**Spec:** `docs/superpowers/specs/2026-03-12-on-demand-reporting-design.md`

---

## Chunk 1: Structured MCP output + watch --drift flag

### Task 1: Structured JSON from `check_drift` MCP tool

**Files:**
- Modify: `packages/mcp/src/index.ts:68-74`

The `check_drift` tool currently returns `result.summary` (a prose string). Change it to return the full `DriftResult` serialized as JSON so agents can iterate `drifted[]` directly.

Current code at line 68–74:
```typescript
server.tool("check_drift", "Check if indexed docs have drifted from the current state",
  {},
  async () => {
    const result = await checkDrift(REPO);
    return { content: [{ type: "text", text: result.summary }] };
  }
);
```

- [ ] **Step 1: Update `check_drift` MCP tool to return JSON**

Change `result.summary` to `JSON.stringify(result, null, 2)`:

```typescript
server.tool("check_drift", "Check if indexed docs have drifted from the current state",
  {},
  async () => {
    const result = await checkDrift(REPO);
    return { content: [{ type: "text", text: JSON.stringify(result, null, 2) }] };
  }
);
```

- [ ] **Step 2: Verify the MCP package builds**

```bash
cd packages/mcp && bun run build 2>&1 | tail -5
```

Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add packages/mcp/src/index.ts
git commit -m "feat(mcp): return structured JSON from check_drift tool"
```

---

### Task 2: `sensei watch --drift` flag

**Files:**
- Modify: `packages/cli/src/cli.ts` — add `drift` to `parseArgs` options + help text + pass to `watch()`
- Modify: `packages/cli/src/commands/watch.ts` — accept `{ drift?: boolean }`, call `checkDrift` after reindex if set
- Create: `packages/cli/src/commands/watch.spec.ts` — unit tests

#### Step group A: Tests first

- [ ] **Step 1: Create `watch.spec.ts` with failing tests**

```typescript
// packages/cli/src/commands/watch.spec.ts
import { describe, it, expect, vi, beforeEach } from "vitest";

// Mock @sensei/tools before importing watch
vi.mock("@sensei/tools", () => ({
  reindexRepo: vi.fn(),
  checkDrift: vi.fn(),
}));

// Mock chokidar
vi.mock("chokidar", () => ({
  default: {
    watch: vi.fn(() => ({
      on: vi.fn().mockReturnThis(),
      close: vi.fn().mockResolvedValue(undefined),
    })),
  },
}));

// Mock fs
vi.mock("fs", () => ({
  existsSync: vi.fn(() => true),
}));

import { watch } from "./watch.js";
import { reindexRepo, checkDrift } from "@sensei/tools";
import chokidar from "chokidar";

const mockReindex = reindexRepo as ReturnType<typeof vi.fn>;
const mockCheckDrift = checkDrift as ReturnType<typeof vi.fn>;
const mockChokidar = chokidar.watch as ReturnType<typeof vi.fn>;

describe("watch --drift", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockReindex.mockResolvedValue({ added: 0, updated: 1, removed: 0, unchanged: 5, total: 6, skipped: 0, forced: false });
    mockCheckDrift.mockResolvedValue({ drifted: [], summary: "No drift detected.", lastIndexedCommit: "abc1234" });
  });

  it("does NOT call checkDrift when drift option is false", async () => {
    // Simulate a change event triggering reindex
    let changeHandler: () => void = () => {};
    mockChokidar.mockReturnValue({
      on: vi.fn((event: string, handler: () => void) => {
        if (event === "change") changeHandler = handler;
        return { on: vi.fn().mockReturnThis(), close: vi.fn().mockResolvedValue(undefined) };
      }),
      close: vi.fn().mockResolvedValue(undefined),
    });

    // We need to trigger the watch loop and then stop it
    // Use a SIGINT simulation approach: resolve after first reindex
    mockReindex.mockImplementation(async () => {
      process.emit("SIGINT");
      return { added: 0, updated: 1, removed: 0, unchanged: 5, total: 6, skipped: 0, forced: false };
    });

    await watch("/repo", { drift: false });
    expect(mockCheckDrift).not.toHaveBeenCalled();
  });

  it("calls checkDrift after reindex when drift option is true", async () => {
    mockReindex.mockImplementation(async () => {
      process.emit("SIGINT");
      return { added: 0, updated: 1, removed: 0, unchanged: 5, total: 6, skipped: 0, forced: false };
    });

    await watch("/repo", { drift: true });
    expect(mockCheckDrift).toHaveBeenCalledWith("/repo");
  });

  it("prints drift summary when drift found", async () => {
    const consoleSpy = vi.spyOn(console, "log").mockImplementation(() => {});
    mockCheckDrift.mockResolvedValue({
      drifted: [{ docPath: "docs/design/03-mcp-server.md", reason: "code-changed", changedFiles: ["packages/mcp/src/index.ts"] }],
      summary: "1 doc(s) drifted since abc1234:\ndocs/design/03-mcp-server.md: code changed — packages/mcp/src/index.ts",
    });
    mockReindex.mockImplementation(async () => {
      process.emit("SIGINT");
      return { added: 0, updated: 1, removed: 0, unchanged: 5, total: 6, skipped: 0, forced: false };
    });

    await watch("/repo", { drift: true });

    expect(consoleSpy).toHaveBeenCalledWith(expect.stringContaining("1 doc(s) drifted"));
    consoleSpy.mockRestore();
  });

  it("is silent when no drift found", async () => {
    const consoleSpy = vi.spyOn(console, "log").mockImplementation(() => {});
    mockReindex.mockImplementation(async () => {
      process.emit("SIGINT");
      return { added: 0, updated: 1, removed: 0, unchanged: 5, total: 6, skipped: 0, forced: false };
    });

    await watch("/repo", { drift: true });

    // Should not log the "No drift detected" message (silent on clean)
    const driftLogs = consoleSpy.mock.calls.filter(c => String(c[0]).includes("drift"));
    expect(driftLogs).toHaveLength(0);
    consoleSpy.mockRestore();
  });

  it("continues watching if checkDrift throws", async () => {
    const consoleWarnSpy = vi.spyOn(console, "warn").mockImplementation(() => {});
    mockCheckDrift.mockRejectedValue(new Error("drift check failed"));
    mockReindex.mockImplementation(async () => {
      process.emit("SIGINT");
      return { added: 0, updated: 1, removed: 0, unchanged: 5, total: 6, skipped: 0, forced: false };
    });

    // Should not throw
    await expect(watch("/repo", { drift: true })).resolves.toBeUndefined();
    consoleWarnSpy.mockRestore();
  });
});
```

- [ ] **Step 2: Run tests to confirm they fail**

```bash
cd packages/cli && bunx vitest run src/commands/watch.spec.ts 2>&1 | tail -15
```

Expected: FAIL — `watch` function doesn't accept options yet.

#### Step group B: Implementation

- [ ] **Step 3: Update `watch.ts` to accept `{ drift?: boolean }` and call `checkDrift`**

Replace the entire `watch` function signature and add drift logic after the reindex `.then()`:

```typescript
// packages/cli/src/commands/watch.ts
import chokidar from "chokidar";
import { join } from "path";
import { existsSync } from "fs";
import { reindexRepo, checkDrift } from "@sensei/tools";

export interface WatchOptions {
  drift?: boolean;
}

export async function watch(repoPath: string, options: WatchOptions = {}): Promise<void> {
  const watchTargets = [
    join(repoPath, "src"),
    join(repoPath, "docs"),
    join(repoPath, "package.json"),
  ].filter(p => existsSync(p));

  if (watchTargets.length === 0) {
    console.log("Nothing to watch — no src/, docs/, or package.json found.");
    return;
  }

  let debounceTimer: ReturnType<typeof setTimeout> | null = null;
  let reindexPromise: Promise<void> | null = null;

  const watcher = chokidar.watch(watchTargets, {
    ignored: [
      /\.sensei\//,
      /node_modules/,
      /\.git\//,
    ],
    ignoreInitial: true,
    persistent: true,
  });

  function triggerReindex() {
    if (debounceTimer) clearTimeout(debounceTimer);
    debounceTimer = setTimeout(async () => {
      debounceTimer = null;
      if (reindexPromise) return; // skip — reindex in flight
      const start = Date.now();
      reindexPromise = reindexRepo(repoPath)
        .then(async summary => {
          const changed = summary.added + summary.updated + summary.removed;
          const elapsed = Date.now() - start;
          console.log(`reindexed ${changed} files (${elapsed}ms)`);
          if (options.drift) {
            try {
              const drift = await checkDrift(repoPath);
              if (drift.drifted.length > 0) {
                console.log(drift.summary);
              }
            } catch (err) {
              console.warn("drift check failed:", (err as Error).message);
            }
          }
        })
        .catch(err => console.error("reindex error:", err.message))
        .finally(() => { reindexPromise = null; });
    }, 500);
  }

  watcher.on("change", triggerReindex);
  watcher.on("add", triggerReindex);
  watcher.on("unlink", triggerReindex);

  console.log(`Watching ${watchTargets.map(p => p.replace(repoPath + "/", "")).join(", ")}... (Ctrl+C to stop)`);

  await new Promise<void>(resolve => {
    process.once("SIGINT", async () => {
      if (debounceTimer) clearTimeout(debounceTimer);
      const inFlight = reindexPromise;
      if (inFlight) await inFlight;
      await watcher.close();
      console.log("Watch stopped.");
      resolve();
    });
  });
}
```

- [ ] **Step 4: Update `cli.ts` — add `drift` option to `parseArgs` and pass to `watch()`**

Add `drift` to the `options` object in `parseArgs` (after `hooks`):

```typescript
    hooks: { type: "boolean", default: false },
    drift: { type: "boolean", default: false },
```

Add `--drift` to the watch help section (after `--repo <path>`):

```
watch:
  --repo <path>            Repo to watch (default: auto-detected repo root)
  --drift                  Check for doc drift after each reindex (prints only when drift found)
```

Update the `watch` case to pass the option:

```typescript
    case "watch": {
      const { watch } = await import("./commands/watch.js");
      const repo = values.repo ?? repoRoot;
      await watch(repo, { drift: values.drift });
      break;
    }
```

- [ ] **Step 5: Run tests — confirm they pass**

```bash
cd packages/cli && bunx vitest run src/commands/watch.spec.ts 2>&1 | tail -15
```

Expected: 5 pass, 0 fail.

- [ ] **Step 6: Run full test suite**

```bash
bun test 2>&1 | tail -5
```

Expected: same pass count as before (241+) with 0 new failures.

- [ ] **Step 7: Commit**

```bash
git add packages/cli/src/cli.ts packages/cli/src/commands/watch.ts packages/cli/src/commands/watch.spec.ts
git commit -m "feat(cli): add --drift flag to sensei watch"
```

---

### Task 3: Update traceability

**Files:**
- Modify: `docs/traceability.yaml` — mark `on-demand-reporting` as done

- [ ] **Step 1: Mark `on-demand-reporting` as done in `docs/traceability.yaml`**

Find:
```yaml
      - id: on-demand-reporting
        section: "#on-demand-drift-reporting"
        status: planned
```

Replace with:
```yaml
      - id: on-demand-reporting
        section: "#on-demand-drift-reporting"
        status: done
```

- [ ] **Step 2: Commit**

```bash
git add docs/traceability.yaml
git commit -m "docs(traceability): mark on-demand-reporting as done"
```
