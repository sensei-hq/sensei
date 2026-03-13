# Phase 1: Foundation Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `sensei init` on a TypeScript repo indexes symbols into Supabase, the dashboard at `localhost:3000` shows the repo with file + symbol counts, and an MCP-enabled agent can call `get_session_context`, `search`, and `load_context` and get real results.

**Architecture:** A new `packages/engine` package owns the Scan → Parse → Index pipeline. `packages/shared` exposes Supabase types and client. `packages/server` gains a stdio MCP server alongside the existing HTTP report server. `packages/cli` `init` command is updated to call the engine and write `.sensei/config.yaml`, `CLAUDE.md`, and `AGENTS.md`. The dashboard gets a Repo list and Symbol browser reading from Supabase.

**Tech Stack:** Bun, TypeScript, Supabase (PostgreSQL), ts-morph (TypeScript AST), `@modelcontextprotocol/sdk`, SvelteKit, Vitest, fast-glob

---

## Pre-flight: Verify local Supabase is running

Before writing any code, make sure the local Supabase instance is available for tests and development.

- [ ] Run `supabase start` from the repo root. Confirm it outputs `API URL: http://localhost:54321`.
- [ ] Note the `service_role key` from the output — you'll use it in tests and `.sensei/config.yaml`.

---

## Chunk 1: Schema and Shared Types

### Task 1: Supabase Migration — Phase 1 Tables

**Files:**
- Create: `supabase/migrations/20260313000000_phase1_foundation.sql`

These tables power the engine index. They live in the `sensei` schema (already exposed via `supabase/config.toml`).

- [ ] **Step 1: Write the migration**

Create `supabase/migrations/20260313000000_phase1_foundation.sql`:

```sql
-- Phase 1 Foundation: repos, symbols, call_edges, imports, scan_state

create extension if not exists "uuid-ossp";

create schema if not exists sensei;

-- Repos: one row per indexed repository
create table if not exists sensei.repos (
  id               uuid primary key default gen_random_uuid(),
  name             text not null,
  local_path       text not null unique,
  remote_url       text,
  stack            text[] not null default '{}',
  entry_points     jsonb not null default '[]',
  last_indexed_at  timestamptz,
  created_at       timestamptz not null default now()
);

-- Symbols: exported functions, classes, types, interfaces from parsed files
create table if not exists sensei.symbols (
  id          uuid primary key default gen_random_uuid(),
  repo_id     uuid not null references sensei.repos(id) on delete cascade,
  file_path   text not null,
  name        text not null,
  kind        text not null check(kind in ('function','class','type','interface','enum','const','method','component','hook','unknown')),
  signature   text,
  docstring   text,
  line_start  integer not null,
  line_end    integer not null,
  is_exported boolean not null default false,
  updated_at  timestamptz not null default now(),
  unique(repo_id, file_path, name, kind)
);

create index if not exists idx_symbols_repo_file on sensei.symbols(repo_id, file_path);
create index if not exists idx_symbols_name on sensei.symbols(name);

-- Call edges: which symbol calls which other symbol
create table if not exists sensei.call_edges (
  id            uuid primary key default gen_random_uuid(),
  repo_id       uuid not null references sensei.repos(id) on delete cascade,
  caller_id     uuid not null references sensei.symbols(id) on delete cascade,
  callee_name   text not null,
  callee_file   text,
  created_at    timestamptz not null default now()
);

create index if not exists idx_call_edges_caller on sensei.call_edges(caller_id);

-- Imports: module-level import relationships
create table if not exists sensei.imports (
  id           uuid primary key default gen_random_uuid(),
  repo_id      uuid not null references sensei.repos(id) on delete cascade,
  source_file  text not null,
  target_path  text not null,   -- resolved or raw specifier
  names        text[] not null, -- imported names (empty = namespace import)
  created_at   timestamptz not null default now(),
  unique(repo_id, source_file, target_path)  -- required for upsert ON CONFLICT
);

create index if not exists idx_imports_source on sensei.imports(repo_id, source_file);

-- Scan state: tracks file fingerprints for incremental indexing
create table if not exists sensei.scan_state (
  repo_id     uuid not null references sensei.repos(id) on delete cascade,
  file_path   text not null,
  mtime       bigint not null,   -- milliseconds epoch
  content_hash text not null,   -- sha256 hex
  indexed_at  timestamptz not null default now(),
  primary key (repo_id, file_path)
);

-- Events: telemetry from the collector daemon (collector already writes here)
create table if not exists sensei.events (
  id           bigserial primary key,
  user_uuid    text not null,
  session_id   text,
  repo_id      uuid references sensei.repos(id),
  phase        text not null check(phase in ('pre','post')),
  tool         text not null,
  project_path text not null default '',
  input        jsonb,
  ts           timestamptz not null,
  seq          integer,
  duration_ms  integer,
  success      boolean,
  error        text
);

create index if not exists idx_events_session on sensei.events(session_id);
create index if not exists idx_events_ts on sensei.events(ts desc);

-- NOTE: The `embeddings` table (pgvector) is intentionally deferred to Phase 2.
-- Phase 1 `search` uses ilike substring matching. Phase 2 adds semantic search via pgvector.

-- Grant PostgREST roles access
grant usage on schema sensei to anon, authenticated, service_role;
grant all on all tables in schema sensei to anon, authenticated, service_role;
grant all on all sequences in schema sensei to anon, authenticated, service_role;
alter default privileges in schema sensei grant all on tables to anon, authenticated, service_role;
alter default privileges in schema sensei grant all on sequences to anon, authenticated, service_role;
```

- [ ] **Step 2: Apply the migration to the local Supabase instance**

```bash
supabase db reset
```

> `supabase db reset` replays all migrations against the local Docker instance. Use this — NOT `supabase db push` (which pushes to a remote linked project and will error if none is linked).

Expected: Migration applied successfully with no errors, tables re-created.

- [ ] **Step 3: Verify tables exist**

```bash
psql "postgresql://postgres:postgres@localhost:54322/postgres" -c "\dt sensei.*"
```

Expected: `repos`, `symbols`, `call_edges`, `imports`, `scan_state`, `events` tables in the `sensei` schema.

- [ ] **Step 4: Commit**

```bash
git add supabase/migrations/20260313000000_phase1_foundation.sql
git commit -m "feat(schema): add Phase 1 foundation tables — repos, symbols, call_edges, imports, scan_state"
```

---

### Task 2: Shared Types — New Domain Entities

The existing `packages/shared/src/types.ts` has old types (`LlmSpec`, `FileAnalysis`, etc.) — do not remove them, existing code depends on them. Add the new domain types in a separate file.

**Files:**
- Create: `packages/shared/src/domain.ts`
- Modify: `packages/shared/src/index.ts`

- [ ] **Step 1: Create domain.ts with new types**

Create `packages/shared/src/domain.ts`:

```typescript
// ─── Phase 1 domain types ─────────────────────────────────────────────────────

export interface Repo {
  id: string;
  name: string;
  local_path: string;
  remote_url: string | null;
  stack: string[];
  entry_points: EntryPoint[];
  last_indexed_at: string | null;
  created_at: string;
}

export interface EntryPoint {
  path: string;
  role: string;
}

export type SymbolKind =
  | "function" | "class" | "type" | "interface"
  | "enum" | "const" | "method" | "component" | "hook" | "unknown";

export interface RepoSymbol {
  id: string;
  repo_id: string;
  file_path: string;
  name: string;
  kind: SymbolKind;
  signature: string | null;
  docstring: string | null;
  line_start: number;
  line_end: number;
  is_exported: boolean;
  updated_at: string;
}

export interface CallEdge {
  id: string;
  repo_id: string;
  caller_id: string;
  callee_name: string;
  callee_file: string | null;
}

export interface Import {
  id: string;
  repo_id: string;
  source_file: string;
  target_path: string;
  names: string[];
}

export interface FileEntry {
  path: string;       // repo-relative path
  absPath: string;    // absolute path on disk
  mtime: number;      // milliseconds epoch
  hash: string;       // sha256 hex of file contents
  size: number;       // bytes
}

export interface ScanResult {
  repoId: string;
  files: FileEntry[];
  changed: string[];   // repo-relative paths changed since last scan
  deleted: string[];   // repo-relative paths removed since last scan
}

export interface ParsedSymbol {
  name: string;
  kind: SymbolKind;
  signature: string | null;
  docstring: string | null;
  lineStart: number;
  lineEnd: number;
  isExported: boolean;
}

export interface ParsedEdge {
  callerName: string;
  calleeName: string;
  calleeFile: string | null;
}

export interface ParsedImport {
  targetPath: string;
  names: string[];
}

export interface ParsedFile {
  filePath: string;    // repo-relative
  language: string;
  symbols: ParsedSymbol[];
  edges: ParsedEdge[];
  imports: ParsedImport[];
}

export interface IndexResult {
  repoId: string;
  symbolsUpserted: number;
  edgesUpserted: number;
  importsUpserted: number;
  filesIndexed: number;
  filesDeleted: number;
  durationMs: number;
  errors: string[];
}
```

- [ ] **Step 2: Export domain types from index.ts**

In `packages/shared/src/index.ts`, add this export line (at the end, after existing exports):

```typescript
export * from "./domain.js";
```

- [ ] **Step 3: Write unit tests**

Create `packages/shared/src/domain.spec.ts`:

