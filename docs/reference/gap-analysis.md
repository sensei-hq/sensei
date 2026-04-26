# Gap Analysis — sensei codebase

> Generated: 2026-04-10 | Covers: all packages + desktop app
> Purpose: Identify inconsistencies, broken logic, SaaS remnants, and architectural drift from the local-first roadmap

---

## Executive Summary

The codebase has drifted significantly from its roadmap. The pivot from multi-tenant SaaS (Supabase/pgvector) to local-first (SQLite/Kuzu) is partially complete — the graph indexer and daemon work, but **Supabase remnants are still wired into shared types, CLI commands, and config schemas**. Recent rapid feature additions introduced **duplicated logic, inconsistent patterns, and several runtime bugs**. There are **61 TypeScript errors** across CLI and server packages that need fixing before any new work.

### Critical Issues (must fix immediately)

| # | Issue | Location | Impact |
|---|-------|----------|--------|
| C1 | `getLibDocsTool` called with wrong arity — MCP tool broken at runtime | `server/mcp-server.ts:277` | Library docs MCP tool always fails |
| C2 | `fnIdByName` map built from callers only — CALLS edges silently dropped for leaf functions | `graph-indexer/indexer.ts:499-504` | Graph missing most call relationships |
| C3 | `activity-log.ts` passes `.run([...])` instead of `.run(...)` — 25 instances | `server/activity-log.ts` (25 lines) | All session/event writes may silently corrupt or fail |
| C4 | 61 TypeScript errors across CLI + server | See TypeScript section | CI cannot pass type checks |

---

## 1. SaaS Remnants (Local-First Pivot Incomplete)

The roadmap explicitly says "zero external dependencies — no Docker, no cloud accounts, no Supabase, no login." These items violate that:

### 1.1 Supabase Still in Shared Package

| File | What | Action |
|------|------|--------|
| `shared/src/supabase-client.ts` | Exports `makeSenseiClient` pulling in `@supabase/supabase-js` | Remove file, remove export from `shared/src/index.ts` |
| `shared/src/config.ts:18` | `supabase_url?: string` in `SenseiRepoConfig` | Remove field |
| `shared/src/config.ts:57` | `loadCredentials` reads `supabase_service_key` | Remove Supabase credential loading |
| `shared/src/types.ts:87-105` | `IndexConfig` with `backend: "ollama" \| "regex"`, `SetupStatus` with `ollamaBinary`/`onnxModel` | Remove stale types |
| `@supabase/supabase-js` in root deps | Pulled transitively into every package | Remove dependency |

### 1.2 CLI Login Flow to Non-Existent Platform

