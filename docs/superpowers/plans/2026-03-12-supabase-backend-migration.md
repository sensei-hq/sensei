# Supabase Backend Migration + Config Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Wire the sensei backend (collector daemon, indexer, MCP) to read/write from Supabase instead of SQLite/JSON files. Add `.sensei/config.yaml` (repo_id + supabase_url) and `~/.config/sensei/credentials.yaml` (service key) support.

**Architecture:** Three phases. (1) Config + Supabase client in `packages/shared` — zero runtime deps change until used. (2) Collector migration — replace SQLite write path with Supabase upsert (keep JSONL fallback). (3) Indexer migration — write symbols, chunks, docs to Supabase after each reindex; search reads from Supabase instead of JSON files.

**Tech Stack:** TypeScript, Bun, `@supabase/supabase-js`, `js-yaml`, `@sensei/shared`, Vitest

**Prerequisites:** Complete `2026-03-12-supabase-database-setup.md` first — Supabase must be running with the `sensei` schema applied. A `repo_id` must exist in `sensei.repos` for the repo under test.

**Spec:** `docs/superpowers/specs/2026-03-12-supabase-central-store-design.md`

---

## Chunk 1: Config loader + Supabase client (packages/shared)

### Task 1: Config reader — `.sensei/config.yaml` + `~/.config/sensei/credentials.yaml`

**Files:**
- Create: `packages/shared/src/config.ts`
- Modify: `packages/shared/src/index.ts` — export new module
- Modify: `packages/shared/package.json` — add `js-yaml` dependency

#### Step group A: Tests first

- [ ] **Step 1: Add js-yaml to shared package**

```bash
cd packages/shared
bun add js-yaml
bun add -d @types/js-yaml
```

- [ ] **Step 2: Create `packages/shared/src/config.spec.ts`**

```typescript
import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { mkdir, writeFile, rm } from "fs/promises";
import { join } from "path";
import os from "os";

// Tests use real temp directories — no mocking of fs
describe("loadSenseiConfig", () => {
  let tmpRepo: string;
  let tmpHome: string;

  beforeEach(async () => {
    tmpRepo = await makeTmpDir("repo");
    tmpHome = await makeTmpDir("home");
  });

  afterEach(async () => {
    await rm(tmpRepo, { recursive: true, force: true });
    await rm(tmpHome, { recursive: true, force: true });
  });

  it("returns null when .sensei/config.yaml does not exist", async () => {
    const { loadSenseiConfig } = await import("./config.js");
    const result = await loadSenseiConfig(tmpRepo);
    expect(result).toBeNull();
  });

  it("reads repo_id and supabase_url from .sensei/config.yaml", async () => {
    const { loadSenseiConfig } = await import("./config.js");
    await mkdir(join(tmpRepo, ".sensei"), { recursive: true });
    await writeFile(join(tmpRepo, ".sensei", "config.yaml"),
      "repo_id: abc-123\nsupabase_url: http://localhost:54321\n");
    const result = await loadSenseiConfig(tmpRepo);
    expect(result?.repo_id).toBe("abc-123");
    expect(result?.supabase_url).toBe("http://localhost:54321");
  });

  it("reads service key from credentials.yaml", async () => {
    const { loadCredentials } = await import("./config.js");
    await mkdir(join(tmpHome, ".config", "sensei"), { recursive: true });
    await writeFile(join(tmpHome, ".config", "sensei", "credentials.yaml"),
      "supabase_service_key: sk-test-key\n");
    const result = await loadCredentials(tmpHome);
    expect(result?.supabase_service_key).toBe("sk-test-key");
  });

  it("returns null for credentials when file does not exist", async () => {
    const { loadCredentials } = await import("./config.js");
    const result = await loadCredentials(tmpHome);
    expect(result).toBeNull();
  });

  it("prefers SUPABASE_SERVICE_KEY env over credentials file", async () => {
    const { loadCredentials } = await import("./config.js");
    process.env.SUPABASE_SERVICE_KEY = "env-key";
    const result = await loadCredentials(tmpHome);
    expect(result?.supabase_service_key).toBe("env-key");
    delete process.env.SUPABASE_SERVICE_KEY;
  });
});

async function makeTmpDir(prefix: string): Promise<string> {
  const dir = join(os.tmpdir(), `sensei-config-test-${prefix}-${Date.now()}`);
  await mkdir(dir, { recursive: true });
  return dir;
}
```