```typescript
import { describe, it, expect } from "vitest";
import type { Repo, RepoSymbol, FileEntry, ScanResult, ParsedFile, IndexResult } from "./domain.js";

describe("domain types", () => {
  it("Repo type has required fields", () => {
    const repo: Repo = {
      id: "uuid-1",
      name: "test-repo",
      local_path: "/tmp/test",
      remote_url: null,
      stack: ["typescript"],
      entry_points: [],
      last_indexed_at: null,
      created_at: new Date().toISOString(),
    };
    expect(repo.id).toBe("uuid-1");
    expect(repo.stack).toContain("typescript");
  });

  it("RepoSymbol kind is restricted to valid values", () => {
    const sym: RepoSymbol = {
      id: "sym-1",
      repo_id: "repo-1",
      file_path: "src/index.ts",
      name: "createClient",
      kind: "function",
      signature: "(): Client",
      docstring: null,
      line_start: 1,
      line_end: 10,
      is_exported: true,
      updated_at: new Date().toISOString(),
    };
    expect(sym.kind).toBe("function");
  });

  it("ScanResult separates changed from deleted", () => {
    const result: ScanResult = {
      repoId: "repo-1",
      files: [],
      changed: ["src/a.ts"],
      deleted: ["src/old.ts"],
    };
    expect(result.changed).toHaveLength(1);
    expect(result.deleted).toHaveLength(1);
  });
});
```

- [ ] **Step 4: Run tests**

```bash
cd packages/shared && bunx vitest run src/domain.spec.ts
```

Expected: 3 tests passing.

- [ ] **Step 5: Commit**

```bash
git add packages/shared/src/domain.ts packages/shared/src/domain.spec.ts packages/shared/src/index.ts
git commit -m "feat(shared): add Phase 1 domain types — Repo, RepoSymbol, FileEntry, ScanResult, ParsedFile, IndexResult"
```

---

## Chunk 2: Engine Package — Scanner

### Task 3: Create packages/engine Package

**Files:**
- Create: `packages/engine/package.json`
- Create: `packages/engine/tsconfig.json`
- Create: `packages/engine/vitest.config.ts`
- Create: `packages/engine/src/index.ts`

- [ ] **Step 1: Create package.json**

Create `packages/engine/package.json`:

```json
{
  "name": "@sensei/engine",
  "version": "0.1.0",
  "type": "module",
  "exports": {
    ".": "./src/index.ts"
  },
  "scripts": {
    "build": "bun build src/index.ts --outdir dist --target bun",
    "test": "bunx vitest run",
    "test:watch": "bunx vitest"
  },
  "dependencies": {
    "@sensei/shared": "workspace:*",
    "@supabase/supabase-js": "^2.99.1",
    "fast-glob": "^3.3.0",
    "ts-morph": "^24.0.0"
  },
  "devDependencies": {
    "@types/node": "^22.0.0",
    "typescript": "^5.5.0",
    "vitest": "^4.0.18"
  }
}
```

- [ ] **Step 2: Create tsconfig.json**

Create `packages/engine/tsconfig.json`:

```json
{
  "extends": "../../config/tsconfig.base.json",
  "compilerOptions": {
    "outDir": "dist",
    "rootDir": "src"
  },
  "include": ["src/**/*"]
}
```

If `config/tsconfig.base.json` does not exist yet, create it:

```json
{
  "compilerOptions": {
    "target": "ESNext",
    "module": "ESNext",
    "moduleResolution": "bundler",
    "strict": true,
    "esModuleInterop": true,
    "skipLibCheck": true,
    "declaration": true,
    "declarationMap": true
  }
}
```

- [ ] **Step 3: Create vitest.config.ts**

Create `packages/engine/vitest.config.ts`:

```typescript
import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    include: ["src/**/*.spec.ts"],
    environment: "node",
    testTimeout: 30000,
  },
});
```

- [ ] **Step 4: Create placeholder src/index.ts**

Create `packages/engine/src/index.ts`:

```typescript
export * from "./scanner.js";
export * from "./adapters/typescript.js";
export * from "./indexer.js";
```

- [ ] **Step 5: Install ts-morph**

```bash
cd packages/engine && bun install
```

Expected: `bun.lock` updated, `ts-morph` present in node_modules.

- [ ] **Step 6: Add engine to CLI dependency**

In `packages/cli/package.json`, add to `"dependencies"`:
```json
"@sensei/engine": "workspace:*"
```

Then run:
```bash
bun install
```

- [ ] **Step 7: Commit**

```bash
git add packages/engine/ packages/cli/package.json bun.lock
git commit -m "feat(engine): scaffold engine package with ts-morph dependency"
```

---

### Task 4: Scanner — File Discovery and Change Detection

The Scanner produces a `ScanResult` — the list of all files, which changed, and which were deleted since the last index run. It compares current file mtimes/hashes against `sensei.scan_state` in Supabase.

**Files:**
- Create: `packages/engine/src/scanner.ts`
- Create: `packages/engine/src/scanner.spec.ts`

- [ ] **Step 1: Write the failing test first**

Create `packages/engine/src/scanner.spec.ts`:

```typescript
import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { mkdtemp, writeFile, rm, mkdir } from "fs/promises";
import { join } from "path";
import { tmpdir } from "os";
import { Scanner } from "./scanner.js";

describe("Scanner", () => {
  let dir: string;

  beforeEach(async () => {
    dir = await mkdtemp(join(tmpdir(), "engine-scanner-"));
    await mkdir(join(dir, "src"), { recursive: true });
    await writeFile(join(dir, "src/a.ts"), "export const a = 1;");
    await writeFile(join(dir, "src/b.ts"), "export const b = 2;");
    await mkdir(join(dir, "node_modules"), { recursive: true });
    await writeFile(join(dir, "node_modules/lib.js"), "// library");
  });

  afterEach(async () => {
    await rm(dir, { recursive: true, force: true });
  });

  it("discovers typescript files and excludes node_modules", async () => {
    const scanner = new Scanner({ repoPath: dir, repoId: "test-repo" });
    const result = await scanner.scan();
    const paths = result.files.map(f => f.path);
    expect(paths).toContain("src/a.ts");
    expect(paths).toContain("src/b.ts");
    expect(paths.some(p => p.includes("node_modules"))).toBe(false);
  });

  it("marks all files as changed on first scan (no prior state)", async () => {
    const scanner = new Scanner({ repoPath: dir, repoId: "test-repo" });
    const result = await scanner.scan();
    expect(result.changed).toContain("src/a.ts");
    expect(result.changed).toContain("src/b.ts");
    expect(result.deleted).toHaveLength(0);
  });

  it("marks only modified file as changed on re-scan", async () => {
    // Simulate prior state: both files already indexed with current hashes
    const scanner = new Scanner({ repoPath: dir, repoId: "test-repo" });
    const firstResult = await scanner.scan();

    // Modify one file
    await writeFile(join(dir, "src/a.ts"), "export const a = 99;");

    // Re-scan with prior state injected
    const scanner2 = new Scanner({
      repoPath: dir,
      repoId: "test-repo",
      priorState: firstResult.files.map(f => ({
        file_path: f.path,
        mtime: f.mtime,
        content_hash: f.hash,
      })),
    });
    const result2 = await scanner2.scan();
    expect(result2.changed).toContain("src/a.ts");
    expect(result2.changed).not.toContain("src/b.ts");
  });

  it("marks deleted file in result.deleted", async () => {
    const scanner = new Scanner({ repoPath: dir, repoId: "test-repo" });
    await scanner.scan();

    // Remove a.ts, re-scan with prior state that includes it
    await rm(join(dir, "src/a.ts"));
    const scanner2 = new Scanner({
      repoPath: dir,
      repoId: "test-repo",
      priorState: [{ file_path: "src/a.ts", mtime: 0, content_hash: "old" }],
    });
    const result2 = await scanner2.scan();
    expect(result2.deleted).toContain("src/a.ts");
    expect(result2.changed).not.toContain("src/a.ts");
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cd packages/engine && bunx vitest run src/scanner.spec.ts
```

Expected: FAIL — `Scanner` not found.

- [ ] **Step 3: Implement scanner.ts**

Create `packages/engine/src/scanner.ts`:

```typescript
import glob from "fast-glob";
import { createHash } from "crypto";
import { readFile, stat } from "fs/promises";
import { join, relative } from "path";
import type { FileEntry, ScanResult } from "@sensei/shared";

const DEFAULT_EXCLUDE = [
  "**/node_modules/**",
  "**/.git/**",
  "**/dist/**",
  "**/build/**",
  "**/.sensei/**",
  "**/__pycache__/**",
  "**/.venv/**",
];

const DEFAULT_EXTENSIONS = [
  "**/*.ts",
  "**/*.tsx",
  "**/*.js",
  "**/*.jsx",
  "**/*.mjs",
  "**/*.cjs",
  "**/*.py",
  "**/*.go",
  "**/*.rs",
  "**/*.md",
  "**/*.yaml",
  "**/*.yml",
  "**/*.json",
  "**/*.toml",
];

export interface PriorFileState {
  file_path: string;
  mtime: number;
  content_hash: string;
}

export interface ScannerOptions {
  repoPath: string;
  repoId: string;
  include?: string[];
  exclude?: string[];
  priorState?: PriorFileState[];
}

export class Scanner {
  constructor(private opts: ScannerOptions) {}

  async scan(): Promise<ScanResult> {
    const { repoPath, repoId } = this.opts;
    const include = this.opts.include ?? DEFAULT_EXTENSIONS;
    const exclude = this.opts.exclude ?? DEFAULT_EXCLUDE;

    const absPaths = await glob(include, {
      cwd: repoPath,
      ignore: exclude,
      absolute: true,
      followSymbolicLinks: false,
    });

    const files: FileEntry[] = await Promise.all(
      absPaths.map(abs => this.fingerprint(abs, repoPath))
    );

    const priorByPath = new Map<string, PriorFileState>(
      (this.opts.priorState ?? []).map(s => [s.file_path, s])
    );

    const currentPaths = new Set(files.map(f => f.path));
    const priorPaths = new Set(priorByPath.keys());

    const changed = files
      .filter(f => {
        const prior = priorByPath.get(f.path);
        if (!prior) return true;                         // new file
        if (prior.mtime !== f.mtime) {
          return prior.content_hash !== f.hash;          // mtime changed → check hash
        }
        return false;                                    // unchanged
      })
      .map(f => f.path);

    const deleted = [...priorPaths].filter(p => !currentPaths.has(p));

    return { repoId, files, changed, deleted };
  }

  private async fingerprint(absPath: string, repoPath: string): Promise<FileEntry> {
    const [stats, contents] = await Promise.all([
      stat(absPath),
      readFile(absPath),
    ]);
    const hash = createHash("sha256").update(contents).digest("hex");
    return {
      path: relative(repoPath, absPath),
      absPath,
      mtime: stats.mtimeMs,
      hash,
      size: stats.size,
    };
  }
}
```

