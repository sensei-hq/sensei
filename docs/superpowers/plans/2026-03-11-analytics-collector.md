# Analytics Collector Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the `@sensei/collector` package: a local SQLite-backed daemon that captures every Claude tool call via hooks, plus a `sensei stats` CLI command that reports usage and missed opportunities.

**Architecture:** A Node.js/Bun HTTP server on `localhost:51789` persists events to `~/.sensei/<uuid>/analytics.db`. Claude `PreToolUse`/`PostToolUse` hooks POST to the daemon with a 100ms timeout; if the daemon is unavailable, they append to a JSONL fallback file which the daemon drains on next startup. The `sensei stats` command queries SQLite directly.

**Tech Stack:** Bun, TypeScript, `bun:sqlite` (built-in), `Bun.serve`, Vitest, bash hook scripts with Bun shebang.

---

## Chunk 1: Foundation

### Task 1: Package Scaffold

Create the new `packages/collector/` package with its `package.json`, vitest config, and empty index.

**Files:**
- Create: `packages/collector/package.json`
- Create: `packages/collector/vitest.config.ts`
- Create: `packages/collector/src/index.ts`
- Create: `packages/collector/src/__stubs__/bun-globals.ts`

- [ ] **Step 1: Create `packages/collector/package.json`**

```json
{
  "name": "@sensei/collector",
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
    "@sensei/shared": "workspace:*"
  },
  "devDependencies": {
    "@types/better-sqlite3": "^7.6.0",
    "@types/node": "^22.0.0",
    "better-sqlite3": "^9.4.0",
    "typescript": "^5.5.0",
    "vitest": "^4.0.18"
  }
}
```

- [ ] **Step 2: Install dependencies**

```bash
cd packages/collector && bun install
```

Expected: `better-sqlite3` and `@types/better-sqlite3` appear in `node_modules/`.

- [ ] **Step 3: Copy the `Bun.serve` stub from `packages/server/src/__stubs__/bun-globals.ts`**

The stub lets Vitest (running under Node.js or Bun) call `Bun.serve(...)` via a real `http.createServer`:

```bash
mkdir -p packages/collector/src/__stubs__
cp packages/server/src/__stubs__/bun-globals.ts packages/collector/src/__stubs__/bun-globals.ts
```

Update the comment at the top of the copied file (line 1) to say:
```
 * Stub for Bun.serve used in Vitest test runs for @sensei/collector.
```

- [ ] **Step 4: Create `packages/collector/src/__stubs__/bun-sqlite.ts`**

This stub wraps `better-sqlite3` to match the `bun:sqlite` API used in the collector package. Using a real SQLite engine means schema tests, drain tests, and stats tests all execute actual SQL — unlike the no-op stub in `packages/server`.

```typescript
/**
 * Stub for bun:sqlite used in Vitest test runs for @sensei/collector.
 * Uses better-sqlite3 as the underlying engine so tests run real SQL.
 */
import BetterSqlite3 from "better-sqlite3";

export class Database {
  private _db: BetterSqlite3.Database;

  constructor(path: string) {
    this._db = new BetterSqlite3(path);
  }

  run(sql: string, params?: unknown[]): void {
    this._db.prepare(sql).run(...(params ?? []));
  }

  prepare(sql: string) {
    const stmt = this._db.prepare(sql);
    return {
      all: (...args: unknown[]) => stmt.all(...args),
      get: (...args: unknown[]) => stmt.get(...args),
      run: (...args: unknown[]) => stmt.run(...args),
    };
  }

  close(): void {
    this._db.close();
  }
}
```

- [ ] **Step 5: Create `packages/collector/vitest.config.ts`**

Alias both `bun:sqlite` (to the real-SQL stub) and set up `Bun.serve` via the globals stub. This matches the pattern in `packages/server/vitest.config.ts`.

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

- [ ] **Step 6: Create `packages/collector/src/index.ts`** (empty barrel, filled in later)

```typescript
export {};
```

- [ ] **Step 7: Verify Vitest finds the package**

```bash
cd packages/collector && bunx vitest run
```

