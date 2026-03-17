---
id: gaps
type: design
created: 2026-03-17
---

# Design vs Implementation Gaps

This document records discrepancies between the design docs in `docs/design/` and the current implementation. Organized by severity and category.

---

## 1. Local Files Still Used Instead of Supabase DB

These are the highest-priority gaps. The design goal is Supabase as single source of truth, but local JSON files are still written and/or read in several places.

### 1a. `chunks.json` and `embeddings.json` written to disk

**File:** `packages/tools/src/tools/chunker.ts:265-285`

When `buildChunksAndEmbeddings()` runs without a DB client (`useDb = false`), it falls back to writing `chunks.json` and `embeddings.json` to `.sensei/`. The code comment says "tests and offline mode" but there is no offline-mode use case in the design — the design requires Supabase.

**Impact:** `sensei reindex` run without Supabase config writes stale local files. `search.ts` reads these as fallback (see below), creating a split-brain situation.

**Fix:** Remove the file-write path. If `useDb = false`, either error out with a helpful message ("run `sensei init` to set up Supabase") or silently skip (no local fallback).

---

### 1b. `chunks.json` and `embeddings.json` read as fallback in search

**File:** `packages/tools/src/tools/search.ts:127-151` (`loadChunks`, `loadEmbeddings`)

After checking DB, search falls back to reading `chunks.json` and `embeddings.json` from `.sensei/`. These files are created by the write path above (1a) and by the test harness.

**Impact:** Repos that were indexed before Supabase was set up may return stale results from local files, silently bypassing DB.

**Fix:** Remove file-based fallback from `loadChunks` and `loadEmbeddings`. If DB returns nothing, return null (triggers zero-hit reindex guard).

---

### 1c. `symbol-map.json` and `doc-index.json` read as fallback in reindex

**File:** `packages/tools/src/tools/reindex.ts:96-109`

After loading from DB, `reindexRepo` falls back to `symbol-map.json` and `doc-index.json` when DB has nothing. Comment says "Fallback to legacy files if DB had nothing."

**Impact:** On first `sensei index` after migrating to Supabase, old local files may inject stale symbols into the index.

**Fix:** Remove the file fallback. If DB has nothing, do a full scan (`force = true`). This is the correct behavior anyway.

---

### 1d. `doc-index.json` still written to disk

**File:** `packages/tools/src/tools/reindex.ts` (near end of function — writes doc-index to `.sensei/`)

Doc fingerprints (mtime/size per file) are written to `.sensei/doc-index.json` and also stored in `repos.doc_fingerprints` in Supabase. The DB path is correct; the file write is dead weight.

**Fix:** Remove the `writeFile` call for `doc-index.json`. Read fingerprints from DB only.

---

## 2. Two MCP Servers — Confusion and Wrong Default

### 2a. `packages/mcp/` is a bare, uninstrumented server

**Files:** `packages/mcp/src/index.ts`

This package exposes MCP tools (search, load_context, etc.) but has no session tracking, no `beat()` wrapper, no FTR, no analytics. It reads from local files only — no Supabase client. It was the original implementation.

**Status:** Superseded by `packages/server/src/mcp-entry.ts`, which wraps every tool with `beat()` and uses Supabase.

**`~/.claude/mcp.json` now correctly points to `packages/server/src/mcp-entry.ts`** (fixed this session). But `packages/mcp/` still exists and could confuse contributors.

**Fix options:**
- Archive/delete `packages/mcp/`
- Or add a deprecation notice in its README pointing to `packages/server/`

---

### 2b. `setupMcp()` was pointing to wrong server

**Status: Fixed this session.** `packages/cli/src/commands/setup.ts` now resolves `packages/server/src/mcp-entry.ts` and writes `SENSEI_REPO_PATH` env var to `~/.claude/mcp.json`.

---

## 3. Design Docs With No Implementation

These docs describe features that have not been built.

### 3a. Pattern Store (`docs/design/17-pattern-store.md`)

Describes automatic pattern detection during indexing, persisted to `.sensei/patterns.md`, with `search_patterns()` and `capture_pattern()` MCP tools.

**Implementation:** None. No `patterns.md`, no DB table, no MCP tools for patterns.

**Decision needed:** Build it, defer it, or mark as future/backlog in the design doc.

---

### 3b. Response Cache (`docs/design/18-response-cache.md`)

Describes persisting Claude outputs to Supabase with semantic tags, surfaced by `get_session_context()` as hints.

**Implementation:** None. `get_session_context()` does not surface cached responses.

**Decision needed:** Build it, defer it, or mark as future/backlog.

---

### 3c. Context Manager (`docs/design/19-context-manager.md`)

Describes a `ContextManager` that maintains a budget-aware in-memory context graph, exposes `context_pack()` and `load_context()` with priority queuing.

**Implementation:** `context_pack()` and `load_context()` exist in `packages/server/src/mcp-server.ts` but without the budget-aware graph described in the design.

**Decision needed:** The simpler implementation may be sufficient. If so, update the design doc to match actual behavior.

---

### 3d. Pipeline Adapter (`docs/design/20-pipeline-adapter.md`)

Describes an abstract `PipelineAdapter` with pluggable backends (local file, Supabase, cloud).

**Implementation:** None as a formal abstraction. The code uses ad-hoc `if (useDb) { ... } else { ... }` patterns in `chunker.ts`, `reindex.ts`, `search.ts`.

**Decision needed:** The ad-hoc approach is simpler. If the adapter abstraction adds value, build it; otherwise update the doc to reflect the simpler design.