- [ ] **Step 4: Run tests and verify they pass**

```bash
cd packages/engine && bunx vitest run src/scanner.spec.ts
```

Expected: 4 tests passing.

- [ ] **Step 5: Commit**

```bash
git add packages/engine/src/scanner.ts packages/engine/src/scanner.spec.ts
git commit -m "feat(engine): Scanner — file discovery with mtime/hash fingerprinting and incremental change detection"
```

---

### Task 5: TypeScript Adapter — AST Symbol Extraction

The TypeScript adapter uses `ts-morph` to parse `.ts`/`.tsx`/`.js`/`.jsx` files and extract exported functions, classes, types, interfaces, enums, and const declarations. It also extracts call edges and import relationships.

**Files:**
- Create: `packages/engine/src/adapters/typescript.ts`
- Create: `packages/engine/src/adapters/typescript.spec.ts`
- Create: `packages/engine/src/adapters/fixtures/sample.ts` (test fixture)

- [ ] **Step 1: Create the fixture file**

Create `packages/engine/src/adapters/fixtures/sample.ts`:

```typescript
import { readFile } from "fs/promises";
import { join } from "path";

/**
 * Reads a file and returns its contents as a string.
 */
export async function readTextFile(filePath: string): Promise<string> {
  const contents = await readFile(filePath, "utf-8");
  return contents.trim();
}

export class FileCache {
  private cache: Map<string, string> = new Map();

  get(key: string): string | undefined {
    return this.cache.get(key);
  }

  set(key: string, value: string): void {
    this.cache.set(key, value);
  }
}

export interface CacheOptions {
  ttl: number;
  maxSize: number;
}

export type CacheKey = string;

const DEFAULT_TTL = 60_000;

function resolveFilePath(base: string, name: string): string {
  return join(base, name);
}
```

- [ ] **Step 2: Write the failing test**

Create `packages/engine/src/adapters/typescript.spec.ts`:

```typescript
import { describe, it, expect } from "vitest";
import { resolve, dirname } from "path";
import { fileURLToPath } from "url";
import { TypeScriptAdapter } from "./typescript.js";

const _dir = dirname(fileURLToPath(import.meta.url));
const fixturePath = resolve(_dir, "fixtures/sample.ts");

describe("TypeScriptAdapter", () => {
  const adapter = new TypeScriptAdapter();

  it("handles .ts and .tsx extensions", () => {
    expect(adapter.extensions).toContain(".ts");
    expect(adapter.extensions).toContain(".tsx");
  });

  it("extracts exported function with docstring", async () => {
    const result = await adapter.parse({ path: "src/fixtures/sample.ts", absPath: fixturePath, mtime: 0, hash: "", size: 0 });
    const fn = result.symbols.find(s => s.name === "readTextFile");
    expect(fn).toBeDefined();
    expect(fn!.kind).toBe("function");
    expect(fn!.isExported).toBe(true);
    expect(fn!.docstring).toContain("Reads a file");
    expect(fn!.lineStart).toBeGreaterThan(0);
    expect(fn!.lineEnd).toBeGreaterThanOrEqual(fn!.lineStart);
  });

  it("extracts exported class", async () => {
    const result = await adapter.parse({ path: "src/fixtures/sample.ts", absPath: fixturePath, mtime: 0, hash: "", size: 0 });
    const cls = result.symbols.find(s => s.name === "FileCache");
    expect(cls).toBeDefined();
    expect(cls!.kind).toBe("class");
    expect(cls!.isExported).toBe(true);
  });

  it("extracts exported interface", async () => {
    const result = await adapter.parse({ path: "src/fixtures/sample.ts", absPath: fixturePath, mtime: 0, hash: "", size: 0 });
    const iface = result.symbols.find(s => s.name === "CacheOptions");
    expect(iface).toBeDefined();
    expect(iface!.kind).toBe("interface");
  });

  it("extracts exported type alias", async () => {
    const result = await adapter.parse({ path: "src/fixtures/sample.ts", absPath: fixturePath, mtime: 0, hash: "", size: 0 });
    const t = result.symbols.find(s => s.name === "CacheKey");
    expect(t).toBeDefined();
    expect(t!.kind).toBe("type");
  });

  it("does not include unexported function", async () => {
    const result = await adapter.parse({ path: "src/fixtures/sample.ts", absPath: fixturePath, mtime: 0, hash: "", size: 0 });
    const internal = result.symbols.find(s => s.name === "resolveFilePath");
    expect(internal).toBeUndefined(); // not exported — adapter skips it
  });

  it("extracts import from 'fs/promises'", async () => {
    const result = await adapter.parse({ path: "src/fixtures/sample.ts", absPath: fixturePath, mtime: 0, hash: "", size: 0 });
    const imp = result.imports.find(i => i.targetPath === "fs/promises");
    expect(imp).toBeDefined();
    expect(imp!.names).toContain("readFile");
  });
});
```

- [ ] **Step 3: Run test to verify it fails**

```bash
cd packages/engine && bunx vitest run src/adapters/typescript.spec.ts
```

Expected: FAIL — `TypeScriptAdapter` not found.

- [ ] **Step 4: Implement TypeScriptAdapter**

Create `packages/engine/src/adapters/typescript.ts`:

```typescript
import { Project, SyntaxKind, type SourceFile } from "ts-morph";
import type { FileEntry, ParsedFile, ParsedSymbol, ParsedEdge, ParsedImport, SymbolKind } from "@sensei/shared";

export class TypeScriptAdapter {
  readonly extensions = [".ts", ".tsx", ".js", ".jsx", ".mjs", ".cjs"];

  async parse(file: FileEntry): Promise<ParsedFile> {
    const project = new Project({
      useInMemoryFileSystem: false,
      compilerOptions: { allowJs: true, jsx: 1 },
    });

    let sf: SourceFile;
    try {
      sf = project.addSourceFileAtPath(file.absPath);
    } catch {
      return { filePath: file.path, language: "typescript", symbols: [], edges: [], imports: [] };
    }

    const symbols = this.extractSymbols(sf, file.path);
    const imports = this.extractImports(sf);
    const edges = this.extractEdges(sf, symbols);

    return { filePath: file.path, language: "typescript", symbols, edges, imports };
  }

  private extractSymbols(sf: SourceFile, filePath: string): ParsedSymbol[] {
    const results: ParsedSymbol[] = [];

    // Exported functions
    for (const fn of sf.getFunctions()) {
      if (!fn.isExported()) continue;
      results.push({
        name: fn.getName() ?? "<anonymous>",
        kind: "function",
        signature: fn.getSignature()?.getDeclaration()?.getText()?.split("{")[0]?.trim() ?? null,
        docstring: this.getDocstring(fn),
        lineStart: fn.getStartLineNumber(),
        lineEnd: fn.getEndLineNumber(),
        isExported: true,
      });
    }

    // Exported classes
    for (const cls of sf.getClasses()) {
      if (!cls.isExported()) continue;
      results.push({
        name: cls.getName() ?? "<anonymous>",
        kind: "class",
        signature: `class ${cls.getName()}`,
        docstring: this.getDocstring(cls),
        lineStart: cls.getStartLineNumber(),
        lineEnd: cls.getEndLineNumber(),
        isExported: true,
      });
    }

    // Exported interfaces
    for (const iface of sf.getInterfaces()) {
      if (!iface.isExported()) continue;
      results.push({
        name: iface.getName(),
        kind: "interface",
        signature: `interface ${iface.getName()}`,
        docstring: this.getDocstring(iface),
        lineStart: iface.getStartLineNumber(),
        lineEnd: iface.getEndLineNumber(),
        isExported: true,
      });
    }

    // Exported type aliases
    for (const ta of sf.getTypeAliases()) {
      if (!ta.isExported()) continue;
      results.push({
        name: ta.getName(),
        kind: "type",
        signature: `type ${ta.getName()} = ${ta.getTypeNode()?.getText() ?? "unknown"}`,
        docstring: this.getDocstring(ta),
        lineStart: ta.getStartLineNumber(),
        lineEnd: ta.getEndLineNumber(),
        isExported: true,
      });
    }

    // Exported enums
    for (const en of sf.getEnums()) {
      if (!en.isExported()) continue;
      results.push({
        name: en.getName(),
        kind: "enum",
        signature: `enum ${en.getName()}`,
        docstring: this.getDocstring(en),
        lineStart: en.getStartLineNumber(),
        lineEnd: en.getEndLineNumber(),
        isExported: true,
      });
    }

    // Exported const declarations
    for (const vs of sf.getVariableStatements()) {
      if (!vs.isExported()) continue;
      for (const decl of vs.getDeclarations()) {
        const name = decl.getName();
        results.push({
          name,
          kind: "const",
          signature: `const ${name}`,
          docstring: this.getDocstring(vs),
          lineStart: vs.getStartLineNumber(),
          lineEnd: vs.getEndLineNumber(),
          isExported: true,
        });
      }
    }

    return results;
  }

  private extractImports(sf: SourceFile): ParsedImport[] {
    return sf.getImportDeclarations().map(decl => ({
      targetPath: decl.getModuleSpecifierValue(),
      names: [
        ...decl.getNamedImports().map(n => n.getName()),
        ...(decl.getDefaultImport() ? [decl.getDefaultImport()!.getText()] : []),
      ],
    }));
  }

  private extractEdges(sf: SourceFile, symbols: ParsedSymbol[]): ParsedEdge[] {
    const edges: ParsedEdge[] = [];
    const symbolNames = new Set(symbols.map(s => s.name));

    for (const fn of sf.getFunctions()) {
      if (!fn.isExported()) continue;
      const callerName = fn.getName();
      if (!callerName) continue;

      const calls = fn.getDescendantsOfKind(SyntaxKind.CallExpression);
      for (const call of calls) {
        const expr = call.getExpression();
        const calleeName = expr.getText().split(".").pop() ?? expr.getText();
        if (calleeName && calleeName !== callerName) {
          edges.push({ callerName, calleeName, calleeFile: null });
        }
      }
    }

    return edges;
  }

  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  private getDocstring(node: any): string | null {
    try {
      const jsDoc = node.getJsDocs?.();
      if (jsDoc && jsDoc.length > 0) {
        return jsDoc[0].getDescription().trim() || null;
      }
    } catch {}
    return null;
  }
}
```