Expected: `No test files found` (no test files yet — that's correct, zero failures).

- [ ] **Step 8: Commit**

```bash
git add packages/collector/
git commit -m "feat(collector): scaffold @sensei/collector package"
```

---

### Task 2: UUID Module

Read or generate the anonymous user UUID stored at `~/.sensei/uuid`.

**Files:**
- Create: `packages/collector/src/uuid.ts`
- Create: `packages/collector/src/uuid.spec.ts`

- [ ] **Step 1: Write the failing test**

Create `packages/collector/src/uuid.spec.ts`:

```typescript
import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { mkdirSync, rmSync, existsSync, readFileSync } from "fs";
import { join } from "path";
import { tmpdir } from "os";
import { readOrCreateUuid } from "./uuid.js";

const TMP = join(tmpdir(), `sensei-uuid-test-${Date.now()}`);

beforeEach(() => mkdirSync(TMP, { recursive: true }));
afterEach(() => rmSync(TMP, { recursive: true, force: true }));

describe("readOrCreateUuid", () => {
  it("creates a UUID file on first call", async () => {
    const uuidPath = join(TMP, "uuid");
    const id = await readOrCreateUuid(uuidPath);
    expect(id).toMatch(/^[0-9a-f-]{36}$/);
    expect(existsSync(uuidPath)).toBe(true);
    expect(readFileSync(uuidPath, "utf8").trim()).toBe(id);
  });

  it("returns the same UUID on repeated calls", async () => {
    const uuidPath = join(TMP, "uuid");
    const a = await readOrCreateUuid(uuidPath);
    const b = await readOrCreateUuid(uuidPath);
    expect(a).toBe(b);
  });

  it("does not overwrite an existing UUID", async () => {
    const uuidPath = join(TMP, "uuid");
    // Pre-seed with a known UUID
    const { writeFileSync } = await import("fs");
    writeFileSync(uuidPath, "preset-uuid-value");
    const id = await readOrCreateUuid(uuidPath);
    expect(id).toBe("preset-uuid-value");
  });
});
```

- [ ] **Step 2: Run test — verify it fails**

```bash
cd packages/collector && bunx vitest run src/uuid.spec.ts
```

Expected: FAIL with `Cannot find module './uuid.js'`

- [ ] **Step 3: Implement `uuid.ts`**

```typescript
import { readFileSync, writeFileSync, existsSync, mkdirSync } from "fs";
import { dirname } from "path";
import { randomUUID } from "crypto";

export async function readOrCreateUuid(uuidPath: string): Promise<string> {
  if (existsSync(uuidPath)) {
    return readFileSync(uuidPath, "utf8").trim();
  }
  mkdirSync(dirname(uuidPath), { recursive: true });
  const id = randomUUID();
  writeFileSync(uuidPath, id, "utf8");
  return id;
}
```

- [ ] **Step 4: Run test — verify it passes**

```bash
cd packages/collector && bunx vitest run src/uuid.spec.ts
```

Expected: PASS (3 tests)

- [ ] **Step 5: Export from index.ts**

```typescript
export { readOrCreateUuid } from "./uuid.js";
```

- [ ] **Step 6: Commit**

```bash
git add packages/collector/src/uuid.ts packages/collector/src/uuid.spec.ts packages/collector/src/index.ts
git commit -m "feat(collector): add UUID read/generate module"
```

---

### Task 3: Schema Module

SQLite migrations — create the `events`, `projects`, and `daily_stats` tables.

**Files:**
- Create: `packages/collector/src/schema.ts`
- Create: `packages/collector/src/schema.spec.ts`

- [ ] **Step 1: Write the failing test**

Create `packages/collector/src/schema.spec.ts`:

```typescript
import { describe, it, expect, afterEach } from "vitest";
import { Database } from "bun:sqlite";
import { mkdirSync, rmSync } from "fs";
import { join } from "path";
import { tmpdir } from "os";
import { createTables } from "./schema.js";

const TMP = join(tmpdir(), `sensei-schema-test-${Date.now()}`);

afterEach(() => rmSync(TMP, { recursive: true, force: true }));

describe("createTables", () => {
  it("creates events table with correct columns including seq", () => {
    const db = new Database(":memory:");
    createTables(db);

    const cols = db
      .prepare("PRAGMA table_info(events)")
      .all() as Array<{ name: string; type: string; notnull: number }>;

    const names = cols.map(c => c.name);
    expect(names).toContain("id");
    expect(names).toContain("user_uuid");
    expect(names).toContain("session_id");
    expect(names).toContain("seq");
    expect(names).toContain("ts");
    expect(names).toContain("tool");
    expect(names).toContain("phase");
    expect(names).toContain("duration_ms");
    expect(names).toContain("success");
    expect(names).toContain("input");
    expect(names).toContain("error");
    expect(names).toContain("project_path");
  });

  it("creates projects table", () => {
    const db = new Database(":memory:");
    createTables(db);
    const cols = db.prepare("PRAGMA table_info(projects)").all() as Array<{ name: string }>;
    const names = cols.map(c => c.name);
    expect(names).toContain("path");
    expect(names).toContain("first_seen");
    expect(names).toContain("last_seen");
  });

  it("creates daily_stats table", () => {
    const db = new Database(":memory:");
    createTables(db);
    const cols = db.prepare("PRAGMA table_info(daily_stats)").all() as Array<{ name: string }>;
    const names = cols.map(c => c.name);
    expect(names).toContain("date");
    expect(names).toContain("tool");
    expect(names).toContain("calls");
    expect(names).toContain("successes");
    expect(names).toContain("total_duration_ms");
  });

  it("is idempotent — runs twice without error", () => {
    const db = new Database(":memory:");
    createTables(db);
    expect(() => createTables(db)).not.toThrow();
  });

  it("enables WAL mode on a real file database", () => {
    mkdirSync(TMP, { recursive: true });
    const db = new Database(join(TMP, "test.db"));
    createTables(db);
    const row = db.prepare("PRAGMA journal_mode").get() as { journal_mode: string };
    expect(row.journal_mode).toBe("wal");
    db.close();
  });
});
```

- [ ] **Step 2: Run test — verify it fails**

```bash
cd packages/collector && bunx vitest run src/schema.spec.ts
```

Expected: FAIL with `Cannot find module './schema.js'`

- [ ] **Step 3: Implement `schema.ts`**

```typescript
import { Database } from "bun:sqlite";

export function createTables(db: Database): void {
  db.run("PRAGMA journal_mode=WAL");

  db.run(`
    CREATE TABLE IF NOT EXISTS events (
      id          INTEGER PRIMARY KEY AUTOINCREMENT,
      user_uuid   TEXT    NOT NULL,
      session_id  TEXT    NOT NULL DEFAULT '',
      seq         INTEGER,
      ts          INTEGER NOT NULL,
      tool        TEXT    NOT NULL,
      phase       TEXT    NOT NULL CHECK(phase IN ('pre', 'post')),
      duration_ms INTEGER,
      success     INTEGER,
      input       TEXT,
      error       TEXT,
      project_path TEXT   NOT NULL DEFAULT ''
    )
  `);

  db.run(`
    CREATE TABLE IF NOT EXISTS projects (
      path       TEXT PRIMARY KEY,
      first_seen INTEGER NOT NULL,
      last_seen  INTEGER NOT NULL
    )
  `);

  db.run(`
    CREATE TABLE IF NOT EXISTS daily_stats (
      date             TEXT NOT NULL,
      tool             TEXT NOT NULL,
      calls            INTEGER NOT NULL DEFAULT 0,
      successes        INTEGER NOT NULL DEFAULT 0,
      total_duration_ms INTEGER NOT NULL DEFAULT 0,
      PRIMARY KEY (date, tool)
    )
  `);

  db.run("CREATE INDEX IF NOT EXISTS idx_events_session ON events(session_id)");
  db.run("CREATE INDEX IF NOT EXISTS idx_events_tool ON events(tool)");
  db.run("CREATE INDEX IF NOT EXISTS idx_events_ts ON events(ts)");
}
```

- [ ] **Step 4: Run test — verify it passes**

```bash
cd packages/collector && bunx vitest run src/schema.spec.ts
```

Expected: PASS (5 tests)

- [ ] **Step 5: Export from index.ts**

```typescript
export { readOrCreateUuid } from "./uuid.js";
export { createTables } from "./schema.js";
```

- [ ] **Step 6: Commit**

```bash
git add packages/collector/src/schema.ts packages/collector/src/schema.spec.ts packages/collector/src/index.ts
git commit -m "feat(collector): add SQLite schema migrations"
```

---

## Chunk 2: Daemon

### Task 4: Daemon Core — HTTP Server + Event Ingestion

The daemon accepts `POST /event` and `GET /health`. Events are validated and written to SQLite.

**Files:**
- Create: `packages/collector/src/daemon.ts`
- Create: `packages/collector/src/daemon.spec.ts`

- [ ] **Step 1: Write the failing tests**

Create `packages/collector/src/daemon.spec.ts`:

```typescript
import { describe, it, expect, afterAll } from "vitest";
import { Database } from "bun:sqlite";
import { join } from "path";
import { tmpdir } from "os";
import { createTables } from "./schema.js";
import { startDaemon } from "./daemon.js";

// Use a unique port per test file to avoid conflicts
const PORT = 51800;

describe("startDaemon", () => {
  let db: Database;
  let daemon: { stop: () => void; port: number };

  afterAll(() => daemon?.stop());

  it("starts and responds to GET /health", async () => {
    db = new Database(":memory:");
    createTables(db);
    daemon = await startDaemon(PORT, { db });

    const res = await fetch(`http://localhost:${PORT}/health`);
    const body = await res.json() as Record<string, unknown>;
    expect(res.status).toBe(200);
    expect(body.ok).toBe(true);
    expect(typeof body.uptime).toBe("number");
  });

  it("POST /event stores a valid pre event in SQLite", async () => {
    const event = {
      user_uuid: "test-uuid",
      session_id: "sess-1",
      ts: Date.now(),
      tool: "Bash",
      phase: "pre",
      project_path: "/projects/foo",
      input: JSON.stringify({ command: "ls" }).slice(0, 2048),
    };

    const res = await fetch(`http://localhost:${PORT}/event`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(event),
    });
    const body = await res.json() as Record<string, unknown>;
    expect(res.status).toBe(200);
    expect(body.ok).toBe(true);

    const row = db.prepare("SELECT * FROM events WHERE session_id = ?").get("sess-1") as Record<string, unknown> | null;
    expect(row).not.toBeNull();
    expect(row!.tool).toBe("Bash");
    expect(row!.phase).toBe("pre");
  });

  it("POST /event stores a post event with success and error", async () => {
    const event = {
      user_uuid: "test-uuid",
      session_id: "sess-2",
      ts: Date.now(),
      tool: "search_index",
      phase: "post",
      success: true,
      project_path: "/projects/foo",
    };

    await fetch(`http://localhost:${PORT}/event`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(event),
    });

    const row = db.prepare("SELECT * FROM events WHERE session_id = ?").get("sess-2") as Record<string, unknown> | null;
    expect(row!.success).toBe(1);
    expect(row!.error).toBeNull();
  });

  it("POST /event returns 400 for missing required fields", async () => {
    const res = await fetch(`http://localhost:${PORT}/event`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ tool: "Bash" }), // missing phase and ts
    });
    const body = await res.json() as Record<string, unknown>;
    expect(res.status).toBe(400);
    expect(body.ok).toBe(false);
    expect(typeof body.error).toBe("string");
  });

  it("POST /event rejects invalid JSON", async () => {
    const res = await fetch(`http://localhost:${PORT}/event`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: "not json",
    });
    const body = await res.json() as Record<string, unknown>;
    expect(res.status).toBe(400);
    expect(body.ok).toBe(false);
    expect(typeof body.error).toBe("string");
  });

  it("returns 404 for unknown routes", async () => {
    const res = await fetch(`http://localhost:${PORT}/unknown`);
    expect(res.status).toBe(404);
  });
});
```

- [ ] **Step 2: Run test — verify it fails**

```bash
cd packages/collector && bunx vitest run src/daemon.spec.ts
```

Expected: FAIL with `Cannot find module './daemon.js'`

- [ ] **Step 3: Implement `daemon.ts`**

```typescript
import { Database } from "bun:sqlite";
import { mkdirSync } from "fs";
import { dirname } from "path";
import { createTables } from "./schema.js";

export interface DaemonOptions {
  db?: Database;          // injectable for tests; if omitted, opened from dbPath
  dbPath?: string;        // ignored when db is provided
}

export interface Daemon {
  stop: () => void;
  port: number;
}

interface EventPayload {
  user_uuid?: string;
  session_id?: string;
  seq?: number | null;
  ts: number;
  tool: string;
  phase: "pre" | "post";
  duration_ms?: number | null;
  success?: boolean | null;
  input?: string | null;
  error?: string | null;
  project_path?: string;
}

function isValidEvent(body: unknown): body is EventPayload {
  if (!body || typeof body !== "object") return false;
  const b = body as Record<string, unknown>;
  return (
    typeof b.ts === "number" &&
    typeof b.tool === "string" && b.tool.length > 0 &&
    (b.phase === "pre" || b.phase === "post")
  );
}

function insertEvent(db: Database, e: EventPayload): void {
  db.prepare(`
    INSERT INTO events (user_uuid, session_id, seq, ts, tool, phase, duration_ms, success, input, error, project_path)
    VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
  `).run(
    e.user_uuid ?? "",
    e.session_id ?? "",
    e.seq ?? null,
    e.ts,
    e.tool,
    e.phase,
    e.duration_ms ?? null,
    e.success == null ? null : (e.success ? 1 : 0),
    e.input?.slice(0, 2048) ?? null,
    e.error ?? null,
    e.project_path ?? "",
  );

  // Upsert into projects
  const now = Date.now();
  db.prepare(`
    INSERT INTO projects (path, first_seen, last_seen) VALUES (?, ?, ?)
    ON CONFLICT(path) DO UPDATE SET last_seen = excluded.last_seen
  `).run(e.project_path ?? "", now, now);
}