- [ ] **Step 3: Run tests to confirm they fail**

```bash
cd packages/shared && bunx vitest run src/config.spec.ts 2>&1 | tail -10
```

Expected: FAIL — `loadSenseiConfig` not found.

#### Step group B: Implementation

- [ ] **Step 4: Create `packages/shared/src/config.ts`**

```typescript
import { readFile, access } from "fs/promises";
import { join } from "path";
import yaml from "js-yaml";
import os from "os";

export interface SenseiRepoConfig {
  repo_id: string;
  supabase_url: string;
}

export interface SenseiCredentials {
  supabase_service_key: string;
}

/** Read .sensei/config.yaml from repoPath. Returns null if missing. */
export async function loadSenseiConfig(repoPath: string): Promise<SenseiRepoConfig | null> {
  const configPath = join(repoPath, ".sensei", "config.yaml");
  try {
    await access(configPath);
    const raw = await readFile(configPath, "utf-8");
    return yaml.load(raw) as SenseiRepoConfig;
  } catch {
    return null;
  }
}

/** Read credentials from ~/.config/sensei/credentials.yaml, or SUPABASE_SERVICE_KEY env. */
export async function loadCredentials(homeDir?: string): Promise<SenseiCredentials | null> {
  if (process.env.SUPABASE_SERVICE_KEY) {
    return { supabase_service_key: process.env.SUPABASE_SERVICE_KEY };
  }
  // homeDir is an injection point for tests; production uses os.homedir()
  const home = homeDir ?? os.homedir();
  const credPath = join(home, ".config", "sensei", "credentials.yaml");
  try {
    await access(credPath);
    const raw = await readFile(credPath, "utf-8");
    return yaml.load(raw) as SenseiCredentials;
  } catch {
    return null;
  }
}
```

- [ ] **Step 5: Export from `packages/shared/src/index.ts`**

Add to the end of `packages/shared/src/index.ts`:
```typescript
export * from "./config.js";
```

- [ ] **Step 6: Run tests to confirm they pass**

```bash
cd packages/shared && bunx vitest run src/config.spec.ts 2>&1 | tail -10
```

Expected: 5 pass, 0 fail.

- [ ] **Step 7: Run full test suite**

```bash
cd /path/to/sensei && bun test 2>&1 | tail -5
```

Expected: same pass count as before, 0 new failures.

- [ ] **Step 8: Commit**

```bash
git add packages/shared/
git commit -m "feat(shared): add config loader for .sensei/config.yaml + credentials"
```

---

### Task 2: Supabase client factory in shared

**Files:**
- Create: `packages/shared/src/supabase-client.ts`
- Modify: `packages/shared/src/index.ts` — export new module
- Modify: `packages/shared/package.json` — add `@supabase/supabase-js`

This is a thin factory — no unit tests needed (pure credential wiring).

- [ ] **Step 1: Add @supabase/supabase-js to shared**

```bash
cd packages/shared
bun add @supabase/supabase-js
```

- [ ] **Step 2: Create `packages/shared/src/supabase-client.ts`**

```typescript
import { createClient, type SupabaseClient } from "@supabase/supabase-js";
import { loadSenseiConfig, loadCredentials } from "./config.js";

/** Build a service-role Supabase client scoped to the `sensei` schema. Returns null if config missing. */
export async function makeSenseiClient(repoPath: string): Promise<SupabaseClient | null> {
  const [config, creds] = await Promise.all([
    loadSenseiConfig(repoPath),
    loadCredentials(),
  ]);
  if (!config || !creds) return null;
  // db.schema scopes all queries to sensei.* — no need for .schema() chaining on each call
  return createClient(config.supabase_url, creds.supabase_service_key, {
    db: { schema: "sensei" },
    auth: { persistSession: false },
  });
}
```