- [ ] **Step 5: Run tests and verify they pass**

```bash
cd packages/engine && bunx vitest run src/adapters/typescript.spec.ts
```

Expected: 7 tests passing.

- [ ] **Step 6: Commit**

```bash
git add packages/engine/src/adapters/typescript.ts packages/engine/src/adapters/typescript.spec.ts packages/engine/src/adapters/fixtures/sample.ts
git commit -m "feat(engine): TypeScriptAdapter — AST symbol, edge, and import extraction using ts-morph"
```

---

## Chunk 3: Engine — Indexer

### Task 6: Indexer — Supabase Writes

The Indexer takes `ScanResult` + per-file `ParsedFile` results and upserts them into Supabase. It also deletes scan_state rows for deleted files. It is the single write boundary to Supabase for indexed data.

**Files:**
- Create: `packages/engine/src/indexer.ts`
- Create: `packages/engine/src/indexer.spec.ts`
- Create: `packages/engine/src/indexer.integration.spec.ts`

> **Note on tests:** Unit tests mock the Supabase client. The integration test hits a real local Supabase instance (requires `supabase start`). Run integration tests with `SUPABASE_INTEGRATION=1 bunx vitest run src/indexer.integration.spec.ts`.

- [ ] **Step 1: Write the unit test (mocked client)**

Create `packages/engine/src/indexer.spec.ts`:

```typescript
import { describe, it, expect, vi } from "vitest";
import { Indexer } from "./indexer.js";
import type { ScanResult, ParsedFile } from "@sensei/shared";

// Each call to from() gets a fresh mock object that correctly stubs all chained methods.
function makeMockClient() {
  const upsertFn = vi.fn().mockResolvedValue({ error: null });
  const inDeleteFn = vi.fn().mockResolvedValue({ error: null });
  const eqDeleteFn = vi.fn(() => ({ in: inDeleteFn }));
  const deleteChain = vi.fn(() => ({ eq: eqDeleteFn }));

  const fromFn = vi.fn(() => ({
    upsert: upsertFn,
    delete: deleteChain,
    select: vi.fn(() => ({
      eq: vi.fn(() => ({
        select: vi.fn().mockResolvedValue({ data: [], error: null }),
        in: vi.fn().mockResolvedValue({ data: [], error: null }),
      })),
    })),
  }));
  return { from: fromFn, _upsert: upsertFn };
}

describe("Indexer", () => {
  it("calls upsert for each parsed file's symbols", async () => {
    const client = makeMockClient();
    const indexer = new Indexer(client as any);

    const scan: ScanResult = {
      repoId: "repo-1",
      files: [],
      changed: ["src/a.ts"],
      deleted: [],
    };

    const parsed: ParsedFile[] = [{
      filePath: "src/a.ts",
      language: "typescript",
      symbols: [{
        name: "foo",
        kind: "function",
        signature: "(): void",
        docstring: null,
        lineStart: 1,
        lineEnd: 5,
        isExported: true,
      }],
      edges: [],
      imports: [],
    }];

    const result = await indexer.indexFiles(scan, parsed);
    expect(result.symbolsUpserted).toBe(1);
    expect(result.errors).toHaveLength(0);
  });

  it("returns zero counts when no parsed files given", async () => {
    const client = makeMockClient();
    const indexer = new Indexer(client as any);
    const scan: ScanResult = { repoId: "repo-1", files: [], changed: [], deleted: [] };
    const result = await indexer.indexFiles(scan, []);
    expect(result.symbolsUpserted).toBe(0);
    expect(result.filesIndexed).toBe(0);
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cd packages/engine && bunx vitest run src/indexer.spec.ts
```

Expected: FAIL — `Indexer` not found.

- [ ] **Step 3: Implement indexer.ts**

Create `packages/engine/src/indexer.ts`:

```typescript
import type { SupabaseClient } from "@supabase/supabase-js";
import type { ScanResult, ParsedFile, IndexResult } from "@sensei/shared";

export class Indexer {
  constructor(private client: SupabaseClient) {}

  async indexFiles(scan: ScanResult, parsed: ParsedFile[]): Promise<IndexResult> {
    const start = Date.now();
    let symbolsUpserted = 0;
    let edgesUpserted = 0;
    let importsUpserted = 0;
    let filesIndexed = 0;
    const errors: string[] = [];

    for (const file of parsed) {
      try {
        // Upsert symbols
        if (file.symbols.length > 0) {
          const symbolRows = file.symbols.map(s => ({
            repo_id: scan.repoId,
            file_path: file.filePath,
            name: s.name,
            kind: s.kind,
            signature: s.signature,
            docstring: s.docstring,
            line_start: s.lineStart,
            line_end: s.lineEnd,
            is_exported: s.isExported,
            updated_at: new Date().toISOString(),
          }));
          const { error } = await this.client
            .from("symbols")
            .upsert(symbolRows, { onConflict: "repo_id,file_path,name,kind" });
          if (error) {
            errors.push(`symbols upsert ${file.filePath}: ${error.message}`);
          } else {
            symbolsUpserted += symbolRows.length;
          }
        }

        // Upsert imports
        if (file.imports.length > 0) {
          const importRows = file.imports.map(i => ({
            repo_id: scan.repoId,
            source_file: file.filePath,
            target_path: i.targetPath,
            names: i.names,
          }));
          const { error } = await this.client
            .from("imports")
            .upsert(importRows, { onConflict: "repo_id,source_file,target_path" });
          if (error) {
            errors.push(`imports upsert ${file.filePath}: ${error.message}`);
          } else {
            importsUpserted += importRows.length;
          }
        }

        // Upsert call edges (requires symbol IDs — look up caller by name first)
        if (file.edges.length > 0) {
          const { data: callerRows } = await this.client
            .from("symbols")
            .select("id,name")
            .eq("repo_id", scan.repoId)
            .eq("file_path", file.filePath);

          const callerIdByName = Object.fromEntries(
            (callerRows ?? []).map(r => [r.name, r.id])
          );

          const edgeRows = file.edges
            .map(e => ({
              repo_id: scan.repoId,
              caller_id: callerIdByName[e.callerName],
              callee_name: e.calleeName,
              callee_file: e.calleeFile,
            }))
            .filter(r => r.caller_id != null);

          if (edgeRows.length > 0) {
            const { error } = await this.client.from("call_edges").insert(edgeRows);
            if (error) {
              errors.push(`call_edges insert ${file.filePath}: ${error.message}`);
            } else {
              edgesUpserted += edgeRows.length;
            }
          }
        }

        filesIndexed++;
      } catch (err) {
        errors.push(`${file.filePath}: ${err instanceof Error ? err.message : String(err)}`);
      }
    }

    // Update scan state for all changed files
    if (scan.files.length > 0) {
      const stateRows = scan.files
        .filter(f => scan.changed.includes(f.path))
        .map(f => ({
          repo_id: scan.repoId,
          file_path: f.path,
          mtime: Math.floor(f.mtime),
          content_hash: f.hash,
          indexed_at: new Date().toISOString(),
        }));

      if (stateRows.length > 0) {
        const { error } = await this.client
          .from("scan_state")
          .upsert(stateRows, { onConflict: "repo_id,file_path" });
        if (error) errors.push(`scan_state upsert: ${error.message}`);
      }
    }

    // Remove deleted files from scan_state and symbols
    if (scan.deleted.length > 0) {
      await this.client
        .from("scan_state")
        .delete()
        .eq("repo_id", scan.repoId)
        .in("file_path", scan.deleted);

      await this.client
        .from("symbols")
        .delete()
        .eq("repo_id", scan.repoId)
        .in("file_path", scan.deleted);
    }

    return {
      repoId: scan.repoId,
      symbolsUpserted,
      edgesUpserted,
      importsUpserted,
      filesIndexed,
      filesDeleted: scan.deleted.length,
      durationMs: Date.now() - start,
      errors,
    };
  }
}
```