export async function startDaemon(port: number, opts: DaemonOptions = {}): Promise<Daemon> {
  const db = opts.db ?? (() => {
    mkdirSync(dirname(opts.dbPath!), { recursive: true });
    return new Database(opts.dbPath!);
  })();

  createTables(db);

  const startedAt = Date.now();

  const server = Bun.serve({
    port,
    async fetch(req: Request): Promise<Response> {
      const url = new URL(req.url);

      if (req.method === "GET" && url.pathname === "/health") {
        return Response.json({ ok: true, uptime: Date.now() - startedAt });
      }

      if (req.method === "POST" && url.pathname === "/event") {
        let body: unknown;
        try {
          body = await req.json();
        } catch {
          return Response.json({ ok: false, error: "invalid JSON" }, { status: 400 });
        }
        if (!isValidEvent(body)) {
          return Response.json({ ok: false, error: "invalid payload: ts, tool, and phase are required" }, { status: 400 });
        }
        try {
          insertEvent(db, body);
          return Response.json({ ok: true });
        } catch (err) {
          console.error("[collector] SQLite write error:", (err as Error).message);
          return Response.json({ ok: false, error: "write failed" }, { status: 500 });
        }
      }

      return Response.json({ ok: false, error: "not found" }, { status: 404 });
    },
  });

  return { stop: () => server.stop(), port: server.port };
}
```

- [ ] **Step 4: Run test — verify it passes**

```bash
cd packages/collector && bunx vitest run src/daemon.spec.ts
```

Expected: PASS (6 tests)

- [ ] **Step 5: Export from index.ts**

```typescript
export { readOrCreateUuid } from "./uuid.js";
export { createTables } from "./schema.js";
export { startDaemon } from "./daemon.js";
export type { DaemonOptions, Daemon } from "./daemon.js";
```

- [ ] **Step 6: Commit**

```bash
git add packages/collector/src/daemon.ts packages/collector/src/daemon.spec.ts packages/collector/src/index.ts
git commit -m "feat(collector): add HTTP daemon with POST /event and GET /health"
```

---

### Task 5: JSONL Drain

On startup, the daemon drains any buffered events from `events.jsonl` into SQLite, then deletes the file.

**Files:**
- Create: `packages/collector/src/drain.ts`
- Create: `packages/collector/src/drain.spec.ts`
- Modify: `packages/collector/src/daemon.ts` (call `drainJsonl` during init)

- [ ] **Step 1: Write the failing tests**

Create `packages/collector/src/drain.spec.ts`:

```typescript
import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { Database } from "bun:sqlite";
import { mkdirSync, writeFileSync, existsSync, rmSync } from "fs";
import { join } from "path";
import { tmpdir } from "os";
import { createTables } from "./schema.js";
import { drainJsonl } from "./drain.js";

const TMP = join(tmpdir(), `sensei-drain-test-${Date.now()}`);
const JSONL_PATH = join(TMP, "events.jsonl");

beforeEach(() => mkdirSync(TMP, { recursive: true }));
afterEach(() => rmSync(TMP, { recursive: true, force: true }));

function makeEvent(overrides: Record<string, unknown> = {}) {
  return JSON.stringify({
    user_uuid: "u1",
    session_id: "s1",
    ts: Date.now(),
    tool: "Bash",
    phase: "pre",
    project_path: "/proj",
    ...overrides,
  });
}

describe("drainJsonl", () => {
  it("imports events from JSONL into SQLite and deletes the file", async () => {
    const db = new Database(":memory:");
    createTables(db);

    writeFileSync(JSONL_PATH, [makeEvent(), makeEvent({ tool: "Read" })].join("\n") + "\n");

    await drainJsonl(db, JSONL_PATH);

    const rows = db.prepare("SELECT tool FROM events ORDER BY id").all() as Array<{ tool: string }>;
    expect(rows).toHaveLength(2);
    expect(rows[0].tool).toBe("Bash");
    expect(rows[1].tool).toBe("Read");
    expect(existsSync(JSONL_PATH)).toBe(false);
  });

  it("is a no-op when the JSONL file does not exist", async () => {
    const db = new Database(":memory:");
    createTables(db);

    await expect(drainJsonl(db, JSONL_PATH)).resolves.not.toThrow();

    const count = (db.prepare("SELECT COUNT(*) as n FROM events").get() as { n: number }).n;
    expect(count).toBe(0);
  });

  it("skips malformed JSON lines and still deletes the file", async () => {
    const db = new Database(":memory:");
    createTables(db);

    // JSON parse failures are benign — skipped, valid lines still imported
    const lines = [
      makeEvent({ tool: "Read" }),
      "not valid json",
      makeEvent({ tool: "Glob" }),
    ].join("\n") + "\n";
    writeFileSync(JSONL_PATH, lines);

    await drainJsonl(db, JSONL_PATH);

    // File is deleted because the parse errors are non-fatal (not SQLite errors)
    expect(existsSync(JSONL_PATH)).toBe(false);

    const rows = db.prepare("SELECT tool FROM events ORDER BY id").all() as Array<{ tool: string }>;
    expect(rows).toHaveLength(2);
    expect(rows.map(r => r.tool)).toEqual(["Read", "Glob"]);
  });

  it("leaves JSONL intact if a SQLite transaction fails", async () => {
    // Simulate a SQLite failure by passing a db that has no events table
    const brokenDb = new Database(":memory:");
    // intentionally do NOT call createTables — events table does not exist

    writeFileSync(JSONL_PATH, makeEvent() + "\n");

    await drainJsonl(brokenDb, JSONL_PATH);

    // File must remain intact because the SQLite insert failed
    expect(existsSync(JSONL_PATH)).toBe(true);
  });

  it("skips blank lines silently", async () => {
    const db = new Database(":memory:");
    createTables(db);

    writeFileSync(JSONL_PATH, `\n${makeEvent()}\n\n${makeEvent({ tool: "Write" })}\n`);

    await drainJsonl(db, JSONL_PATH);

    const count = (db.prepare("SELECT COUNT(*) as n FROM events").get() as { n: number }).n;
    expect(count).toBe(2);
  });
});
```

- [ ] **Step 2: Run test — verify it fails**

```bash
cd packages/collector && bunx vitest run src/drain.spec.ts
```

Expected: FAIL with `Cannot find module './drain.js'`

- [ ] **Step 3: Implement `drain.ts`**

```typescript
import { Database } from "bun:sqlite";
import { existsSync, readFileSync, unlinkSync } from "fs";

interface StoredEvent {
  user_uuid?: string;
  session_id?: string;
  seq?: number | null;
  ts?: number;
  tool?: string;
  phase?: string;
  duration_ms?: number | null;
  success?: boolean | null;
  input?: string | null;
  error?: string | null;
  project_path?: string;
}

function insertRaw(db: Database, e: StoredEvent): void {
  if (!e.ts || !e.tool || (e.phase !== "pre" && e.phase !== "post")) return;

  db.prepare(`
    INSERT INTO events (user_uuid, session_id, seq, ts, tool, phase, duration_ms, success, input, error, project_path)
    VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
  `).run(
    e.user_uuid ?? "",
    e.session_id ?? "",
    e.seq ?? null,
    e.ts,
    e.tool,
    e.phase,
    e.duration_ms ?? null,
    e.success == null ? null : (e.success ? 1 : 0),
    e.input?.slice(0, 2048) ?? null,
    e.error ?? null,
    e.project_path ?? "",
  );
}

export async function drainJsonl(db: Database, jsonlPath: string): Promise<void> {
  if (!existsSync(jsonlPath)) return;

  const content = readFileSync(jsonlPath, "utf8");
  const lines = content.split("\n").filter(l => l.trim().length > 0);

  // Parse JSON first — parse errors are benign skips, not failures
  const validEvents: StoredEvent[] = [];
  for (const line of lines) {
    try {
      validEvents.push(JSON.parse(line) as StoredEvent);
    } catch {
      console.warn("[collector] drain: skipping malformed JSONL line");
    }
  }

  // Insert in a transaction — if any SQLite write fails, rollback and leave file intact
  try {
    db.run("BEGIN");
    for (const event of validEvents) {
      insertRaw(db, event);
    }
    db.run("COMMIT");
  } catch (err) {
    try { db.run("ROLLBACK"); } catch {}
    console.error("[collector] drain: SQLite write failed, leaving JSONL intact:", (err as Error).message);
    return; // File stays intact — caller can retry on next startup
  }

  // All events inserted — delete the JSONL file
  try {
    unlinkSync(jsonlPath);
  } catch (err) {
    console.error("[collector] drain: failed to delete JSONL file:", (err as Error).message);
  }
}
```

- [ ] **Step 4: Run test — verify it passes**

```bash
cd packages/collector && bunx vitest run src/drain.spec.ts
```

Expected: PASS (5 tests)

- [ ] **Step 5: Wire drain into daemon startup**

In `packages/collector/src/daemon.ts`, add `drainJsonl` call before starting the HTTP server. Add `jsonlPath` to `DaemonOptions`. Replace the `startDaemon` function body's init section:

```typescript
// Add to imports:
import { drainJsonl } from "./drain.js";