- [ ] **Step 3: Export from `packages/shared/src/index.ts`**

```typescript
export * from "./supabase-client.js";
```

- [ ] **Step 4: Typecheck**

```bash
cd packages/shared && bunx tsc --noEmit
```

Expected: no errors.

- [ ] **Step 5: Commit**

```bash
git add packages/shared/
git commit -m "feat(shared): add Supabase client factory — makeSenseiClient"
```

---

## Chunk 2: Collector migration — SQLite → Supabase

### Task 3: Migrate collector daemon to write to `sensei.events`

**Files:**
- Create: `packages/collector/src/supabase-writer.ts`
- Modify: `packages/collector/src/daemon.ts` — dual-write: SQLite + Supabase
- Modify: `packages/collector/package.json` — add `@supabase/supabase-js`

**Strategy:** Keep SQLite as fallback (JSONL drain still works). When `makeSenseiClient()` returns a client, write events to Supabase in addition to SQLite. This is dual-write — no data loss risk during migration.

- [ ] **Step 1: Add `@supabase/supabase-js` as a dev dependency to collector**

The collector imports `SupabaseClient` as a type only (in `supabase-writer.ts`). The actual client is constructed in `packages/shared`. Add as devDependency for types:

```bash
cd packages/collector
bun add -d @supabase/supabase-js
```

- [ ] **Step 2: Create `packages/collector/src/supabase-writer.spec.ts`**

```typescript
import { describe, it, expect, vi, beforeEach } from "vitest";
import { writeEventToSupabase } from "./supabase-writer.js";

const mockInsert = vi.fn().mockReturnValue({ error: null });
const mockFrom = vi.fn(() => ({ insert: mockInsert }));
// Client created with db.schema at construction — no .schema() chaining needed
const mockClient = { from: mockFrom } as any;

describe("writeEventToSupabase", () => {
  beforeEach(() => vi.clearAllMocks());

  it("inserts event with correct schema and fields", async () => {
    await writeEventToSupabase(mockClient, {
      user_uuid: "user-1",
      session_id: "sess-1",
      repo_id: "repo-uuid",
      phase: "post",
      tool: "Edit",
      project_path: "/projects/foo",
      input: { file: "src/a.ts" },
      ts: new Date("2026-01-01T00:00:00Z"),
    });

    expect(mockFrom).toHaveBeenCalledWith("events");
    const inserted = mockInsert.mock.calls[0][0];
    expect(inserted.tool).toBe("Edit");
    expect(inserted.phase).toBe("post");
    expect(inserted.user_uuid).toBe("user-1");
  });

  it("does not throw when insert returns error", async () => {
    mockInsert.mockReturnValueOnce({ error: new Error("network error") });
    await expect(writeEventToSupabase(mockClient, {
      user_uuid: "u", session_id: "s", repo_id: null,
      phase: "pre", tool: "Bash", project_path: "/p",
      input: null, ts: new Date(),
    })).resolves.toBeUndefined();
  });
});
```

- [ ] **Step 3: Run tests to confirm they fail**

```bash
cd packages/collector && bunx vitest run src/supabase-writer.spec.ts 2>&1 | tail -5
```

Expected: FAIL — module not found.

- [ ] **Step 4: Create `packages/collector/src/supabase-writer.ts`**

```typescript
import type { SupabaseClient } from "@supabase/supabase-js";

export interface SupabaseEvent {
  user_uuid: string;
  session_id: string | null;
  repo_id: string | null;
  phase: "pre" | "post";
  tool: string;
  project_path: string;
  input: Record<string, unknown> | null;
  ts: Date;
}

/** Write a single event to sensei.events. Logs and swallows errors — never throws.
 *  The client must be created with db: { schema: "sensei" } (via makeSenseiClient). */
export async function writeEventToSupabase(
  client: SupabaseClient,
  event: SupabaseEvent,
): Promise<void> {
  const { error } = await client
    .from("events")
    .insert({
      user_uuid:    event.user_uuid,
      session_id:   event.session_id,
      repo_id:      event.repo_id,
      phase:        event.phase,
      tool:         event.tool,
      project_path: event.project_path,
      input:        event.input,
      ts:           event.ts.toISOString(),
    });
  if (error) {
    console.error("[collector] Supabase write error:", error.message);
  }
}
```