- [ ] **Step 4: Run unit tests**

```bash
cd packages/engine && bunx vitest run src/indexer.spec.ts
```

Expected: 2 tests passing.

- [ ] **Step 5: Write the integration test**

Create `packages/engine/src/indexer.integration.spec.ts`:

```typescript
/**
 * Integration test — requires local Supabase.
 * Run with: SUPABASE_INTEGRATION=1 bunx vitest run src/indexer.integration.spec.ts
 */
import { describe, it, expect, beforeAll } from "vitest";
import { createClient } from "@supabase/supabase-js";
import { Indexer } from "./indexer.js";
import type { ScanResult, ParsedFile } from "@sensei/shared";

const SUPABASE_URL = process.env.SUPABASE_URL ?? "http://localhost:54321";
const SUPABASE_KEY = process.env.SUPABASE_SERVICE_KEY ?? "";
const RUN = process.env.SUPABASE_INTEGRATION === "1";

describe.skipIf(!RUN || !SUPABASE_KEY)("Indexer integration", () => {
  let client: ReturnType<typeof createClient>;
  const testRepoId = `test-${Date.now()}`;

  beforeAll(async () => {
    client = createClient(SUPABASE_URL, SUPABASE_KEY, {
      db: { schema: "sensei" },
      auth: { persistSession: false },
    });

    // Ensure a repo row exists for foreign key constraints
    await client.from("repos").upsert({
      id: testRepoId,
      name: "test-repo",
      local_path: `/tmp/test-${testRepoId}`,
      stack: [],
      entry_points: [],
    });
  });

  it("upserts symbols into Supabase and reads them back", async () => {
    const indexer = new Indexer(client as any);

    const scan: ScanResult = {
      repoId: testRepoId,
      files: [{ path: "src/test.ts", absPath: "/tmp/test.ts", mtime: Date.now(), hash: "abc", size: 100 }],
      changed: ["src/test.ts"],
      deleted: [],
    };

    const parsed: ParsedFile[] = [{
      filePath: "src/test.ts",
      language: "typescript",
      symbols: [{
        name: "createClient",
        kind: "function",
        signature: "(): Client",
        docstring: "Creates a client",
        lineStart: 1,
        lineEnd: 5,
        isExported: true,
      }],
      edges: [],
      imports: [],
    }];

    const result = await indexer.indexFiles(scan, parsed);
    expect(result.errors).toHaveLength(0);
    expect(result.symbolsUpserted).toBe(1);

    const { data } = await client
      .from("symbols")
      .select("*")
      .eq("repo_id", testRepoId)
      .eq("name", "createClient");

    expect(data).toHaveLength(1);
    expect(data![0].kind).toBe("function");
  });
});
```

- [ ] **Step 6: Run integration test (requires supabase start)**

```bash
SUPABASE_INTEGRATION=1 SUPABASE_SERVICE_KEY=<your-service-key> cd packages/engine && bunx vitest run src/indexer.integration.spec.ts
```

Expected: 1 test passing, symbol appears in Supabase.

- [ ] **Step 7: Commit**

```bash
git add packages/engine/src/indexer.ts packages/engine/src/indexer.spec.ts packages/engine/src/indexer.integration.spec.ts
git commit -m "feat(engine): Indexer — upsert symbols/imports/scan_state to Supabase, remove deleted files"
```

---

### Task 7: Engine Pipeline — Full Scan → Parse → Index

Wire up a single `indexRepo` function that orchestrates Scanner + TypeScriptAdapter + Indexer. This is what `sensei init` will call.

**Files:**
- Create: `packages/engine/src/pipeline.ts`
- Create: `packages/engine/src/pipeline.spec.ts`
- Modify: `packages/engine/src/index.ts`

- [ ] **Step 1: Write the failing test**

Create `packages/engine/src/pipeline.spec.ts`:

```typescript
import { describe, it, expect, vi } from "vitest";
import { indexRepo } from "./pipeline.js";

describe("indexRepo pipeline", () => {
  it("returns IndexResult with repoId", async () => {
    // This test uses the real Scanner and TypeScriptAdapter but a mock Supabase client.
    // Point it at a directory that contains some .ts files.
    const mockClient = {
      from: vi.fn(() => ({
        upsert: vi.fn().mockResolvedValue({ error: null }),
        delete: vi.fn(() => ({ eq: vi.fn(() => ({ in: vi.fn().mockResolvedValue({ error: null }) })) })),
      })),
    };

    const result = await indexRepo({
      repoPath: process.cwd(), // packages/engine itself has .ts files
      repoId: "test-pipeline",
      client: mockClient as any,
    });

    expect(result.repoId).toBe("test-pipeline");
    expect(result.symbolsUpserted).toBeGreaterThan(0);
    expect(result.filesIndexed).toBeGreaterThan(0);
    expect(result.errors).toHaveLength(0);
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cd packages/engine && bunx vitest run src/pipeline.spec.ts
```

Expected: FAIL — `indexRepo` not found.

- [ ] **Step 3: Implement pipeline.ts**

Create `packages/engine/src/pipeline.ts`:

```typescript
import type { SupabaseClient } from "@supabase/supabase-js";
import type { IndexResult } from "@sensei/shared";
import { Scanner } from "./scanner.js";
import { TypeScriptAdapter } from "./adapters/typescript.js";
import { Indexer } from "./indexer.js";

export interface IndexRepoOptions {
  repoPath: string;
  repoId: string;
  client: SupabaseClient;
  include?: string[];
  exclude?: string[];
}

export async function indexRepo(opts: IndexRepoOptions): Promise<IndexResult> {
  const { repoPath, repoId, client } = opts;

  // Load prior scan state for incremental indexing
  let priorState: Array<{ file_path: string; mtime: number; content_hash: string }> = [];
  try {
    const { data } = await client
      .from("scan_state")
      .select("file_path,mtime,content_hash")
      .eq("repo_id", repoId);
    priorState = (data ?? []) as typeof priorState;
  } catch {
    // First run — no prior state
  }

  // Scan
  const scanner = new Scanner({ repoPath, repoId, priorState, include: opts.include, exclude: opts.exclude });
  const scan = await scanner.scan();

  // Parse only changed files
  const adapter = new TypeScriptAdapter();
  const parsedFiles = await Promise.all(
    scan.files
      .filter(f => scan.changed.includes(f.path) && adapter.extensions.some(ext => f.path.endsWith(ext)))
      .map(f => adapter.parse(f).catch(() => null))
  );
  const validParsed = parsedFiles.filter((p): p is NonNullable<typeof p> => p !== null);

  // Index
  const indexer = new Indexer(client);
  return indexer.indexFiles(scan, validParsed);
}
```

- [ ] **Step 4: Update engine/src/index.ts to export pipeline**

```typescript
export * from "./scanner.js";
export * from "./adapters/typescript.js";
export * from "./indexer.js";
export * from "./pipeline.js";
```

- [ ] **Step 5: Run tests**

```bash
cd packages/engine && bunx vitest run src/pipeline.spec.ts
```