// Add jsonlPath to DaemonOptions:
export interface DaemonOptions {
  db?: Database;
  dbPath?: string;
  jsonlPath?: string;  // path to JSONL fallback file; drained on startup
}

// In startDaemon, after createTables(db) and before Bun.serve:
if (opts.jsonlPath) {
  await drainJsonl(db, opts.jsonlPath);
}
```

- [ ] **Step 6: Run all collector tests**

```bash
cd packages/collector && bunx vitest run
```

Expected: PASS (all existing tests still pass)

- [ ] **Step 7: Export drain from index.ts**

```typescript
export { drainJsonl } from "./drain.js";
```

- [ ] **Step 8: Commit**

```bash
git add packages/collector/src/drain.ts packages/collector/src/drain.spec.ts packages/collector/src/daemon.ts packages/collector/src/index.ts
git commit -m "feat(collector): drain JSONL fallback into SQLite on daemon startup"
```

---

## Chunk 3: Analytics

### Task 6: Stats Query Engine

Query the events SQLite database for all `sensei stats` display modes.

**Files:**
- Create: `packages/collector/src/stats.ts`
- Create: `packages/collector/src/stats.spec.ts`

- [ ] **Step 1: Write the failing tests**

Create `packages/collector/src/stats.spec.ts`:

```typescript
import { describe, it, expect } from "vitest";
import { Database } from "bun:sqlite";
import { createTables } from "./schema.js";
import { queryStats } from "./stats.js";

function seedDb(db: Database) {
  const insert = db.prepare(`
    INSERT INTO events (user_uuid, session_id, seq, ts, tool, phase, duration_ms, success, input, error, project_path)
    VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
  `);

  const now = Date.now();
  const day = 24 * 60 * 60 * 1000;
  const recent = now - 3 * day; // 3 days ago — within 7-day window
  const old    = now - 10 * day; // 10 days ago — outside 7-day window

  // Recent events (within 7 days)
  insert.run("u1", "sess-a", 1, recent, "search_index", "post", 120, 1, null, null, "/proj");
  insert.run("u1", "sess-a", 2, recent + 1000, "search_index", "post", 200, 1, null, null, "/proj");
  insert.run("u1", "sess-a", 3, recent + 2000, "Bash", "post", 300, 0, null, "exit 1", "/proj");
  insert.run("u1", "sess-b", 1, recent + 3000, "Read", "post", 50, 1, null, null, "/proj");

  // Old events (outside 7 days)
  insert.run("u1", "sess-c", 1, old, "search_index", "post", 100, 1, null, null, "/proj");
}

describe("queryStats", () => {
  it("default 7-day summary counts only recent events", () => {
    const db = new Database(":memory:");
    createTables(db);
    seedDb(db);

    const result = queryStats(db, {});
    expect(result.total_calls).toBe(4); // 3 recent post events + 1 Read post
    expect(result.tools.some(t => t.name === "search_index")).toBe(true);
    expect(result.sessions).toBe(2); // sess-a, sess-b
    expect(result.period.from).toBeDefined();
  });

  it("--all includes events older than 7 days", () => {
    const db = new Database(":memory:");
    createTables(db);
    seedDb(db);

    const result = queryStats(db, { all: true });
    expect(result.total_calls).toBe(5); // includes the old one
  });

  it("--tool returns stats for one tool only", () => {
    const db = new Database(":memory:");
    createTables(db);
    seedDb(db);

    const result = queryStats(db, { tool: "search_index" });
    expect(result.tool).toBeDefined();
    expect(result.tool!.name).toBe("search_index");
    expect(result.tool!.calls).toBe(2); // 2 recent search_index events
    expect(result.tool!.success_rate).toBe(1.0);
    expect(result.tool!.avg_duration_ms).toBe(160); // (120+200)/2
    expect(result.tool!.last_called).toBeTypeOf("number");
  });

  it("--session returns chronological events for that session", () => {
    const db = new Database(":memory:");
    createTables(db);
    seedDb(db);

    const result = queryStats(db, { session: "sess-a" });
    expect(result.events).toBeDefined();
    expect(result.events!.length).toBe(3);
    expect(result.events![0].tool).toBe("search_index");
  });

  it("--since excludes events before the given date", () => {
    const db = new Database(":memory:");
    createTables(db);
    seedDb(db);

    const yesterday = new Date(Date.now() - 24 * 60 * 60 * 1000).toISOString().slice(0, 10);
    const result = queryStats(db, { since: yesterday });
    // All seed events are 3+ days ago, so nothing falls within "since yesterday"
    expect(result.total_calls).toBe(0);
  });

  it("--since includes events on or after the given date", () => {
    const db = new Database(":memory:");
    createTables(db);
    // Seed one event right now (not via seedDb which uses old timestamps)
    db.prepare(`INSERT INTO events (user_uuid, session_id, seq, ts, tool, phase, duration_ms, success, input, error, project_path)
      VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)`)
      .run("u1", "now-sess", 1, Date.now(), "Read", "post", 50, 1, null, null, "/proj");

    const today = new Date().toISOString().slice(0, 10);
    const result = queryStats(db, { since: today });
    expect(result.total_calls).toBe(1);
  });

  it("--session returns events for sessions older than 7 days (no date filter)", () => {
    const db = new Database(":memory:");
    createTables(db);
    // seed an event 30 days ago — outside any default window
    const thirtyDaysAgo = Date.now() - 30 * 24 * 60 * 60 * 1000;
    db.prepare(`INSERT INTO events (user_uuid, session_id, seq, ts, tool, phase, duration_ms, success, input, error, project_path)
      VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)`)
      .run("u1", "old-sess", 1, thirtyDaysAgo, "Read", "post", 40, 1, null, null, "/proj");

    const result = queryStats(db, { session: "old-sess" });
    expect(result.events).toBeDefined();
    expect(result.events!.length).toBe(1); // must not be filtered out by 7-day default
  });

  it("--session period.from reflects the earliest event timestamp", () => {
    const db = new Database(":memory:");
    createTables(db);
    seedDb(db);

    const result = queryStats(db, { session: "sess-a" });
    expect(result.events).toBeDefined();
    expect(result.period.from).toMatch(/^\d{4}-\d{2}-\d{2}$/); // YYYY-MM-DD
    // The from date must be derived from the event timestamp, not empty string
    expect(result.period.from).not.toBe("");
  });

  it("--since combined with --tool filters by both date and tool", () => {
    const db = new Database(":memory:");
    createTables(db);
    seedDb(db); // has search_index events 3 days ago and one 10 days ago

    // Filter: only search_index events from the last 5 days
    const fiveDaysAgo = new Date(Date.now() - 5 * 24 * 60 * 60 * 1000).toISOString().slice(0, 10);
    const result = queryStats(db, { tool: "search_index", since: fiveDaysAgo });
    // Should find 2 recent search_index events but NOT the old one
    expect(result.tool).toBeDefined();
    expect(result.tool!.calls).toBe(2);
  });

  it("tools list is sorted by call count descending", () => {
    const db = new Database(":memory:");
    createTables(db);
    seedDb(db);

    const result = queryStats(db, {});
    for (let i = 1; i < result.tools.length; i++) {
      expect(result.tools[i - 1].calls).toBeGreaterThanOrEqual(result.tools[i].calls);
    }
  });
});
```

- [ ] **Step 2: Run test — verify it fails**

```bash
cd packages/collector && bunx vitest run src/stats.spec.ts
```

Expected: FAIL with `Cannot find module './stats.js'`

- [ ] **Step 3: Implement `stats.ts`**

```typescript
import { Database } from "bun:sqlite";

export interface ToolStat {
  name: string;
  calls: number;
  success_rate: number;
  avg_duration_ms: number;
  last_called?: number;
}

export interface StatsResult {
  period: { from: string; to: string };
  total_calls: number;
  tools: ToolStat[];
  sessions: number;
  projects: number;
  // Populated only for specific flag combinations:
  tool?: ToolStat;
  events?: Array<Record<string, unknown>>;
}

export interface StatsOptions {
  all?: boolean;
  tool?: string;
  session?: string;
  since?: string;   // YYYY-MM-DD
}

/**
 * Build composable WHERE conditions.
 * --since can combine with --tool or --session per the spec.
 * --session alone has no date filter (returns all events for that session).
 */
function buildConditions(opts: StatsOptions): { conditions: string[]; params: unknown[] } {
  const conditions: string[] = [];
  const params: unknown[] = [];

  if (opts.session) {
    conditions.push("session_id = ?");
    params.push(opts.session);
  }

  if (opts.since) {
    conditions.push("ts >= ?");
    params.push(new Date(opts.since).getTime());
  } else if (!opts.all && !opts.session) {
    // Default: last 7 days — NOT applied when --session is active,
    // because --session must return all events for that session regardless of age.
    conditions.push("ts >= ?");
    params.push(Date.now() - 7 * 24 * 60 * 60 * 1000);
  }

  if (opts.tool) {
    conditions.push("tool = ?");
    params.push(opts.tool);
  }

  return { conditions, params };
}