- [ ] **Step 5: Run tests to confirm they pass**

```bash
cd packages/collector && bunx vitest run src/supabase-writer.spec.ts 2>&1 | tail -5
```

Expected: 2 pass, 0 fail.

- [ ] **Step 6: Wire Supabase writer into daemon.ts**

In `packages/collector/src/daemon.ts`:

Add to `DaemonOptions` interface:
```typescript
repoPath?: string;   // used to resolve Supabase config
```

At the top of the file, add imports:
```typescript
import { makeSenseiClient } from "@sensei/shared";
import { writeEventToSupabase } from "./supabase-writer.js";
```

In `startDaemon()`, after `createTables(db)`:
```typescript
let supabaseClient: Awaited<ReturnType<typeof makeSenseiClient>> = null;
makeSenseiClient(opts.repoPath ?? process.cwd()).then(c => { supabaseClient = c; });
```

Inside the request handler, after the `insertEvent(db, body)` call succeeds:
```typescript
if (supabaseClient) {
  writeEventToSupabase(supabaseClient, {
    user_uuid:    body.user_uuid ?? "",
    session_id:   body.session_id ?? null,
    repo_id:      null,
    phase:        body.phase,
    tool:         body.tool,
    project_path: body.project_path ?? "",
    input:        body.input ? (() => { try { return JSON.parse(body.input!); } catch { return null; } })() : null,
    ts:           new Date(body.ts),
  }).catch(() => {});
}
```

- [ ] **Step 7: Run full test suite**

```bash
bun test 2>&1 | tail -5
```

Expected: same pass count as before, 0 new failures.

- [ ] **Step 8: Commit**

```bash
git add packages/collector/
git commit -m "feat(collector): dual-write events to Supabase + SQLite"
```

---

## Chunk 3: Indexer migration — write to Supabase

### Task 4: Write symbols and docs to Supabase after reindex

**Files:**
- Create: `packages/tools/src/tools/supabase-index-writer.ts`
- Create: `packages/tools/src/tools/supabase-index-writer.spec.ts`
- Modify: `packages/tools/src/tools/reindex.ts` — call writer after successful reindex

**Strategy:** After `reindexRepo()` finishes writing JSON files, also upsert symbols and docs to Supabase. JSON files remain as local cache. This makes Supabase authoritative without breaking existing search.

- [ ] **Step 1: Create `packages/tools/src/tools/supabase-index-writer.spec.ts`**

```typescript
import { describe, it, expect, vi, beforeEach } from "vitest";
import { upsertSymbols, upsertDocs } from "./supabase-index-writer.js";

const mockUpsert = vi.fn().mockReturnValue({ error: null });
const mockFrom = vi.fn(() => ({ upsert: mockUpsert }));
// Client has db.schema set at creation — no .schema() method needed on calls
const mockClient = { from: mockFrom } as any;

describe("upsertSymbols", () => {
  beforeEach(() => vi.clearAllMocks());

  it("upserts symbol rows keyed by (repo_id, file_path)", async () => {
    const symbolMap = {
      "src/a.ts": { L0: ["foo"], L1: ["function foo()"], L2: ["function foo() — does foo"] },
    };
    await upsertSymbols(mockClient, "repo-1", symbolMap);
    const rows = mockUpsert.mock.calls[0][0];
    expect(rows[0]).toMatchObject({ repo_id: "repo-1", file_path: "src/a.ts", l0: ["foo"] });
  });

  it("skips upsert when symbolMap is empty", async () => {
    await upsertSymbols(mockClient, "repo-1", {});
    expect(mockUpsert).not.toHaveBeenCalled();
  });
});

describe("upsertDocs", () => {
  beforeEach(() => vi.clearAllMocks());

  it("upserts doc coverage rows", async () => {
    const traceability = [
      { docPath: "docs/design/01.md", covers: ["src/a.ts"], autoDetected: false },
    ];
    await upsertDocs(mockClient, "repo-1", traceability);
    const rows = mockUpsert.mock.calls[0][0];
    expect(rows[0]).toMatchObject({ repo_id: "repo-1", doc_path: "docs/design/01.md", covers: ["src/a.ts"] });
  });
});
```