Expected: 1 test passing, symbolsUpserted > 0 (the engine's own .ts files).

- [ ] **Step 6: Commit**

```bash
git add packages/engine/src/pipeline.ts packages/engine/src/pipeline.spec.ts packages/engine/src/index.ts
git commit -m "feat(engine): indexRepo pipeline — wires Scanner + TypeScriptAdapter + Indexer end-to-end"
```

---

## Chunk 4: MCP Server

### Task 8: MCP Server — get_session_context, search, load_context

The MCP server uses stdio transport. Tool handlers are thin wrappers: validate input → call shared/engine → return formatted result. The existing `packages/server` has an HTTP report server; we add the MCP server alongside it.

**Files:**
- Create: `packages/server/src/mcp-server.ts`
- Create: `packages/server/src/tools/get-session-context.ts`
- Create: `packages/server/src/tools/search.ts`
- Create: `packages/server/src/tools/load-context.ts`
- Create: `packages/server/src/mcp-server.spec.ts`
- Modify: `packages/server/package.json` (add MCP SDK + engine dependency)
- Modify: `packages/server/src/index.ts`

- [ ] **Step 1: Add MCP SDK, engine dependency, and vitest to server**

In `packages/server/package.json`, add to `"dependencies"`:
```json
"@modelcontextprotocol/sdk": "^1.27.1",
"@sensei/engine": "workspace:*"
```

And add to `"devDependencies"`:
```json
"vitest": "^4.0.18"
```

Run:
```bash
bun install
```

- [ ] **Step 2: Write failing tests**

Create `packages/server/src/mcp-server.spec.ts`:

```typescript
import { describe, it, expect } from "vitest";
import { createSenseiMcpServer } from "./mcp-server.js";

describe("createSenseiMcpServer", () => {
  it("returns a server object with a connect method", () => {
    const server = createSenseiMcpServer({ repoId: "test", repoPath: "/tmp" });
    expect(server).toBeDefined();
    expect(typeof server.connect).toBe("function");
  });
});
```

- [ ] **Step 3: Run test to verify it fails**

```bash
cd packages/server && bunx vitest run src/mcp-server.spec.ts
```

Expected: FAIL — `createSenseiMcpServer` not found.

- [ ] **Step 4: Implement tool handlers**

Create `packages/server/src/tools/get-session-context.ts`:

```typescript
import type { SupabaseClient } from "@supabase/supabase-js";

export interface SessionContextResult {
  repo_name: string;
  repo_path: string;
  symbol_count: number;
  file_count: number;
  last_indexed_at: string | null;
  stack: string[];
  message: string;
}

export async function getSessionContext(
  client: SupabaseClient,
  repoId: string,
  repoPath: string,
): Promise<SessionContextResult> {
  const { data: repo } = await client.from("repos").select("*").eq("id", repoId).single();
  const { count: symbolCount } = await client
    .from("symbols")
    .select("*", { count: "exact", head: true })
    .eq("repo_id", repoId);
  const { count: fileCount } = await client
    .from("scan_state")
    .select("*", { count: "exact", head: true })
    .eq("repo_id", repoId);

  return {
    repo_name: repo?.name ?? "unknown",
    repo_path: repoPath,
    symbol_count: symbolCount ?? 0,
    file_count: fileCount ?? 0,
    last_indexed_at: repo?.last_indexed_at ?? null,
    stack: repo?.stack ?? [],
    message: `Repo "${repo?.name ?? "unknown"}" — ${symbolCount ?? 0} symbols across ${fileCount ?? 0} files. Call search() to find code.`,
  };
}
```

Create `packages/server/src/tools/search.ts`:

```typescript
import type { SupabaseClient } from "@supabase/supabase-js";
import type { RepoSymbol } from "@sensei/shared";

export interface SearchResult {
  symbols: Array<{
    name: string;
    kind: string;
    file_path: string;
    line_start: number;
    signature: string | null;
    docstring: string | null;
  }>;
  total: number;
  query: string;
}

export async function search(
  client: SupabaseClient,
  repoId: string,
  query: string,
  limit = 20,
): Promise<SearchResult> {
  // Phase 1: substring text search using ilike. Phase 2 upgrades to BM25 via pg_trgm / to_tsvector.
  const { data, error } = await client
    .from("symbols")
    .select("name,kind,file_path,line_start,signature,docstring")
    .eq("repo_id", repoId)
    .or(`name.ilike.%${query}%,signature.ilike.%${query}%,docstring.ilike.%${query}%`)
    .eq("is_exported", true)
    .order("name")
    .limit(limit);

  if (error) throw new Error(`Search failed: ${error.message}`);

  return {
    symbols: (data ?? []).map(s => ({
      name: s.name,
      kind: s.kind,
      file_path: s.file_path,
      line_start: s.line_start,
      signature: s.signature,
      docstring: s.docstring,
    })),
    total: (data ?? []).length,
    query,
  };
}
```

Create `packages/server/src/tools/load-context.ts`:

```typescript
import type { SupabaseClient } from "@supabase/supabase-js";
import { readFile } from "fs/promises";
import { join } from "path";

export interface LoadContextResult {
  file_path: string;
  content: string;
  symbols: Array<{
    name: string;
    kind: string;
    line_start: number;
    line_end: number;
    signature: string | null;
    is_exported: boolean;
  }>;
  line_count: number;
}

export async function loadContext(
  client: SupabaseClient,
  repoId: string,
  repoPath: string,
  filePath: string,
): Promise<LoadContextResult> {
  const absPath = join(repoPath, filePath);
  let content: string;
  try {
    content = await readFile(absPath, "utf-8");
  } catch {
    throw new Error(`File not found: ${filePath}`);
  }

  const { data: symbols } = await client
    .from("symbols")
    .select("name,kind,line_start,line_end,signature,is_exported")
    .eq("repo_id", repoId)
    .eq("file_path", filePath)
    .order("line_start");

  return {
    file_path: filePath,
    content,
    symbols: (symbols ?? []).map(s => ({
      name: s.name,
      kind: s.kind,
      line_start: s.line_start,
      line_end: s.line_end,
      signature: s.signature,
      is_exported: s.is_exported,
    })),
    line_count: content.split("\n").length,
  };
}
```

- [ ] **Step 5: Implement the MCP server**

Create `packages/server/src/mcp-server.ts`:

```typescript
import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { z } from "zod";
import { makeSenseiClient } from "@sensei/shared";
import { getSessionContext } from "./tools/get-session-context.js";
import { search } from "./tools/search.js";
import { loadContext } from "./tools/load-context.js";

export interface McpServerOptions {
  repoId: string;
  repoPath: string;
}

export function createSenseiMcpServer(opts: McpServerOptions) {
  const server = new McpServer({
    name: "sensei",
    version: "0.1.0",
  });

  // Lazy client — created on first use so startup doesn't block on Supabase connection
  let clientPromise: ReturnType<typeof makeSenseiClient> | null = null;
  const getClient = () => {
    if (!clientPromise) clientPromise = makeSenseiClient(opts.repoPath);
    return clientPromise;
  };

  server.tool(
    "get_session_context",
    "Get orientation context for the current repo — symbol count, stack, last indexed timestamp",
    {},
    async () => {
      const client = await getClient();
      if (!client) return { content: [{ type: "text", text: "Error: Supabase client not configured. Run sensei init first." }] };
      const result = await getSessionContext(client as any, opts.repoId, opts.repoPath);
      return { content: [{ type: "text", text: JSON.stringify(result, null, 2) }] };
    }
  );

  server.tool(
    "search",
    "Search indexed symbols by name, signature, or docstring",
    {
      query: z.string().describe("Search term — matches symbol names, signatures, and docstrings"),
      limit: z.number().int().min(1).max(100).optional().default(20).describe("Max results to return"),
    },
    async ({ query, limit }) => {
      const client = await getClient();
      if (!client) return { content: [{ type: "text", text: "Error: Supabase client not configured." }] };
      const result = await search(client as any, opts.repoId, query, limit);
      return { content: [{ type: "text", text: JSON.stringify(result, null, 2) }] };
    }
  );

  server.tool(
    "load_context",
    "Load the source file at the given path along with its extracted symbols",
    {
      file_path: z.string().describe("Repo-relative file path, e.g. src/index.ts"),
    },
    async ({ file_path }) => {
      const client = await getClient();
      if (!client) return { content: [{ type: "text", text: "Error: Supabase client not configured." }] };
      const result = await loadContext(client as any, opts.repoId, opts.repoPath, file_path);
      return { content: [{ type: "text", text: JSON.stringify(result, null, 2) }] };
    }
  );

  return server;
}
```

- [ ] **Step 6: Run tests**

```bash
cd packages/server && bunx vitest run src/mcp-server.spec.ts
```

Expected: 1 test passing.

- [ ] **Step 7: Create the MCP entry point script**

Create `packages/server/src/mcp-entry.ts`:

```typescript
#!/usr/bin/env bun
/**
 * MCP server entry point — launched by agent via stdio transport.
 * Usage: bun packages/server/src/mcp-entry.ts --repo-id <id> --repo-path <path>
 */
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import { createSenseiMcpServer } from "./mcp-server.js";
import { loadSenseiConfig } from "@sensei/shared";

const repoPath = process.env.SENSEI_REPO_PATH ?? process.cwd();
const config = await loadSenseiConfig(repoPath);

if (!config) {
  console.error("[sensei-mcp] No .sensei/config.yaml found. Run sensei init first.");
  process.exit(1);
}

const server = createSenseiMcpServer({ repoId: config.repo_id, repoPath });
const transport = new StdioServerTransport();
await server.connect(transport);
```

- [ ] **Step 8: Update server/src/index.ts to export MCP server**

Add to `packages/server/src/index.ts`:
```typescript
export * from "./mcp-server.js";
```

- [ ] **Step 9: Commit**

```bash
git add packages/server/src/mcp-server.ts packages/server/src/mcp-entry.ts packages/server/src/tools/ packages/server/src/mcp-server.spec.ts packages/server/src/index.ts packages/server/package.json bun.lock
git commit -m "feat(server): MCP server with get_session_context, search, and load_context tools"
```

---

## Chunk 5: CLI — sensei init

### Task 9: Update sensei init to Use Engine + Write Config Files

The existing `init` command uses the old `reindexRepo` from `@sensei/tools`. Update it to use the new engine pipeline and Supabase, write `.sensei/config.yaml`, and generate `CLAUDE.md` and `AGENTS.md`.

**Files:**
- Modify: `packages/cli/src/commands/init.ts`
- Create: `packages/cli/src/templates/claude-md.ts`
- Create: `packages/cli/src/templates/agents-md.ts`

> **Important:** Read the current `packages/cli/src/commands/init.ts` in full before modifying it to understand what to preserve (Ollama checks, clack prompts UI, hook installation).

- [ ] **Step 1: Create CLAUDE.md template**

Create `packages/cli/src/templates/claude-md.ts`:

```typescript
export function claudeMdTemplate(opts: {
  repoName: string;
  stack: string[];
  repoId: string;
}): string {
  return `# Project Context — ${opts.repoName}

> Auto-generated by sensei. Update as needed.

## Orientation
Call \`get_session_context()\` to resume a session.
Call \`get_llmspec()\` for full repo orientation.

## Stack
${opts.stack.map(s => `- ${s}`).join("\n")}

## Shortcuts
See \`.sensei/shortcuts.md\`

## Patterns
See \`.sensei/patterns.md\`
`;
}
```

- [ ] **Step 2: Create AGENTS.md template**

Create `packages/cli/src/templates/agents-md.ts`:

```typescript
export function agentsMdTemplate(opts: {
  repoName: string;
  stack: string[];
}): string {
  return `# ${opts.repoName} — Agent Orientation

## Goals
This repo is indexed by sensei. Start every session with \`get_session_context()\`.

## Stack
${opts.stack.map(s => `- ${s}`).join("\n")}

## Guidelines
- Call \`search(query)\` to find symbols — do not grep or explore manually
- Call \`load_context(file_path)\` to read a file with its extracted symbols
- Call \`get_session_context()\` if you are unsure of your current task

## Patterns
See \`.sensei/patterns.md\` for project-specific conventions.
`;
}
```

- [ ] **Step 3: Update init.ts to use engine**

Replace the body of `packages/cli/src/commands/init.ts` with:

> **Note:** All imports must be at the top of the file — this is a complete rewrite of the file. Also remove `@sensei/tools` from the imports and from `packages/cli/package.json` if it is no longer used by other commands. Check `packages/cli/src/commands/` for any other file importing from `@sensei/tools` before removing it.

```typescript
import { intro, outro, spinner, note, log, isCancel, text } from "@clack/prompts";
import { writeFile, mkdir, access, readFile } from "fs/promises";
import { join } from "path";
import { homedir } from "os";
import { createClient } from "@supabase/supabase-js";
import { indexRepo } from "@sensei/engine";
import { claudeMdTemplate } from "../templates/claude-md.js";
import { agentsMdTemplate } from "../templates/agents-md.js";
import { installHooks } from "@sensei/collector";