export function queryStats(db: Database, opts: StatsOptions): StatsResult {
  const now = Date.now();
  const toDate = new Date(now).toISOString().slice(0, 10);

  const { conditions, params } = buildConditions(opts);
  const baseWhere = conditions.length > 0 ? conditions.join(" AND ") : "1";

  // For session mode, return the chronological event list
  if (opts.session) {
    const events = db
      .prepare(`SELECT * FROM events WHERE ${baseWhere} ORDER BY ts ASC`)
      .all(...params) as Array<Record<string, unknown>>;
    const firstTs = (events[0]?.ts as number | undefined) ?? now;
    return {
      period: { from: new Date(firstTs).toISOString().slice(0, 10), to: toDate },
      total_calls: events.length,
      tools: [],
      sessions: 1,
      projects: 0,
      events,
    };
  }

  const postWhere = `${baseWhere} AND phase = 'post'`;

  const totalRow = db
    .prepare(`SELECT COUNT(*) as n FROM events WHERE ${postWhere}`)
    .get(...params) as { n: number };
  const total_calls = totalRow.n;

  const toolRows = db.prepare(`
    SELECT
      tool,
      COUNT(*) as calls,
      AVG(CASE WHEN success = 1 THEN 1.0 ELSE 0.0 END) as success_rate,
      AVG(CASE WHEN duration_ms IS NOT NULL THEN duration_ms ELSE NULL END) as avg_duration_ms,
      MAX(ts) as last_called
    FROM events
    WHERE ${postWhere}
    GROUP BY tool
    ORDER BY calls DESC
  `).all(...params) as Array<{
    tool: string;
    calls: number;
    success_rate: number;
    avg_duration_ms: number;
    last_called: number;
  }>;

  const tools: ToolStat[] = toolRows.map(r => ({
    name: r.tool,
    calls: r.calls,
    success_rate: Math.round((r.success_rate ?? 0) * 100) / 100,
    avg_duration_ms: Math.round(r.avg_duration_ms ?? 0),
    last_called: r.last_called,
  }));

  const sessionsRow = db
    .prepare(`SELECT COUNT(DISTINCT session_id) as n FROM events WHERE ${postWhere}`)
    .get(...params) as { n: number };

  const projectsRow = db
    .prepare(`SELECT COUNT(DISTINCT project_path) as n FROM events WHERE ${postWhere}`)
    .get(...params) as { n: number };

  const fromTs = opts.all
    ? (db.prepare("SELECT MIN(ts) as t FROM events").get() as { t: number | null })?.t ?? now
    : opts.since
      ? new Date(opts.since).getTime()
      : now - 7 * 24 * 60 * 60 * 1000;

  const result: StatsResult = {
    period: { from: new Date(fromTs).toISOString().slice(0, 10), to: toDate },
    total_calls,
    tools,
    sessions: sessionsRow.n,
    projects: projectsRow.n,
  };

  if (opts.tool && tools.length > 0) {
    result.tool = tools[0];
  }

  return result;
}
```

- [ ] **Step 4: Run test — verify it passes**

```bash
cd packages/collector && bunx vitest run src/stats.spec.ts
```

Expected: PASS (10 tests)

- [ ] **Step 5: Export from index.ts**

Add:
```typescript
export { queryStats } from "./stats.js";
export type { StatsResult, StatsOptions, ToolStat } from "./stats.js";
```

- [ ] **Step 6: Commit**

```bash
git add packages/collector/src/stats.ts packages/collector/src/stats.spec.ts packages/collector/src/index.ts
git commit -m "feat(collector): add stats query engine with --all, --tool, --session, --since support"
```

---

### Task 7: Gaps Detector

Detect bash commands in the events DB that sensei tools could have replaced.

**Files:**
- Create: `packages/collector/src/gaps.ts`
- Create: `packages/collector/src/gaps.spec.ts`

- [ ] **Step 1: Write the failing tests**

Create `packages/collector/src/gaps.spec.ts`:

```typescript
import { describe, it, expect } from "vitest";
import { detectGapPatterns, BASH_TO_TOOL_PATTERNS } from "./gaps.js";

describe("detectGapPatterns", () => {
  it("maps grep to search_index", () => {
    const commands = [
      "grep -r 'auth' src/",
      "grep -r 'token' src/",
      "grep -r 'auth' src/",
    ];
    const gaps = detectGapPatterns(commands);
    const grepGap = gaps.find(g => g.pattern.includes("grep"));
    expect(grepGap).toBeDefined();
    expect(grepGap!.count).toBe(3);
    expect(grepGap!.suggested_tool).toContain("search_index");
  });

  it("maps find to Glob", () => {
    const commands = ["find . -name '*.ts'", "find . -name '*.json'"];
    const gaps = detectGapPatterns(commands);
    const findGap = gaps.find(g => g.pattern.includes("find"));
    expect(findGap).toBeDefined();
    expect(findGap!.suggested_tool).toContain("Glob");
  });

  it("maps cat to Read", () => {
    const commands = ["cat src/index.ts", "cat package.json"];
    const gaps = detectGapPatterns(commands);
    const catGap = gaps.find(g => g.pattern.includes("cat"));
    expect(catGap).toBeDefined();
    expect(catGap!.suggested_tool).toContain("Read");
  });

  it("returns results sorted by count descending", () => {
    const commands = [
      "grep -r 'x' src/",
      "grep -r 'y' src/",
      "grep -r 'z' src/",
      "cat file.ts",
    ];
    const gaps = detectGapPatterns(commands);
    for (let i = 1; i < gaps.length; i++) {
      expect(gaps[i - 1].count).toBeGreaterThanOrEqual(gaps[i].count);
    }
  });

  it("ignores commands that have no sensei equivalent", () => {
    const commands = ["npm install", "git status", "echo hello"];
    const gaps = detectGapPatterns(commands);
    // None of these map to a sensei tool
    expect(gaps).toHaveLength(0);
  });

  it("returns empty array for empty input", () => {
    expect(detectGapPatterns([])).toHaveLength(0);
  });

  it("maps rg to search_index", () => {
    const commands = ["rg 'auth' src/", "rg 'token'"];
    const gaps = detectGapPatterns(commands);
    const rgGap = gaps.find(g => g.suggested_tool.includes("search_index"));
    expect(rgGap).toBeDefined();
    expect(rgGap!.count).toBe(2);
  });

  it("maps sed to Edit", () => {
    const commands = ["sed -i 's/foo/bar/' file.ts"];
    const gaps = detectGapPatterns(commands);
    const sedGap = gaps.find(g => g.suggested_tool.includes("Edit"));
    expect(sedGap).toBeDefined();
  });

  it("maps curl to WebFetch", () => {
    const commands = ["curl https://example.com/api"];
    const gaps = detectGapPatterns(commands);
    const curlGap = gaps.find(g => g.suggested_tool.includes("WebFetch"));
    expect(curlGap).toBeDefined();
  });

  it("exports BASH_TO_TOOL_PATTERNS with at least 4 entries", () => {
    expect(Object.keys(BASH_TO_TOOL_PATTERNS).length).toBeGreaterThanOrEqual(4);
  });
});
```

- [ ] **Step 2: Run test — verify it fails**

```bash
cd packages/collector && bunx vitest run src/gaps.spec.ts
```

Expected: FAIL with `Cannot find module './gaps.js'`

- [ ] **Step 3: Implement `gaps.ts`**

```typescript
export interface GapEntry {
  pattern: string;
  count: number;
  suggested_tool: string;
}

/**
 * Maps bash command prefix patterns to the sensei tool that could replace them.
 * Keys are regex strings matched against the start of bash command strings.
 */
export const BASH_TO_TOOL_PATTERNS: Record<string, string> = {
  "^grep\\s":       "search_index (semantic or full-text)",
  "^rg\\s":         "search_index (semantic or full-text)",
  "^find\\s":       "Glob",
  "^ls\\s|^ls$":    "Glob or LS",
  "^cat\\s":        "Read (or get_file_context)",
  "^head\\s":       "Read",
  "^tail\\s":       "Read",
  "^sed\\s":        "Edit",
  "^awk\\s":        "Edit or Bash (if transforming data)",
  "^curl\\s|^wget\\s": "WebFetch",
};

/**
 * Given a list of raw bash command strings, return gap entries
 * sorted by frequency descending.
 */