| File | What | Action |
|------|------|--------|
| `cli/src/commands/login.ts` | Full OAuth flow to `https://app.sensei.dev` (doesn't exist) | Remove file |
| `cli/src/cli.ts:436-450` | `login`, `logout`, `whoami` command dispatch | Remove commands |
| `cli/src/cli.ts:58` | `login`/`logout` in `GLOBAL_CMDS` | Remove entries |

### 1.3 Dead e2e Test Files (Supabase-Coupled)

All five files should be deleted — they import from packages that no longer exist, use Supabase clients, and contain hardcoded JWTs:

- `server/src/e2e-collector.ts` — imports from non-existent `packages/collector`
- `server/src/e2e-context.ts`
- `server/src/e2e-coaching.ts`
- `server/src/e2e-index.ts`
- `server/src/e2e-session.ts`

### 1.4 CLI Help Text References Supabase

| File | Line | What | Action |
|------|------|------|--------|
| `cli/src/cli.ts:87` | HELP text | "Index custom_libs into Supabase" | Fix to "Index custom_libs locally" |
| `cli/src/cli.ts:88-89` | HELP text | `--global` "Promote lib to shared pool" | Remove — feature doesn't exist |

---

## 2. Duplicated Logic

### 2.1 graph-indexer: indexer.ts vs watcher.ts (6 duplications)

These two files share 150+ lines of identical code that should be in a shared module:

| What | indexer.ts | watcher.ts | Delta |
|------|-----------|------------|-------|
| `ManifestEntry`/`Manifest` types | lines 49-50 | lines 49-54 | Identical |
| `manifestPath()` | line 52 | line 56 | Identical |
| `loadManifest()` | lines 56-62 | lines 60-67 | Identical |
| `saveManifest()` | lines 64-68 | lines 69-73 | **Minified vs pretty-printed JSON** — writes to same file |
| `fileHash()` | lines 70-73 | lines 75-78 | Identical |
| `buildAdapterMap()` | lines 361-368 | lines 101-108 | Identical except variable naming |
| `DEFAULT_EXCLUDE` patterns | lines 378-386 (inline) | lines 85-93 (named constant) | Same values, different structure |
| `isDocFile()` | line 547 (inline glob) | line 95 (function) | Same logic, different form |

**Action:** Extract to `shared.ts`: types, manifest helpers, adapter map builder, default patterns.

### 2.2 engine: ACP Adapter Copy-Paste (5x duplication)

| Method | Files with identical body |
|--------|--------------------------|
| `writeSkills()` | windsurf, zed, kiro, opencode (byte-for-byte identical); cursor differs only in `.mdc` extension |
| `writeLibSkill()` | windsurf, zed, kiro, opencode (identical); cursor `.mdc`; claude slightly different format |
| `installedSkills()` | windsurf, zed, kiro, opencode (identical); cursor `.mdc` filter |
| `registerMcp()` | cursor, windsurf, kiro (identical JSON config pattern) |

**Action:** Extract shared methods to a `BaseAcpAdapter` class or utility module. Adapters only override the file extension and path.

### 2.3 server: Graph Stats Queried in 3 Places

`serve.ts:buildGraphData`, `get-session-context.ts`, and `get-bearings.ts` all open Kuzu, run `COUNT(*)` on Functions, and close. Should be one shared `getGraphStats(repoId)` utility.

### 2.4 server: `config?.repo_id ?? repoId` Repeated in 7 Tool Files

`search.ts`, `context-pack.ts`, `recommend-next.ts`, `get-complexity.ts`, `load-context.ts`, `install-skills.ts`, `index-repo.ts` all load config and derive project the same way. Extract to `resolveProject(repoPath, repoId)`.

### 2.5 engine/lib: Duplicate Helpers

| Function | File 1 | File 2 |
|----------|--------|--------|
| `extractH1()` | `http-adapter.ts:119` | `github-adapter.ts:104` |
| `inferComponent()` | `http-adapter.ts:123` | `github-adapter.ts:113` |
| `TurndownService` singleton | `http-adapter.ts:11` | `doc-utils.ts:7` |

**Action:** Move to `doc-utils.ts` as shared exports.

---

## 3. Broken/Incomplete Logic

### 3.1 Runtime Bugs

| # | Bug | File:Line | Impact |
|---|-----|-----------|--------|
| B1 | `getLibDocsTool` called with 5 args, signature accepts 3 | `mcp-server.ts:277` | MCP lib docs tool always fails |
| B2 | `fnIdByName` built from `callerName -> callerId`; lookup uses `calleeName` | `indexer.ts:499-504` | CALLS edges missing for leaf functions |
| B3 | [RESOLVED — SQLite removed] `activity-log.ts` `.run([values])` wraps args in extra array (25x) | `activity-log.ts` throughout | SQLite queries get wrong params |
| B4 | `record-memory.ts` returns fake UUID for decisions/patterns | `record-memory.ts:25,37` | Returned IDs are meaningless |
| B5 | `clearProgress` writes empty string `""` — JSON parse error for readers | `indexer.ts:90` | Desktop progress poll crashes |
| B6 | `escapeCypher` in doc-indexer only escapes `\` and `'`, missing `\n\r\t` | `doc-indexer.ts:135` | Corrupted Cypher on multiline titles |
| B7 | `supersedes` paths always resolve relative to doc dir, not repo root | `doc-indexer.ts:280` | SUPERSEDES edges silently not created |
| B8 | `edgesCreated++` incremented even when `mergeDocEdge` fails | `doc-indexer.ts:253-255` | Inflated edge count reported |

### 3.2 Incomplete Endpoints

| Endpoint | File | Issue |
|----------|------|-------|
| `POST /stop` | `serve.ts:487` | Doesn't call `pool.stop()` or `queue.close()` — unclean shutdown |
| `GET /api/drift` | `serve.ts:319` | Returns 200 with error body instead of 400 |
| `POST /api/index` | `serve.ts:376` | No validation that path exists on disk |
| `GET /api/sessions` | `serve.ts:299` | `toolUsage: []` and `benchmarkPairs: []` always empty |
| `POST /api/mcp` | `serve.ts:330-364` | Manually duplicates MCP dispatch (subset of tools, different `search` signature) |
| OTLP endpoint | `otlp-endpoint.ts:109` | Events parsed but never persisted (no-op in non-dry-run mode) |

### 3.3 Schema Issues

| Schema Issue | File:Line |
|-------------|-----------|
| `USES_TYPE` relationship declared but never populated — `layers.ts` queries it, always empty | `schema.ts:120`, `layers.ts:116-133` |
| `complexity` column added via ALTER instead of in CREATE TABLE | `schema.ts:134-137` |
| No index on `Function.name` or `Type.name` (high-frequency lookups) | `schema.ts` (absent) |

---

## 4. Inconsistent Patterns

### 4.1 CORS Headers Missing on Some Endpoints

`serve.ts` uses `jsonResponse()` (with CORS) for `/api/*` routes but raw `Response.json()` (no CORS) for:
- `/analyze`
- `/setup/status`
- `/setup/ollama`
- `/stop`
- `/health`

Desktop app cross-origin requests to these endpoints will be blocked by the browser.

### 4.2 Database Handle Leaks

`get-symbol-graph.ts:32` and `search-code-graph.ts:17` destructure only `{ conn }` from `getOrCreateDb` — `db.close()` is never called, leaking the database handle. All other tool files properly close both `db` and `conn`.

### 4.3 Cypher Injection

`serve.ts:57-112`, `get-bearings.ts`, and `get-complexity.ts` interpolate `repoId` directly into Cypher queries without escaping. `load-context.ts` at least escapes with `.replace(/'/g, "\\'")`.

### 4.4 Error Handling Asymmetry

| Operation | indexer.ts | watcher.ts |
|-----------|-----------|------------|
| `deleteFileFromGraph` fails | `.catch(() => {})` — swallowed | No catch — throws, aborts entire rescan |
| File indexing fails | Logged to `index-errors.json` | Silently swallowed |
| Doc indexing fails | Silently swallowed (no logging) | Silently swallowed |

### 4.5 Glob-to-Regex Hand-Roll

`watcher.ts:221-228` converts globs via `p.replace(/\*\*/g, ".*").replace(/\*/g, "[^/]*")`. This is broken for brace expansion, character classes, negation, and anchoring. Should use `micromatch` (already available as fast-glob dependency).

---

## 5. ACP Adapter Interface Gaps

### 5.1 Interface vs Implementation Mismatch

| Method on `AcpAdapter` interface | Implemented by |
|----------------------------------|---------------|
| `writeSkills()` | All 6 adapters |
| `writeLibSkill()` | All 6 adapters **but not declared on interface** |
| `installedSkills()` | All 6 adapters |
| `registerMcp()` | Claude, Cursor, Windsurf, Kiro |
| `writeRules?()` | Only Cursor, Windsurf |
| `installPlugin?()` | Only Claude |
| `injectSettings?()` | Only Claude — **other adapters never get OTLP endpoint** |
| `installHooks?()` | **None** — declared but never implemented |
| `registerCommands?()` | **None** — declared but never implemented |

### 5.2 Agent ID Hardcoded to 'claude'

`shared/src/types.ts:128`: `AgentSkillsManifest.agent` is typed as literal `'claude'`. CLI's `setup.ts:107` writes `agent: 'claude'` regardless of which ACP was used.

---

## 6. Desktop App Issues

### 6.1 Non-Functional UI Elements

| Page | Element | Issue |
|------|---------|-------|
| Ideas | "New idea" button | No `onclick` handler |
| Ideas | "Graduate to repo" button | No handler |
| Ideas | Prompt bar input | No submit handler |
| Libraries | MCP Explorer | Posts to `/api/mcp` which doesn't exist on daemon |
| Sessions | Analytics view | `data.stats.totalSessions` accessed when `data.stats` is `null` — runtime crash |
| Sessions | Session cards | `s.cost.toFixed(2)` and `s.tokens.in` — throws if undefined |

### 6.2 Server Route Stubs

All four `(server)/api/` routes return empty/stub data:
- `api/projects/+server.ts` → `[]`
- `api/sessions/+server.ts` → `{ stats: null, sessions: [] }`
- `api/graph/+server.ts` → empty graph
- `api/ideas/+server.ts` → `[]`

These exist as fallbacks when daemon isn't running but contain no actual implementation.

### 6.3 State Management Inconsistencies

| Issue | Location |
|-------|----------|
| Graph page reads port at module eval time (not reactive) | `graph/+page.svelte:8` |
| `repoId` missing from localStorage-scanned projects | `projects/+page.ts:66-73` |
| `triggerAllUnindexed` derives `repoId` from path (clashes with config.yaml) | `(app)/+layout.svelte:68` |
| Version hardcoded as `'0.1.0'` | `settings/+page.svelte:27` |
| Setup page has `prerender = true` contradicting root `prerender = false` | `setup/+page.ts:1` |

---

## 7. TypeScript Health

### 7.1 Error Summary (61 errors total)

| Package | Errors | Root Cause |
|---------|--------|------------|
| `packages/engine` | 0 | Clean |
| `packages/graph-indexer` | 0 | Clean |
| `packages/server` | 28 | `.run([...])` pattern (25), `getLibDocsTool` arity (1), `query<>` generic (1), spec missing field (1) |
| `packages/cli` | 33 | Missing `"types": ["bun"]` in tsconfig (28), stubs not excluded (4), register.ts (1) |

### 7.2 tsconfig Inconsistencies

| Issue | Package |
|-------|---------|
| Missing `"types": ["bun"]` | CLI (server has it) |
| Missing `@types/bun` devDep | CLI (server has it) |
| `__stubs__` not excluded | CLI (server excludes them) |
| Stale `@supabase+supabase-js@2.99.1` path | CLI (root has `2.101.1`) |
| `"types": "./src/index.ts"` points to source | graph-indexer (works under bundler resolution, fragile) |

---

## 8. Dead Code / Stale Features

| What | Location | Action |
|------|----------|--------|
| 5 e2e test files (Supabase-coupled) | `server/src/e2e-*.ts` | Delete |
| `login`/`logout`/`whoami` commands | `cli/src/commands/login.ts` | Delete |
| `migrate` command (one-time, obsolete) | `cli/src/commands/migrate.ts` | Delete or hide |
| `reformat` undocumented alias for `doctor` | `cli/src/cli.ts:225` | Remove alias |
| `--drift` flag on `watch` (silently ignored) | `cli/src/commands/watch.ts:8` | Remove or implement |
| `--hooks` flag (defined, never used) | `cli/src/cli.ts:37` | Remove |
| [RESOLVED] `better-sqlite3` in server deps (not imported) | `server/package.json` | Remove dep |
| `mergeHooks` exported from `acp-utils.ts` | `engine/src/agent/acp-utils.ts:49` | Remove (dead import in claude-adapter) |
| 3 uncalled methods in `ActivityLog` | `server/activity-log.ts:517-557` | Remove or wire up |
| `installHooks()` / `registerCommands()` on interface | `engine/src/agent/acp-adapter.ts:32,41` | Remove (no implementations exist) |

---

## 9. Remediation Plan

### Phase A — Stabilize (fix broken things, no new features)

**A1. Fix TypeScript errors (61 errors → 0)**
- Fix `activity-log.ts` `.run([...])` → `.run(...)` (25 instances)
- Fix `mcp-server.ts:277` `getLibDocsTool` call signature
- Add `"types": ["bun"]` + `@types/bun` to CLI tsconfig
- Add `"exclude": ["src/__stubs__"]` to CLI tsconfig
- Fix `index-queue.ts` query generic
- Fix `lib-indexer.spec.ts` missing `sourceType`

**A2. Fix critical runtime bugs**
- Fix `fnIdByName` in `indexer.ts:499-504` — build map from all function nodes, not just caller edges
- Fix `clearProgress` to write `{}` or delete file instead of empty string
- Fix `escapeCypher` in `doc-indexer.ts` to match `indexer.ts` version
- Fix `supersedes` path resolution in `doc-indexer.ts:280`
- Fix `POST /stop` to call `pool.stop()` and `queue.close()`
- Fix CORS: use `jsonResponse()` for all endpoints, not just `/api/*`
- Fix db handle leaks in `get-symbol-graph.ts` and `search-code-graph.ts`

**A3. Fix desktop runtime crashes**
- Add null guards for `data.stats` in sessions page
- Add null guards for `s.cost`/`s.tokens` in session cards
- Remove or stub the MCP Explorer in libraries page (endpoint doesn't exist)

### Phase B — Clean Up (remove dead weight)

**B1. Delete SaaS remnants**
- Delete all 5 `e2e-*.ts` files
- Delete `commands/login.ts` and remove `login`/`logout`/`whoami` dispatch
- Remove `supabase-client.ts` and its export from `shared/index.ts`
- Remove `supabase_url` from `SenseiRepoConfig`
- Remove `supabase_service_key` from `loadCredentials`
- Remove stale types: `IndexConfig`, `SetupStatus`
- [RESOLVED] Remove `better-sqlite3` from server deps
- Fix CLI help text referencing Supabase

**B2. Delete dead code**
- Remove `migrate` command (or hide behind `--legacy` flag)
- Remove `reformat` alias
- Remove `--drift` and `--hooks` unused flags
- Remove `mergeHooks` from `acp-utils.ts` and dead import from `claude-adapter.ts`
- Remove `installHooks()`/`registerCommands()` from `AcpAdapter` interface
- Remove uncalled `ActivityLog` methods or wire them to endpoints

**B3. Fix stale references**
- `init.ts:175`: `apps/dashboard` → `apps/desktop`
- `get-bearings.ts:102`: Remove hardcoded "TypeScript monorepo"
- `settings/+page.svelte:27`: Read version from package.json at build time

### Phase C — Consolidate (reduce duplication, enforce patterns)

**C1. Extract graph-indexer shared module**
Create `packages/graph-indexer/src/shared.ts`:
- `ManifestEntry`, `Manifest` types
- `manifestPath()`, `loadManifest()`, `saveManifest()`, `fileHash()`
- `buildAdapterMap()`
- `DEFAULT_INCLUDE`, `DEFAULT_EXCLUDE`, `isDocFile()`
- `escapeCypherStr()` (one version, used by both indexer and doc-indexer)

**C2. Extract ACP adapter base class**
Create `packages/engine/src/agent/base-adapter.ts`:
- `writeSkills()` with configurable extension parameter
- `writeLibSkill()` with configurable extension
- `installedSkills()` with configurable filter
- `registerMcpInJsonFile()` shared helper
- Add `writeLibSkill()` to `AcpAdapter` interface
- Make `agent` field in `AgentSkillsManifest` a string union, not literal `'claude'`

**C3. Extract server shared helpers**
- `getGraphStats(repoId)` — used by `serve.ts`, `get-session-context.ts`, `get-bearings.ts`
- `resolveProject(repoPath, repoId)` — used by 7 tool files
- `escapeCypherValue(str)` — used by all Cypher-interpolating tools
- `openGraph(repoId)` → returns `{ db, conn, [Symbol.asyncDispose] }` using `using` pattern to prevent handle leaks

**C4. Fix watcher glob matching**
Replace hand-rolled regex in `watcher.ts:221-228` with `micromatch.isMatch()`.

**C5. Add doc files to manifest in full indexer**
`indexer.ts` Pass 4 should check manifest for `.md`/`.mdx` files, matching watcher behavior.

### Phase D — Schema & Interface Alignment

**D1. Clean up Kuzu schema**
- Move `complexity` column into original `CREATE NODE TABLE Function` DDL
- Remove `USES_TYPE` from schema (or implement population in indexer)
- Add secondary indexes on `Function.name`, `Type.name`, `Function.project`

**D2. Align `AcpAdapter` interface with reality**
- Add `writeLibSkill()` to interface
- Remove `installHooks()` and `registerCommands()` (no implementations)
- Document which optional methods each adapter supports
- Ensure `injectSettings()` has a base implementation or is called for all adapters that need OTLP

**D3. Type Kuzu query results**
Create typed row interfaces for each query pattern instead of `Record<string, KuzuValue>` with string casts. One typed `typedQuery<T>()` wrapper eliminates 20+ `as QueryResult` casts.

---

## 10. Recommended Execution Order

```
Week 1: Phase A (stabilize)
  A1 — Fix all 61 TS errors                    [4h]
  A2 — Fix critical runtime bugs               [4h]
  A3 — Fix desktop crashes                     [2h]

Week 2: Phase B (clean up)
  B1 — Delete SaaS remnants                    [2h]
  B2 — Delete dead code                        [1h]
  B3 — Fix stale references                    [1h]

Week 3: Phase C (consolidate)
  C1 — graph-indexer shared module             [3h]
  C2 — ACP adapter base class                  [3h]
  C3 — server shared helpers                   [2h]
  C4 — watcher glob fix                        [1h]
  C5 — doc manifest in full indexer            [1h]

Week 4: Phase D (schema & interfaces)
  D1 — Kuzu schema cleanup                    [2h]
  D2 — AcpAdapter interface alignment          [2h]
  D3 — Typed Kuzu queries                      [3h]
```

After Phase D, the codebase will be in a consistent state with zero TS errors, no SaaS remnants, no duplicated logic, and aligned interfaces — ready for new roadmap features.