export async function init(cwd: string): Promise<void> {
  intro("sensei init");

  // 1. Detect stack from manifest files
  const stack: string[] = [];
  const entryPoints: Array<{ path: string; role: string }> = [];

  try {
    const pkgJson = JSON.parse(await readFile(join(cwd, "package.json"), "utf-8").catch(() => "null"));
    if (pkgJson) {
      stack.push("typescript");
      if (pkgJson.dependencies?.["react"] || pkgJson.dependencies?.["svelte"]) {
        stack.push(pkgJson.dependencies?.["react"] ? "react" : "svelte");
      }
      if (pkgJson.main) entryPoints.push({ path: pkgJson.main, role: "main" });
      if (pkgJson.exports?.["."]?.default) entryPoints.push({ path: pkgJson.exports["."].default, role: "main" });
    }
  } catch {}

  try {
    await access(join(cwd, "pyproject.toml"));
    stack.push("python");
  } catch {}

  try {
    await access(join(cwd, "go.mod"));
    stack.push("go");
  } catch {}

  // 2. Prompt for Supabase URL and service key
  const supabaseUrl = await text({
    message: "Supabase URL (from supabase start or your hosted project):",
    placeholder: "http://localhost:54321",
    validate: v => (v.startsWith("http") ? undefined : "Must be a URL"),
  });
  if (isCancel(supabaseUrl)) { outro("Cancelled."); return; }

  const serviceKey = await text({
    message: "Supabase service role key:",
    validate: v => (v.length > 10 ? undefined : "Looks too short"),
  });
  if (isCancel(serviceKey)) { outro("Cancelled."); return; }

  // 3. Create Supabase client and upsert repo row
  const client = createClient(String(supabaseUrl), String(serviceKey), {
    db: { schema: "sensei" },
    auth: { persistSession: false },
  });

  const repoName = cwd.split("/").pop() ?? "repo";
  const { data: repo, error: repoErr } = await client.from("repos").upsert({
    name: repoName,
    local_path: cwd,
    stack,
    entry_points: entryPoints,
  }, { onConflict: "local_path" }).select("id").single();

  if (repoErr || !repo) {
    log.error(`Failed to register repo: ${repoErr?.message ?? "no data returned"}`);
    outro("Failed."); return;
  }
  const repoId: string = repo.id;

  // 4. Write .sensei/config.yaml and credentials
  const senseiDir = join(cwd, ".sensei");
  await mkdir(senseiDir, { recursive: true });
  await writeFile(join(senseiDir, "config.yaml"), `repo_id: ${repoId}\nsupabase_url: ${String(supabaseUrl)}\n`);

  // Write credentials to ~/.config/sensei/ (global, not committed)
  const credsDir = join(homedir(), ".config", "sensei");
  await mkdir(credsDir, { recursive: true });
  const credsPath = join(credsDir, "credentials.yaml");
  await writeFile(credsPath, `supabase_service_key: ${String(serviceKey)}\n`);
  // Restrict credentials file to owner-only (service role key — never share)
  const { chmod } = await import("fs/promises");
  await chmod(credsPath, 0o600);

  // 5. Run first index
  const indexSpinner = spinner();
  indexSpinner.start("Indexing repo (first full scan)...");
  let result;
  try {
    result = await indexRepo({ repoPath: cwd, repoId, client: client as any });
    indexSpinner.stop(`Indexed: ${result.filesIndexed} files, ${result.symbolsUpserted} symbols`);
    if (result.errors.length > 0) {
      log.warn(`${result.errors.length} indexing errors — see details below`);
      result.errors.slice(0, 3).forEach(e => log.warn(e));
    }
    // Update last_indexed_at
    await client.from("repos").update({ last_indexed_at: new Date().toISOString() }).eq("id", repoId);
  } catch (err) {
    indexSpinner.stop(`Indexing failed: ${err instanceof Error ? err.message : String(err)}`);
    log.warn("You can re-index later with: sensei index");
  }

  // 6. Write CLAUDE.md and AGENTS.md
  await writeFile(join(cwd, "CLAUDE.md"), claudeMdTemplate({ repoName, stack, repoId }));
  await writeFile(join(cwd, "AGENTS.md"), agentsMdTemplate({ repoName, stack }));

  // 7. Install hooks
  const hookSpinner = spinner();
  hookSpinner.start("Installing collector hooks...");
  try {
    await installHooks({});
    hookSpinner.stop("Hooks installed");
  } catch (err) {
    hookSpinner.stop(`Hook install skipped: ${err instanceof Error ? err.message : String(err)}`);
  }

  note(
    [
      `Created: .sensei/config.yaml, CLAUDE.md, AGENTS.md`,
      ``,
      `Next steps:`,
      `  1. Start the dashboard: cd apps/dashboard && bun run dev`,
      `  2. Start the collector: sensei serve`,
      `  3. Add the MCP server to your agent config`,
    ].join("\n"),
    "Setup complete"
  );

  outro("Done.");
}
```

- [ ] **Step 4: Add engine dep to CLI package.json if not already present**

Verify `packages/cli/package.json` has:
```json
"@sensei/engine": "workspace:*"
```

Run `bun install` from repo root.

- [ ] **Step 5: Test init in a scratch directory**

```bash
mkdir /tmp/test-sensei-init && cd /tmp/test-sensei-init && echo '{"name":"test-app","dependencies":{"react":"^18"}}' > package.json
SENSEI_REPO=/tmp/test-sensei-init bun run /Users/Jerry/Developer/sensei/packages/cli/src/cli.ts init
```

Enter the Supabase URL (`http://localhost:54321`) and service key when prompted.

Expected output:
- Indexed N files, M symbols
- `.sensei/config.yaml` created
- `CLAUDE.md` and `AGENTS.md` created

- [ ] **Step 6: Verify .sensei/config.yaml exists**

```bash
cat /tmp/test-sensei-init/.sensei/config.yaml
```

Expected: Contains `repo_id:` and `supabase_url:`.

- [ ] **Step 7: Commit**

```bash
cd /Users/Jerry/Developer/sensei
git add packages/cli/src/commands/init.ts packages/cli/src/templates/ bun.lock
git commit -m "feat(cli): update sensei init — engine pipeline, Supabase repo registration, CLAUDE.md + AGENTS.md generation"
```

---

## Chunk 6: Dashboard — Repo List and Symbol Browser

### Task 10: Dashboard Repo List View

The dashboard already has a `repos` route. Update it to read from Supabase `sensei.repos` and `sensei.symbols` using kavach for auth and Supabase data access.

**Auth approach:** The dashboard uses `~/Developer/kavach` — an internal library supporting SvelteKit + Supabase OAuth. Kavach provides the Supabase client and session context in SvelteKit server hooks and load functions. Phase 1 uses kavach's service-role client directly (no user auth yet — Phase 9 adds GitHub OAuth). Phase 9 will add user auth gates; Phase 1 just needs the Supabase connection.

**Files:**
- Create/Modify: `apps/dashboard/src/routes/repos/+page.server.ts`
- Modify: `apps/dashboard/src/routes/repos/+page.svelte`
- Create: `apps/dashboard/src/routes/repos/[id]/+page.server.ts`
- Create: `apps/dashboard/src/routes/repos/[id]/+page.svelte`

> **Why `+page.server.ts` not `+page.ts`:** SvelteKit page load functions in `+page.ts` run both server-side AND client-side. The Supabase service role key must never be exposed to the browser, and `process.cwd()` is not available in browser context. `+page.server.ts` runs only on the server, keeping the key secure.

> **Read first:**
> 1. `~/Developer/kavach` — read its README and SvelteKit integration docs to understand how to get the Supabase client in a server load function
> 2. `apps/dashboard/src/routes/+layout.svelte` and any existing `+hooks.server.ts` to see if kavach is already set up
> 3. `apps/dashboard/src/routes/repos/` for what already exists

If a `+page.ts` file exists in repos/, delete it: `rm apps/dashboard/src/routes/repos/+page.ts`

- [ ] **Step 1: Check kavach setup**

```bash
ls ~/Developer/kavach/
cat ~/Developer/kavach/README.md 2>/dev/null | head -60
ls apps/dashboard/src/hooks.server.ts 2>/dev/null || echo "no hooks.server.ts"
cat apps/dashboard/src/app.d.ts
```

Identify how kavach exposes the Supabase client in SvelteKit server load functions — typically via `event.locals.supabase` or a similar pattern set up in `hooks.server.ts`.