export function detectGapPatterns(commands: string[]): GapEntry[] {
  // Count matches per pattern
  const counts = new Map<string, { count: number; suggested_tool: string; pattern: string }>();

  for (const [regexStr, tool] of Object.entries(BASH_TO_TOOL_PATTERNS)) {
    const re = new RegExp(regexStr, "i");
    const matched = commands.filter(c => re.test(c.trim()));
    if (matched.length > 0) {
      const display = regexStr.replace(/^\^/, "").replace(/\\s.*$/, " ...");
      counts.set(regexStr, { count: matched.length, suggested_tool: tool, pattern: display });
    }
  }

  return Array.from(counts.values()).sort((a, b) => b.count - a.count);
}
```

- [ ] **Step 4: Run test — verify it passes**

```bash
cd packages/collector && bunx vitest run src/gaps.spec.ts
```

Expected: PASS (10 tests)

- [ ] **Step 5: Export from index.ts**

Add:
```typescript
export { detectGapPatterns, BASH_TO_TOOL_PATTERNS } from "./gaps.js";
export type { GapEntry } from "./gaps.js";
```

- [ ] **Step 6: Commit**

```bash
git add packages/collector/src/gaps.ts packages/collector/src/gaps.spec.ts packages/collector/src/index.ts
git commit -m "feat(collector): add bash→sensei gap detector"
```

---

### Task 8: Hook Installer

Write hook scripts to `~/.claude/hooks/` and register them in `~/.claude/settings.json`. Also write the launchd plist for macOS.

**Files:**
- Create: `packages/collector/src/install.ts`
- Create: `packages/collector/src/install.spec.ts`

- [ ] **Step 1: Write the failing tests**

Create `packages/collector/src/install.spec.ts`:

```typescript
import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { mkdirSync, rmSync, existsSync, readFileSync, statSync } from "fs";
import { join } from "path";
import { tmpdir } from "os";
import { installHooks } from "./install.js";

const TMP = join(tmpdir(), `sensei-install-test-${Date.now()}`);
const HOOKS_DIR = join(TMP, "hooks");
const SETTINGS_PATH = join(TMP, "settings.json");
const UUID_PATH = join(TMP, "uuid");
const LAUNCHD_DIR = join(TMP, "LaunchAgents");

beforeEach(() => {
  mkdirSync(TMP, { recursive: true });
  mkdirSync(HOOKS_DIR, { recursive: true });
});

afterEach(() => rmSync(TMP, { recursive: true, force: true }));

describe("installHooks", () => {
  it("creates pre-tool-use and post-tool-use hook scripts", async () => {
    await installHooks({ hooksDir: HOOKS_DIR, settingsPath: SETTINGS_PATH, uuidPath: UUID_PATH, launchdDir: LAUNCHD_DIR });

    expect(existsSync(join(HOOKS_DIR, "sensei-pre-tool-use.ts"))).toBe(true);
    expect(existsSync(join(HOOKS_DIR, "sensei-post-tool-use.ts"))).toBe(true);
  });

  it("hook scripts are executable", async () => {
    await installHooks({ hooksDir: HOOKS_DIR, settingsPath: SETTINGS_PATH, uuidPath: UUID_PATH, launchdDir: LAUNCHD_DIR });

    const mode = statSync(join(HOOKS_DIR, "sensei-pre-tool-use.ts")).mode;
    expect(mode & 0o111).toBeTruthy(); // at least one execute bit set
  });

  it("hook scripts start with a bun shebang", async () => {
    await installHooks({ hooksDir: HOOKS_DIR, settingsPath: SETTINGS_PATH, uuidPath: UUID_PATH, launchdDir: LAUNCHD_DIR });

    const preContent = readFileSync(join(HOOKS_DIR, "sensei-pre-tool-use.ts"), "utf8");
    const postContent = readFileSync(join(HOOKS_DIR, "sensei-post-tool-use.ts"), "utf8");
    expect(preContent.startsWith("#!/usr/bin/env bun")).toBe(true);
    expect(postContent.startsWith("#!/usr/bin/env bun")).toBe(true);
  });

  it("registers hooks in settings.json (creating it if absent)", async () => {
    await installHooks({ hooksDir: HOOKS_DIR, settingsPath: SETTINGS_PATH, uuidPath: UUID_PATH, launchdDir: LAUNCHD_DIR });

    const settings = JSON.parse(readFileSync(SETTINGS_PATH, "utf8")) as Record<string, unknown>;
    const hooks = settings.hooks as Record<string, unknown>;
    expect(hooks).toBeDefined();
    expect(hooks.PreToolUse).toBeDefined();
    expect(hooks.PostToolUse).toBeDefined();
  });

  it("merges into existing settings.json without overwriting other keys", async () => {
    const { writeFileSync } = await import("fs");
    writeFileSync(SETTINGS_PATH, JSON.stringify({ theme: "dark", hooks: {} }));

    await installHooks({ hooksDir: HOOKS_DIR, settingsPath: SETTINGS_PATH, uuidPath: UUID_PATH, launchdDir: LAUNCHD_DIR });

    const settings = JSON.parse(readFileSync(SETTINGS_PATH, "utf8")) as Record<string, unknown>;
    expect(settings.theme).toBe("dark");
  });

  it("creates launchd plist on macOS path", async () => {
    await installHooks({ hooksDir: HOOKS_DIR, settingsPath: SETTINGS_PATH, uuidPath: UUID_PATH, launchdDir: LAUNCHD_DIR });
    expect(existsSync(join(LAUNCHD_DIR, "com.sensei.collector.plist"))).toBe(true);
    const plist = readFileSync(join(LAUNCHD_DIR, "com.sensei.collector.plist"), "utf8");
    expect(plist).toContain("com.sensei.collector");
    expect(plist).toContain("51789");
  });
});
```

- [ ] **Step 2: Run test — verify it fails**

```bash
cd packages/collector && bunx vitest run src/install.spec.ts
```

Expected: FAIL with `Cannot find module './install.js'`

- [ ] **Step 3: Implement `install.ts`**

```typescript
import { readFileSync, writeFileSync, existsSync, mkdirSync, chmodSync } from "fs";
import { join, dirname } from "path";
import { homedir } from "os";

export interface InstallOptions {
  hooksDir?: string;
  settingsPath?: string;
  uuidPath?: string;
  launchdDir?: string;
}

function preHookContent(uuidPath: string): string {
  return `#!/usr/bin/env bun
import { readFileSync, existsSync, mkdirSync, appendFileSync } from "fs";
import { homedir } from "os";
import { join, dirname } from "path";

const HOME = homedir();
const uuidPath = ${JSON.stringify(uuidPath)};
const uuid = existsSync(uuidPath) ? readFileSync(uuidPath, "utf8").trim() : "unknown";

let input = "";
const decoder = new TextDecoder();
for await (const chunk of Bun.stdin.stream()) {
  input += decoder.decode(chunk);
}

let parsed: Record<string, unknown> = {};
try { parsed = JSON.parse(input); } catch {}

const event = {
  user_uuid: uuid,
  phase: "pre",
  tool: (parsed.tool_name ?? "unknown") as string,
  ts: Date.now(),
  session_id: process.env.CLAUDE_SESSION_ID ?? "",
  project_path: process.env.CLAUDE_PROJECT_DIR ?? process.cwd(),
  input: input.slice(0, 2048),
};

const body = JSON.stringify(event);
try {
  const res = await fetch("http://localhost:51789/event", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body,
    signal: AbortSignal.timeout(100),
  });
  if (!res.ok) throw new Error("http " + res.status);
} catch {
  const dir = join(HOME, ".sensei", uuid);
  if (!existsSync(dir)) mkdirSync(dir, { recursive: true });
  appendFileSync(join(dir, "events.jsonl"), body + "\\n");
}
`;
}

function postHookContent(uuidPath: string): string {
  return `#!/usr/bin/env bun
import { readFileSync, existsSync, mkdirSync, appendFileSync } from "fs";
import { homedir } from "os";
import { join } from "path";

const HOME = homedir();
const uuidPath = ${JSON.stringify(uuidPath)};
const uuid = existsSync(uuidPath) ? readFileSync(uuidPath, "utf8").trim() : "unknown";

let input = "";
const decoder = new TextDecoder();
for await (const chunk of Bun.stdin.stream()) {
  input += decoder.decode(chunk);
}

let parsed: Record<string, unknown> = {};
try { parsed = JSON.parse(input); } catch {}

const success = parsed.exit_code == null || parsed.exit_code === 0;
// Note: Claude's PostToolUse hook payload does not include duration_ms.
// The field is left as null (schema allows null) since the hook cannot measure it.
const event = {
  user_uuid: uuid,
  phase: "post",
  tool: (parsed.tool_name ?? "unknown") as string,
  ts: Date.now(),
  session_id: process.env.CLAUDE_SESSION_ID ?? "",
  project_path: process.env.CLAUDE_PROJECT_DIR ?? process.cwd(),
  success,
  duration_ms: null,  // not available from hook payload
  error: success ? null : String(parsed.tool_result ?? ""),
};

const body = JSON.stringify(event);
try {
  const res = await fetch("http://localhost:51789/event", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body,
    signal: AbortSignal.timeout(100),
  });
  if (!res.ok) throw new Error("http " + res.status);
} catch {
  const dir = join(HOME, ".sensei", uuid);
  if (!existsSync(dir)) mkdirSync(dir, { recursive: true });
  appendFileSync(join(dir, "events.jsonl"), body + "\\n");
}
`;
}