---

## 4. Design Docs Diverged From Implementation

These docs describe something that was built but has since changed.

### 4a. MCP server design (`docs/design/03-mcp-server.md`)

Describes tool contracts for the bare `packages/mcp/` server. The actual MCP server is `packages/server/src/mcp-server.ts`, which has a different tool set, session tracking, and Supabase integration.

**Fix:** Rewrite `03-mcp-server.md` to reflect `mcp-server.ts` actual tool contracts, or update `docs/design/40-mcp-tool-contracts.md`.

---

### 4b. Server package (`docs/design/14-server-package.md`)

Already self-annotated as `SUPERSEDED` at the top. The deployment model (local HTTP inference server) was abandoned in favor of Supabase + local embeddings.

**Fix:** Move to `docs/design/archive/` or replace with current architecture description.

---

### 4c. Package adapters (`docs/design/15-package-adapters.md`)

Describes `@sensei/adapters` package with pluggable LLM backends.

**Implementation:** `packages/server/src/model/` has `ClaudeBackend` and `OllamaBackend` directly in the server package — no separate `@sensei/adapters` package.

**Fix:** Update doc to reflect actual model directory structure, or delete if it adds no value.

---

### 4d. Traceability matrix (`docs/design/13-traceability-matrix.md`)

Design describes traceability linking design doc sections to feature implementations. The `traceability.json` file exists in `.sensei/` and is auto-generated by the indexer.

**Gap:** `.sensei/traceability.json` is present but the dashboard has no page to view it. The design doc implies it should be queryable.

**Fix:** Add traceability viewer page to dashboard, or surface it through `get_session_context()`.

---

## 5. Missing Dashboard Pages

The dashboard has repos, sessions, analytics, context-packs, agents, libraries, simulate, and drift pages. The following are referenced in design but missing:

| Missing Page | Design Reference | Notes |
|---|---|---|
| Symbol Browser | `05-indexing.md` | Browse symbols indexed from a repo |
| Traceability Viewer | `13-traceability-matrix.md` | Show design ↔ code coverage |
| FTR / Quality Metrics | `architecture.md`, `cc-rlm.md` | First-Try-Right score trends |
| Pattern Store | `17-pattern-store.md` | Browse captured patterns |

---

## 6. Partially Implemented Features

### 6a. FTR (First-Try-Right) score

**Design:** `checkpoint()` computes FTR from snapshot count, tool error rate, and clean completion. Score is persisted and shown in dashboard.

**Implementation:** `checkpoint()` tool in `mcp-server.ts` writes `outcome` to session row. FTR computation exists as `computeFtr()` in `packages/server/src/tools/`. Analytics page shows some session data.

**Gap:** FTR trend visualization is incomplete. The "quality" coaching feedback described in the design (surfaced at session start for low-FTR patterns) is not implemented.

---

### 6b. Session heartbeat

**Design:** `beat()` fires on every tool call to keep sessions alive and record turn count.

**Implementation:** `beat()` is defined in `packages/server/src/tools/beat.ts` and called in `mcp-entry.ts`'s tool wrappers.

**Gap:** `updateHeartbeat` DB call may not be updating `active_sessions` view correctly — the Sessions page in the dashboard shows empty. (The root cause was the wrong MCP server being registered, now fixed. Needs verification after MCP fix.)

---

### 6c. Compression / llms.txt generation

**Design (`docs/design/06-compression.md`):** `llms.txt` auto-generated from indexed symbols + doc summaries, exposed via `GET /llms.txt`.

**Implementation:** `generateLlmsTxt()` exists in `packages/tools/src/tools/llms-txt.ts` and is called by `reindexRepo`. Output is written to `.sensei/llms.txt`.

**Gap:** No HTTP endpoint to serve it. The design mentions `GET /llms.txt` from the serve command — not implemented.

---

## Summary Table

| Gap | Severity | Effort | Fix Type |
|-----|----------|--------|----------|
| chunks.json / embeddings.json write path | High | Small | ✅ Removed (chunker.ts returns early without DB) |
| chunks.json / embeddings.json read fallback in search | High | Small | ✅ Removed (search.ts returns null without DB) |
| symbol-map.json read fallback in reindex | Medium | Small | ✅ Removed (reindex.ts does full scan without DB) |
| doc-index.json write to disk | Medium | Trivial | ✅ Was never written; only read as legacy fallback (also removed) |
| packages/mcp/ bare server still exists | Low | Trivial | ✅ README deprecation notice added |
| Pattern Store — not built | Low | Large | ✅ Marked not-implemented/backlog in design doc |
| Response Cache — not built | Low | Large | ✅ Marked not-implemented/backlog in design doc |
| Context Manager diverged from design | Low | Small | ✅ Partial-implementation notice added |
| Pipeline Adapter — not built | Low | Medium | ✅ Not-implemented notice added |
| 03-mcp-server.md outdated | Medium | Small | ✅ Superseded notice + frontmatter updated |
| 14-server-package.md superseded | Low | Trivial | ✅ Frontmatter status updated |
| 15-package-adapters.md outdated | Low | Trivial | ✅ Frontmatter status updated |
| Traceability viewer page missing | Medium | Medium | Dashboard page |
| Symbol Browser page missing | Low | Medium | Dashboard page |
| FTR coaching feedback | Low | Medium | mcp-server.ts + dashboard |
| llms.txt HTTP endpoint | Low | Small | serve.ts route |