- [ ] **Step 2: Set up kavach in dashboard if not already done**

If kavach is not yet integrated into the dashboard, follow its SvelteKit setup instructions. At minimum this involves:
- Adding kavach as a dependency to `apps/dashboard/package.json`
- Creating or updating `src/hooks.server.ts` to attach the Supabase client to `locals`
- Updating `src/app.d.ts` to declare the `locals` type

Follow the kavach README exactly for this step.

- [ ] **Step 3: Create repos/+page.server.ts**

Using the Supabase client from kavach (e.g., `event.locals.supabase` or however kavach exposes it), create `apps/dashboard/src/routes/repos/+page.server.ts`:

```typescript
import type { PageServerLoad } from "./$types";

export const load: PageServerLoad = async (event) => {
  // Get the Supabase client from kavach locals — adjust property name to match kavach's API
  const client = event.locals.supabase;
  if (!client) return { repos: [] };

  const { data: repos } = await client
    .from("repos")
    .select("id,name,local_path,stack,last_indexed_at,created_at")
    .order("created_at", { ascending: false });

  // Get symbol counts per repo
  const symbolCounts = await Promise.all(
    (repos ?? []).map(async repo => {
      const { count } = await client
        .from("symbols")
        .select("*", { count: "exact", head: true })
        .eq("repo_id", repo.id);
      const { count: fileCount } = await client
        .from("scan_state")
        .select("*", { count: "exact", head: true })
        .eq("repo_id", repo.id);
      return { repo_id: repo.id, symbolCount: count ?? 0, fileCount: fileCount ?? 0 };
    })
  );

  const countMap = Object.fromEntries(symbolCounts.map(c => [c.repo_id, c]));

  return {
    repos: (repos ?? []).map(r => ({
      ...r,
      symbol_count: countMap[r.id]?.symbolCount ?? 0,
      file_count: countMap[r.id]?.fileCount ?? 0,
    })),
  };
};
```

- [ ] **Step 3: Update or create repos/+page.svelte**

Create `apps/dashboard/src/routes/repos/+page.svelte`:

```svelte
<script lang="ts">
  import type { PageData } from './$types';
  const { data } = $props();
</script>

<h1>Indexed Repos</h1>

{#if data.repos.length === 0}
  <p>No repos indexed yet. Run <code>sensei init</code> in your project.</p>
{:else}
  <table>
    <thead>
      <tr>
        <th>Name</th>
        <th>Stack</th>
        <th>Files</th>
        <th>Symbols</th>
        <th>Traceability</th>
        <th>Last Indexed</th>
      </tr>
    </thead>
    <tbody>
      {#each data.repos as repo}
        <tr>
          <td><a href="/repos/{repo.id}">{repo.name}</a></td>
          <td>{repo.stack.join(', ') || '—'}</td>
          <td>{repo.file_count}</td>
          <td>{repo.symbol_count}</td>
          <td>0 links</td>
          <td>{repo.last_indexed_at ? new Date(repo.last_indexed_at).toLocaleString() : 'Never'}</td>
        </tr>
      {/each}
    </tbody>
  </table>
{/if}
```

- [ ] **Step 4: Create Symbol Browser — repos/[id]/+page.server.ts**

Create `apps/dashboard/src/routes/repos/[id]/+page.server.ts` using the kavach Supabase client:

```typescript
import type { PageServerLoad } from "./$types";
import { error } from "@sveltejs/kit";

export const load: PageServerLoad = async (event) => {
  const client = event.locals.supabase;
  if (!client) throw error(503, "Supabase not configured");
  const { params } = event;

  const { data: repo } = await client
    .from("repos")
    .select("*")
    .eq("id", params.id)
    .single();

  if (!repo) throw error(404, "Repo not found");

  const { data: symbols } = await client
    .from("symbols")
    .select("name,kind,file_path,line_start,is_exported,signature")
    .eq("repo_id", params.id)
    .eq("is_exported", true)
    .order("file_path")
    .order("line_start")
    .limit(500);

  return { repo, symbols: symbols ?? [] };
};
```

- [ ] **Step 5: Create Symbol Browser — repos/[id]/+page.svelte**

Create `apps/dashboard/src/routes/repos/[id]/+page.svelte`:

```svelte
<script lang="ts">
  import type { PageData } from './$types';
  const { data } = $props();

  let query = $state('');
  const filtered = $derived(
    query.trim()
      ? data.symbols.filter(s =>
          s.name.toLowerCase().includes(query.toLowerCase()) ||
          s.file_path.toLowerCase().includes(query.toLowerCase())
        )
      : data.symbols
  );
</script>

<a href="/repos">← All Repos</a>
<h1>{data.repo.name}</h1>

<dl>
  <dt>Stack</dt><dd>{data.repo.stack.join(', ') || '—'}</dd>
  <dt>Last indexed</dt><dd>{data.repo.last_indexed_at ? new Date(data.repo.last_indexed_at).toLocaleString() : 'Never'}</dd>
  <dt>Traceability</dt><dd>0 links</dd>
</dl>

<h2>Symbols ({data.symbols.length})</h2>

<input
  type="search"
  placeholder="Filter by name or file..."
  bind:value={query}
/>

<table>
  <thead>
    <tr>
      <th>Name</th>
      <th>Kind</th>
      <th>File</th>
      <th>Line</th>
    </tr>
  </thead>
  <tbody>
    {#each filtered as sym}
      <tr>
        <td><code>{sym.name}</code></td>
        <td>{sym.kind}</td>
        <td>{sym.file_path}</td>
        <td>{sym.line_start}</td>
      </tr>
    {/each}
  </tbody>
</table>
```

- [ ] **Step 6: Start the dashboard and verify**

```bash
cd apps/dashboard && bun run dev
```

Open `http://localhost:3000/repos` in a browser.

Expected: Repo list with symbol count and file count. Click a repo → symbol browser with filter input.

- [ ] **Step 7: Commit**

```bash
git add apps/dashboard/src/routes/repos/
git commit -m "feat(dashboard): repo list + symbol browser reading from Supabase"
```

---

## Final Verification — Done When

- [ ] **Step 1: Run sensei init on the sensei repo itself**

```bash
cd /Users/Jerry/Developer/sensei && bun run packages/cli/src/cli.ts init
```

Enter `http://localhost:54321` and your service key. Verify:
- Indexed N files, M symbols (should be hundreds)
- `.sensei/config.yaml` created with `repo_id` and `supabase_url`
- `CLAUDE.md` updated
- `AGENTS.md` created

- [ ] **Step 2: Open dashboard and verify**

```bash
cd apps/dashboard && bun run dev
```

Navigate to `http://localhost:3000/repos`. Verify:
- Sensei repo appears in the list with file count > 0, symbol count > 0
- Click through to symbol browser — symbols visible with filter working
- "Traceability: 0 links" shown

- [ ] **Step 3: Test MCP search tool**

Start the MCP server in a separate terminal:

```bash
SENSEI_REPO_PATH=/Users/Jerry/Developer/sensei bun run packages/server/src/mcp-entry.ts
```

Send a test search call via stdin (MCP stdio protocol). The simplest test is to add a `SUPABASE_INTEGRATION=1` integration test:

Create `packages/server/src/tools/search.integration.spec.ts`:

```typescript
import { describe, it, expect } from "vitest";
import { createClient } from "@supabase/supabase-js";
import { search } from "./search.js";
import { loadSenseiConfig, loadCredentials } from "@sensei/shared";

const RUN = process.env.SUPABASE_INTEGRATION === "1";

describe.skipIf(!RUN)("search tool integration", () => {
  it("returns results for 'createClient' query", async () => {
    const config = await loadSenseiConfig(process.cwd());
    const creds = await loadCredentials();
    if (!config || !creds) throw new Error("No Supabase config");

    const client = createClient(config.supabase_url, creds.supabase_service_key, {
      db: { schema: "sensei" },
      auth: { persistSession: false },
    });

    const result = await search(client as any, config.repo_id, "createClient", 10);
    expect(result.total).toBeGreaterThan(0);
    expect(result.symbols.some(s => s.name.toLowerCase().includes("client"))).toBe(true);
  });
});
```

Run:
```bash
SUPABASE_INTEGRATION=1 cd packages/server && bunx vitest run src/tools/search.integration.spec.ts
```

Expected: At least 1 result for "createClient".

- [ ] **Step 4: Run all unit tests**

```bash
cd /Users/Jerry/Developer/sensei && bunx vitest run
```

Expected: All unit tests pass with no failures.

- [ ] **Step 5: Final commit**

```bash
git add packages/server/src/tools/search.integration.spec.ts
git commit -m "test(server): add search integration test for Phase 1 Done-When verification"
```

---

## Summary

**What Phase 1 builds:**

| Package | What's new |
|---|---|
| `supabase/migrations/` | `repos`, `symbols`, `call_edges`, `imports`, `scan_state`, `events` tables |
| `packages/shared` | `domain.ts` — Repo, RepoSymbol, ParsedFile, ScanResult, IndexResult types |
| `packages/engine` | New package: Scanner, TypeScriptAdapter (ts-morph), Indexer, indexRepo pipeline |
| `packages/server` | MCP server: `get_session_context`, `search`, `load_context` tools |
| `packages/cli` | Updated `init` command — Supabase registration, engine indexing, CLAUDE.md + AGENTS.md |
| `apps/dashboard` | Repo list + Symbol browser backed by Supabase |

**Phase 1 Done When:**
`sensei init` on the sensei repo → dashboard shows repo with file/symbol counts → `search` MCP tool returns results for "createClient" → "Traceability: 0 links" visible in dashboard.