function launchdPlist(bunPath: string, daemonScript: string): string {
  return `<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>com.sensei.collector</string>
  <key>ProgramArguments</key>
  <array>
    <string>${bunPath}</string>
    <string>${daemonScript}</string>
  </array>
  <key>RunAtLoad</key>
  <true/>
  <key>KeepAlive</key>
  <true/>
  <key>EnvironmentVariables</key>
  <dict>
    <key>SENSEI_COLLECTOR_PORT</key>
    <string>51789</string>
  </dict>
  <key>StandardOutPath</key>
  <string>${homedir()}/.sensei/collector.log</string>
  <key>StandardErrorPath</key>
  <string>${homedir()}/.sensei/collector.err</string>
</dict>
</plist>`;
}

export async function installHooks(opts: InstallOptions = {}): Promise<void> {
  const HOME = homedir();
  const hooksDir = opts.hooksDir ?? join(HOME, ".claude", "hooks");
  const settingsPath = opts.settingsPath ?? join(HOME, ".claude", "settings.json");
  const uuidPath = opts.uuidPath ?? join(HOME, ".sensei", "uuid");
  const launchdDir = opts.launchdDir ?? join(HOME, "Library", "LaunchAgents");

  mkdirSync(hooksDir, { recursive: true });
  mkdirSync(launchdDir, { recursive: true });

  // Write hook scripts
  const prePath = join(hooksDir, "sensei-pre-tool-use.ts");
  const postPath = join(hooksDir, "sensei-post-tool-use.ts");

  writeFileSync(prePath, preHookContent(uuidPath), "utf8");
  writeFileSync(postPath, postHookContent(uuidPath), "utf8");
  chmodSync(prePath, 0o755);
  chmodSync(postPath, 0o755);

  // Update settings.json
  let settings: Record<string, unknown> = {};
  if (existsSync(settingsPath)) {
    try {
      settings = JSON.parse(readFileSync(settingsPath, "utf8")) as Record<string, unknown>;
    } catch {}
  }
  const hooks = (settings.hooks as Record<string, unknown>) ?? {};
  hooks.PreToolUse = prePath;
  hooks.PostToolUse = postPath;
  settings.hooks = hooks;
  mkdirSync(dirname(settingsPath), { recursive: true });
  writeFileSync(settingsPath, JSON.stringify(settings, null, 2), "utf8");

  // Write launchd plist
  const bunPath = process.execPath; // path to bun binary
  // The daemon entry point — will be a standalone script generated by sensei
  const daemonScript = join(HOME, ".sensei", "collector-daemon.ts");
  const plistPath = join(launchdDir, "com.sensei.collector.plist");
  writeFileSync(plistPath, launchdPlist(bunPath, daemonScript), "utf8");
}
```

- [ ] **Step 4: Run test — verify it passes**

```bash
cd packages/collector && bunx vitest run src/install.spec.ts
```

Expected: PASS (6 tests)

- [ ] **Step 5: Export from index.ts**

Add:
```typescript
export { installHooks } from "./install.js";
export type { InstallOptions } from "./install.js";
```

- [ ] **Step 6: Run all collector tests**

```bash
cd packages/collector && bunx vitest run
```

Expected: PASS (all tests)

- [ ] **Step 7: Commit**

```bash
git add packages/collector/src/install.ts packages/collector/src/install.spec.ts packages/collector/src/index.ts
git commit -m "feat(collector): add hook installer (scripts + settings.json + launchd plist)"
```

---

## Chunk 4: CLI Integration

### Task 9: `sensei stats` Command

Wire `@sensei/collector` into the CLI. Add `sensei stats` with all flags.

**Files:**
- Create: `packages/cli/src/commands/stats.ts`
- Modify: `packages/cli/src/cli.ts`
- Modify: `packages/cli/package.json`

- [ ] **Step 1: Add `@sensei/collector` dependency to CLI**

In `packages/cli/package.json`, add to `"dependencies"`:

```json
"@sensei/collector": "workspace:*"
```

- [ ] **Step 1b: Add `bun:sqlite` alias to CLI vitest.config**

Check whether `packages/cli/vitest.config.ts` already has the `bun:sqlite` alias. If not, add it the same way as in `packages/collector/vitest.config.ts`:

```typescript
resolve: {
  alias: {
    "bun:sqlite": new URL("./src/__stubs__/bun-sqlite.ts", import.meta.url).pathname,
  },
},
```

Also copy (or symlink) `src/__stubs__/bun-sqlite.ts` and `src/__stubs__/bun-globals.ts` from `packages/collector/src/__stubs__/` into `packages/cli/src/__stubs__/` — or add the stub files there directly (same content).

> Note: The CLI stats test imports `Database` from `bun:sqlite` and calls `createTables` which also uses `bun:sqlite`. Without this alias, Vitest in the CLI package will fail to resolve `bun:sqlite`.

- [ ] **Step 2: Write the failing test for the stats command**

Create `packages/cli/src/commands/stats.spec.ts`:

```typescript
import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { Database } from "bun:sqlite";
import { mkdirSync, rmSync, writeFileSync, existsSync } from "fs";
import { join, dirname } from "path";
import { tmpdir } from "os";
import { createTables, queryStats } from "@sensei/collector";
import { formatStats, stats } from "./stats.js";

const TMP = join(tmpdir(), `sensei-stats-cmd-test-${Date.now()}`);

beforeEach(() => mkdirSync(TMP, { recursive: true }));
afterEach(() => rmSync(TMP, { recursive: true, force: true }));

function seedDb(db: Database) {
  const insert = db.prepare(`
    INSERT INTO events (user_uuid, session_id, seq, ts, tool, phase, duration_ms, success, input, error, project_path)
    VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
  `);
  const recent = Date.now() - 1000;
  insert.run("u1", "s1", 1, recent, "search_index", "post", 150, 1, null, null, "/proj");
  insert.run("u1", "s1", 2, recent + 100, "Bash", "post", 300, 0, null, "error", "/proj");
}

describe("formatStats", () => {
  it("default text output includes tool names and total calls", () => {
    const db = new Database(":memory:");
    createTables(db);
    seedDb(db);

    const result = queryStats(db, {});
    const text = formatStats(result, { json: false });
    expect(text).toContain("search_index");
    expect(text).toContain("Bash");
    expect(text).toContain("2"); // total calls
  });

  it("--json output is valid JSON with expected keys", () => {
    const db = new Database(":memory:");
    createTables(db);
    seedDb(db);

    const result = queryStats(db, {});
    const text = formatStats(result, { json: true });
    const parsed = JSON.parse(text) as Record<string, unknown>;
    expect(parsed.total_calls).toBeDefined();
    expect(Array.isArray(parsed.tools)).toBe(true);
    expect(parsed.period).toBeDefined();
  });

  it("tool-specific output includes tool name and success_rate", () => {
    const db = new Database(":memory:");
    createTables(db);
    seedDb(db);

    const result = queryStats(db, { tool: "search_index" });
    const text = formatStats(result, { json: false });
    expect(text).toContain("search_index");
    expect(text).toMatch(/100%|1\.0/);
  });
});

describe("stats() DB path construction", () => {
  it("opens analytics.db at ~/.sensei/<uuid>/analytics.db", async () => {
    const home = join(TMP, "home");
    const uuidVal = "test-uuid-abc";
    mkdirSync(join(home, ".sensei"), { recursive: true });
    writeFileSync(join(home, ".sensei", "uuid"), uuidVal);

    // Capture console.log output
    const output: string[] = [];
    const origLog = console.log;
    console.log = (...args: unknown[]) => output.push(args.join(" "));
    try {
      await stats({ _home: home });
    } finally {
      console.log = origLog;
    }

    // The DB file must have been created at the expected path
    const dbPath = join(home, ".sensei", uuidVal, "analytics.db");
    expect(existsSync(dbPath)).toBe(true);
  });
});
```

- [ ] **Step 3: Run test — verify it fails**

```bash
cd packages/cli && bunx vitest run src/commands/stats.spec.ts
```

Expected: FAIL with `Cannot find module './stats.js'`

- [ ] **Step 4: Implement `packages/cli/src/commands/stats.ts`**

```typescript
import { Database } from "bun:sqlite";
import { existsSync, mkdirSync } from "fs";
import { join } from "path";
import { homedir } from "os";
import { readOrCreateUuid, createTables, queryStats, detectGapPatterns } from "@sensei/collector";
import type { StatsResult } from "@sensei/collector";

export interface StatsCommandOptions {
  all?: boolean;
  tool?: string;
  session?: string;
  since?: string;
  json?: boolean;
  gaps?: boolean;
  /** Override home directory — used in tests to avoid touching ~/.sensei */
  _home?: string;
}