- [ ] **Step 2: Run tests to confirm they fail**

```bash
cd packages/tools && bunx vitest run src/tools/supabase-index-writer.spec.ts 2>&1 | tail -5
```

Expected: FAIL.

- [ ] **Step 3: Create `packages/tools/src/tools/supabase-index-writer.ts`**

```typescript
import type { SupabaseClient } from "@supabase/supabase-js";
import type { SymbolMap } from "@sensei/shared";

export interface TraceabilityEntry {
  docPath: string;
  covers: string[];
  autoDetected: boolean;
}

/** Upsert all symbol rows for a repo. No-op if symbolMap is empty.
 *  Client must be created with db: { schema: "sensei" } (via makeSenseiClient).
 *  Note: L1/L2 arrays are joined with "\n" — this is a deliberate lossy transformation
 *  since sensei.symbols stores them as text columns, not arrays. */
export async function upsertSymbols(
  client: SupabaseClient,
  repoId: string,
  symbolMap: SymbolMap,
): Promise<void> {
  const entries = Object.entries(symbolMap);
  if (entries.length === 0) return;

  const rows = entries.map(([file_path, s]) => ({
    repo_id:   repoId,
    file_path,
    l0: s.L0,
    l1: s.L1.join("\n"),
    l2: s.L2.join("\n"),
  }));

  const { error } = await client
    .from("symbols")
    .upsert(rows, { onConflict: "repo_id,file_path" });

  if (error) console.error("[indexer] Supabase symbols upsert error:", error.message);
}

/** Upsert all doc coverage rows for a repo. No-op if empty.
 *  Client must be created with db: { schema: "sensei" } (via makeSenseiClient). */
export async function upsertDocs(
  client: SupabaseClient,
  repoId: string,
  traceability: TraceabilityEntry[],
): Promise<void> {
  if (traceability.length === 0) return;

  const rows = traceability.map(t => ({
    repo_id:       repoId,
    doc_path:      t.docPath,
    covers:        t.covers,
    auto_detected: t.autoDetected,
  }));

  const { error } = await client
    .from("docs")
    .upsert(rows, { onConflict: "repo_id,doc_path" });

  if (error) console.error("[indexer] Supabase docs upsert error:", error.message);
}
```

- [ ] **Step 4: Run tests to confirm they pass**

```bash
cd packages/tools && bunx vitest run src/tools/supabase-index-writer.spec.ts 2>&1 | tail -5
```

Expected: 3 pass, 0 fail.

- [ ] **Step 5: Wire into reindex.ts**

In `packages/tools/src/tools/reindex.ts`, after the `Promise.all([...writeFile...])` block that writes JSON artifacts, add:

```typescript
// Dual-write symbols + docs to Supabase if config present
const supabaseConfig = await loadSenseiConfig(repoPath);
if (supabaseConfig) {
  const client = await makeSenseiClient(repoPath);
  if (client) {
    const { upsertSymbols, upsertDocs } = await import("./supabase-index-writer.js");
    await Promise.all([
      upsertSymbols(client, supabaseConfig.repo_id, symbolMap),
      upsertDocs(client, supabaseConfig.repo_id, traceability),
    ]);
  }
}
```

Add imports at the top of `reindex.ts`:
```typescript
import { loadSenseiConfig, makeSenseiClient } from "@sensei/shared";
```

The `traceability` variable in `reindex.ts` is `Record<string, string[]>` (docPath → covers[]). Convert it to `TraceabilityEntry[]` at the call site — the reindex code doesn't track `autoDetected` so default to `false`:

```typescript
// Convert Record<string, string[]> → TraceabilityEntry[]
const traceabilityEntries = Object.entries(traceability).map(([docPath, covers]) => ({
  docPath, covers, autoDetected: false,
}));
// ...
upsertDocs(client, supabaseConfig.repo_id, traceabilityEntries),
```