export function formatStats(result: StatsResult, opts: { json: boolean }): string {
  if (opts.json) {
    return JSON.stringify(result, null, 2);
  }

  const lines: string[] = [];

  if (result.tool) {
    // Single-tool view
    const t = result.tool;
    lines.push(`\nsensei stats — ${t.name}`);
    lines.push(`  Calls:       ${t.calls}`);
    lines.push(`  Success:     ${Math.round(t.success_rate * 100)}%`);
    lines.push(`  Avg latency: ${t.avg_duration_ms}ms`);
    if (t.last_called) {
      lines.push(`  Last called: ${new Date(t.last_called).toISOString()}`);
    }
    return lines.join("\n");
  }

  if (result.events) {
    // Session view
    lines.push(`\nSession events (${result.events.length}):`);
    for (const e of result.events) {
      const ts = new Date((e.ts as number) ?? 0).toISOString().slice(11, 19);
      lines.push(`  [${ts}] ${e.phase} ${e.tool}${e.success === 0 ? " ✗" : ""}`);
    }
    return lines.join("\n");
  }

  // Default summary
  lines.push(`\nsensei stats — ${result.period.from} → ${result.period.to}`);
  lines.push(`\nTool calls: ${result.total_calls}`);

  const top5 = result.tools.slice(0, 5);
  for (const t of top5) {
    const pct = result.total_calls > 0
      ? Math.round((t.calls / result.total_calls) * 100)
      : 0;
    const successStr = `✓ ${Math.round(t.success_rate * 100)}%`;
    const durStr = t.avg_duration_ms >= 1000
      ? `${(t.avg_duration_ms / 1000).toFixed(1)}s`
      : `${t.avg_duration_ms}ms`;
    lines.push(`  ${t.name.padEnd(20)} ${String(t.calls).padStart(4)}  (${String(pct).padStart(2)}%)  ${successStr}  avg ${durStr}`);
  }
  if (result.tools.length > 5) lines.push(`  (top 5 by call count)`);
  lines.push(`\nSessions: ${result.sessions} across ${result.projects} project${result.projects !== 1 ? "s" : ""}`);

  return lines.join("\n");
}

export async function stats(opts: StatsCommandOptions): Promise<void> {
  const HOME = opts._home ?? homedir();
  const uuidPath = join(HOME, ".sensei", "uuid");
  const uuid = await readOrCreateUuid(uuidPath);
  const dbPath = join(HOME, ".sensei", uuid, "analytics.db");

  if (!existsSync(dbPath)) {
    const parent = join(HOME, ".sensei", uuid);
    mkdirSync(parent, { recursive: true });
  }

  const db = new Database(dbPath);
  createTables(db);

  if (opts.gaps) {
    // Query all Bash commands in the period, honouring --since / --all
    let gapsWhere = "tool = 'Bash' AND phase = 'post' AND input IS NOT NULL";
    const gapsParams: unknown[] = [];
    if (opts.since) {
      gapsWhere += " AND ts >= ?";
      gapsParams.push(new Date(opts.since).getTime());
    } else if (!opts.all) {
      gapsWhere += " AND ts >= ?";
      gapsParams.push(Date.now() - 7 * 24 * 60 * 60 * 1000);
    }
    const bashEvents = db.prepare(`
      SELECT input FROM events WHERE ${gapsWhere}
    `).all(...gapsParams) as Array<{ input: string }>;

    const commands = bashEvents
      .map(e => {
        try {
          const parsed = JSON.parse(e.input) as { command?: string };
          return parsed.command ?? "";
        } catch {
          return e.input;
        }
      })
      .filter(Boolean);

    const gaps = detectGapPatterns(commands);

    if (opts.json) {
      console.log(JSON.stringify({ gaps }, null, 2));
    } else {
      const period = opts.all ? "all time" : "last 7 days";
      console.log(`\nMissed opportunity report — ${period}\n`);
      if (gaps.length === 0) {
        console.log("  No gaps detected.");
        return;
      }
      const header = "Pattern                          Count   Suggested tool";
      const sep    = "─".repeat(header.length);
      console.log(header);
      console.log(sep);
      for (const g of gaps) {
        console.log(`${g.pattern.padEnd(32)} ${String(g.count).padStart(5)}   ${g.suggested_tool}`);
      }
    }
    return;
  }

  const result = queryStats(db, {
    all: opts.all,
    tool: opts.tool,
    session: opts.session,
    since: opts.since,
  });

  console.log(formatStats(result, { json: opts.json ?? false }));
}
```

- [ ] **Step 5: Run test — verify it passes**

```bash
cd packages/cli && bunx vitest run src/commands/stats.spec.ts
```

Expected: PASS (4 tests)

- [ ] **Step 6: Add `stats` to `cli.ts`**

In `packages/cli/src/cli.ts`, add CLI flags to the `parseArgs` options:

```typescript
// Add these flags to the options object in parseArgs:
tool: { type: "string" },
session: { type: "string" },
since: { type: "string" },
all: { type: "boolean", default: false },
json: { type: "boolean", default: false },
gaps: { type: "boolean", default: false },
```

Add the `stats` case to the switch statement:

```typescript
case "stats": {
  const { stats } = await import("./commands/stats.js");
  await stats({
    all: values.all,
    tool: values.tool,
    session: values.session,
    since: values.since,
    json: values.json,
    gaps: values.gaps,
  });
  break;
}
```

Add to the HELP string (under the Commands section):

```
  stats                    Show tool usage analytics (last 7 days)
```

And to the options section:
```
stats:
  --all                    Show all-time data instead of last 7 days
  --tool <name>            Show stats for a specific tool
  --session <id>           Show all events for a session
  --since <YYYY-MM-DD>     Show events on or after this date
  --json                   Output as JSON
  --gaps                   Show missed-opportunity report (bash vs sensei tools)
```

- [ ] **Step 7: Run all CLI tests**

```bash
cd packages/cli && bunx vitest run
```

Expected: PASS (no regressions)

- [ ] **Step 8: Commit**

```bash
git add packages/cli/src/commands/stats.ts packages/cli/src/commands/stats.spec.ts packages/cli/src/cli.ts packages/cli/package.json
git commit -m "feat(cli): add sensei stats command with --all, --tool, --session, --since, --json, --gaps"
```

---

### Task 10: Update `sensei setup` for Hook Installation + Traceability

Wire hook installation into `sensei setup` and mark analytics items done in traceability.

**Files:**
- Modify: `packages/cli/src/commands/setup.ts`
- Modify: `packages/cli/src/cli.ts` (add `--hooks` flag)
- Modify: `docs/traceability.yaml`

- [ ] **Step 1: Add `--hooks` flag to cli.ts**

In the `parseArgs` options:
```typescript
hooks: { type: "boolean", default: false },
```

Update the `setup` case in the switch:

```typescript
case "setup": {
  if (values.hooks) {
    const { setupHooks } = await import("./commands/setup.js");
    await setupHooks();
    break;
  }
  if (!values.mcp) {
    console.error("Usage: sensei setup --mcp | sensei setup --hooks");
    process.exit(1);
  }
  // ... existing MCP setup code ...
}
```

Add to HELP:
```
  setup --hooks            Install Claude hook scripts and register daemon autostart
```

- [ ] **Step 2: Add `setupHooks` to `packages/cli/src/commands/setup.ts`**

```typescript
import { installHooks } from "@sensei/collector";
import { intro, outro, log } from "@clack/prompts";

export async function setupHooks(): Promise<void> {
  intro("sensei setup --hooks");
  await installHooks();
  log.success("Hook scripts installed to ~/.claude/hooks/");
  log.success("hooks.PreToolUse and hooks.PostToolUse registered in ~/.claude/settings.json");
  log.info("Daemon autostart registered (launchd on macOS / systemd on Linux)");
  log.info("Run: launchctl load ~/Library/LaunchAgents/com.sensei.collector.plist");
  outro("Done. Claude tool calls will now be tracked in ~/.sensei/<uuid>/analytics.db");
}
```

- [ ] **Step 3: Run all CLI tests**

```bash
cd packages/cli && bunx vitest run
```

Expected: PASS

- [ ] **Step 4: Run full test suite from repo root**

```bash
cd /path/to/repo && bunx vitest run
```

Expected: PASS (all packages)

- [ ] **Step 5: Update `docs/traceability.yaml` with code entries**

In the `code:` section at the bottom of `docs/traceability.yaml`, add:

```yaml
  packages/collector/src/uuid.ts:
    implements-design: [analytics-collector]
    status: done
  packages/collector/src/schema.ts:
    implements-design: [analytics-collector]
    status: done
  packages/collector/src/daemon.ts:
    implements-design: [analytics-collector]
    status: done
  packages/collector/src/drain.ts:
    implements-design: [analytics-collector]
    status: done
  packages/collector/src/stats.ts:
    implements-design: [analytics-collector]
    status: done
  packages/collector/src/gaps.ts:
    implements-design: [analytics-collector]
    status: done
  packages/collector/src/install.ts:
    implements-design: [analytics-collector]
    status: done
  packages/cli/src/commands/stats.ts:
    implements-design: [analytics-collector]
    status: done
  packages/cli/src/commands/setup.ts:
    implements-design: [analytics-collector, cli]
    status: done
```

- [ ] **Step 6: Commit everything**

```bash
git add packages/cli/src/commands/setup.ts packages/cli/src/cli.ts docs/traceability.yaml
git commit -m "feat(cli): add sensei setup --hooks command and update traceability"
```

---

## Final Verification

- [ ] Run the complete test suite from repo root:

```bash
bunx vitest run
```

Expected: All tests pass, zero failures.

- [ ] Smoke-test the stats command manually (requires the collector package built):

```bash
sensei stats --help
sensei stats
sensei stats --json
sensei stats --gaps
```