- [ ] **Step 6: Run full test suite**

```bash
bun test 2>&1 | tail -5
```

Expected: all tests pass. The Supabase writes only fire when `.sensei/config.yaml` exists — existing test repos don't have it.

- [ ] **Step 7: Commit**

```bash
git add packages/tools/
git commit -m "feat(tools): dual-write symbols and docs to Supabase after reindex"
```

---

## Chunk 4: Repo registration — generate `.sensei/config.yaml`

### Task 5: Register repo in Supabase and write config on first index

**Files:**
- Create: `packages/tools/src/tools/repo-registration.ts`
- Create: `packages/tools/src/tools/repo-registration.spec.ts`
- Modify: `packages/tools/src/tools/reindex.ts` — call on first index

- [ ] **Step 1: Create `packages/tools/src/tools/repo-registration.spec.ts`**

```typescript
import { describe, it, expect, vi } from "vitest";
import { registerRepo } from "./repo-registration.js";

// Mock chain: client.from('repos').upsert(...).select('id') → Promise<{ data, error }>
// await on a plain object works in JS (resolves to the object), so returning { data, error }
// from select() is sufficient.
const mockSelect = vi.fn(() => ({ data: [{ id: "new-repo-uuid" }], error: null }));
const mockUpsert = vi.fn(() => ({ select: mockSelect }));
const mockFrom = vi.fn(() => ({ upsert: mockUpsert }));
const mockClient = { from: mockFrom } as any;

describe("registerRepo", () => {
  it("upserts repo and returns repo_id", async () => {
    const repoId = await registerRepo(mockClient, {
      name: "sensei",
      remote_url: "git@github.com:org/sensei.git",
      default_branch: "main",
    });
    expect(repoId).toBe("new-repo-uuid");
  });

  it("returns null when upsert errors", async () => {
    const errClient = {
      from: vi.fn(() => ({
        upsert: vi.fn(() => ({
          select: vi.fn(() => ({ data: null, error: new Error("fail") }))
        }))
      }))
    } as any;
    const repoId = await registerRepo(errClient, { name: "x", remote_url: null });
    expect(repoId).toBeNull();
  });
});
```

- [ ] **Step 2: Run tests to confirm they fail**

```bash
cd packages/tools && bunx vitest run src/tools/repo-registration.spec.ts 2>&1 | tail -5
```

Expected: FAIL.

- [ ] **Step 3: Create `packages/tools/src/tools/repo-registration.ts`**

```typescript
import type { SupabaseClient } from "@supabase/supabase-js";

export interface RepoInfo {
  name: string;
  remote_url: string | null;
  default_branch?: string;
  description?: string;
  stack?: string[];
}

/** Upsert repo into sensei.repos. Returns the repo UUID, or null on error.
 *  Client must be created with db: { schema: "sensei" } (via makeSenseiClient). */
export async function registerRepo(
  client: SupabaseClient,
  info: RepoInfo,
): Promise<string | null> {
  const { data, error } = await client
    .from("repos")
    .upsert({
      name:           info.name,
      remote_url:     info.remote_url,
      default_branch: info.default_branch ?? null,
      description:    info.description ?? null,
      stack:          info.stack ?? null,
    }, { onConflict: "remote_url", ignoreDuplicates: false })
    .select("id");

  if (error || !data?.[0]) {
    if (error) console.error("[indexer] Supabase repo upsert error:", error.message);
    return null;
  }
  return data[0].id as string;
}
```

- [ ] **Step 4: Run tests to confirm they pass**

```bash
cd packages/tools && bunx vitest run src/tools/repo-registration.spec.ts 2>&1 | tail -5
```

Expected: 2 pass, 0 fail.

- [ ] **Step 5: Wire into reindex.ts — write config.yaml on first index**

In `packages/tools/src/tools/reindex.ts`, in the Supabase dual-write block added in Task 4, extend it to also handle first-time registration:

```typescript
This block must be placed **after** the following variables are in scope:
- `symbolMap` — built during file scanning
- `traceability` — `Record<string, string[]>`, built during traceability scan
- `existingDescription` — read from llmspec.yaml (around line 233 in reindex.ts)
- `stack` — from `detectStack(repoPath)` (the `stack.languages`/`stack.frameworks` arrays)

Place the block just before `return summary;` at the end of `reindexRepo`:

```typescript
// Dual-write symbols + docs to Supabase if config present
let supabaseConfig = await loadSenseiConfig(repoPath);

// First-time: register repo and write .sensei/config.yaml
if (!supabaseConfig) {
  const { loadCredentials } = await import("@sensei/shared");
  const creds = await loadCredentials();
  if (creds) {
    const supabaseUrl = process.env.SUPABASE_URL ?? "http://localhost:54321";
    const { createClient } = await import("@supabase/supabase-js");
    const tempClient = createClient(supabaseUrl, creds.supabase_service_key, {
      db: { schema: "sensei" },
      auth: { persistSession: false },
    });
    const { registerRepo } = await import("./repo-registration.js");
    const repoName = repoPath.split("/").at(-1) ?? "unknown";
    // stack is the result of detectStack() — use stack.languages and stack.frameworks
    const stackArray = [...(stack.languages ?? []), ...(stack.frameworks ?? [])];
    const repoId = await registerRepo(tempClient, {
      name:           repoName,
      remote_url:     null,   // TODO: detect via git remote get-url origin in a future iteration
      default_branch: null,   // TODO: detect via git rev-parse --abbrev-ref HEAD
      description:    existingDescription || undefined,
      stack:          stackArray,
    });
    if (repoId) {
      const yamlLib = await import("js-yaml");
      const configPath = senseiPath(repoPath, "config.yaml");
      await writeFile(configPath, yamlLib.default.dump({ repo_id: repoId, supabase_url: supabaseUrl }));
      supabaseConfig = { repo_id: repoId, supabase_url: supabaseUrl };
    }
  }
}

if (supabaseConfig) {
  const client = await makeSenseiClient(repoPath);
  if (client) {
    const { upsertSymbols, upsertDocs } = await import("./supabase-index-writer.js");
    // Convert Record<string, string[]> → TraceabilityEntry[] (autoDetected = false, not tracked at this level)
    const traceabilityEntries = Object.entries(traceability).map(([docPath, covers]) => ({
      docPath, covers, autoDetected: false,
    }));
    await Promise.all([
      upsertSymbols(client, supabaseConfig.repo_id, symbolMap),
      upsertDocs(client, supabaseConfig.repo_id, traceabilityEntries),
    ]);
  }
}
```

**To find the right insertion point in `reindex.ts`:** Search for `return summary;` near the bottom of `reindexRepo`. Insert the block immediately before it. At that point all the variables listed above are in scope.

- [ ] **Step 6: Run full test suite**

```bash
bun test 2>&1 | tail -5
```

Expected: all tests pass.

- [ ] **Step 7: Commit**

```bash
git add packages/tools/
git commit -m "feat(tools): register repo in Supabase and write .sensei/config.yaml on first index"
```

---

## Chunk 5: Update traceability

### Task 6: Update traceability.yaml

**Files:**
- Modify: `docs/traceability.yaml`

- [ ] **Step 1: Add code entries for new files**

In the `code:` section of `docs/traceability.yaml`, add:

```yaml
  packages/shared/src/config.ts:
    implements-design: [supabase-store]
    status: done
  packages/shared/src/supabase-client.ts:
    implements-design: [supabase-store]
    status: done
  packages/collector/src/supabase-writer.ts:
    implements-design: [supabase-store, analytics-collector]
    status: done
  packages/tools/src/tools/supabase-index-writer.ts:
    implements-design: [supabase-store, indexer]
    status: done
  packages/tools/src/tools/repo-registration.ts:
    implements-design: [supabase-store]
    status: done
```

- [ ] **Step 2: Commit**

```bash
git add docs/traceability.yaml
git commit -m "docs(traceability): register Supabase backend migration code"
```
