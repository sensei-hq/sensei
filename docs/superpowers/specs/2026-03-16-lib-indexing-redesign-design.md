# Library Indexing Redesign — Design Spec

**Date:** 2026-03-16
**Status:** Approved

---

## Goal

Remove Ollama dependency from library indexing. Split indexing into two independent phases so the dashboard works fully offline/disconnected. Add GitHub folder support as a first-class source type alongside the existing local folder adapter.

---

## Context

Currently, indexing a library requires a running Ollama instance with `nomic-embed-text` pulled. This blocks dashboard use in any environment without a local Ollama server. Ollama is only used for generating embedding vectors — not for fetching, parsing, or summarising content. Everything else (fetch, parse, HTML→Markdown, section splitting) is pure JS.

---

## Source Types

| Type | Value | Who can index | Example |
|------|-------|---------------|---------|
| llms.txt | `'llms.txt'` | Dashboard, CLI | `https://kavach.dev/llms.txt` |
| HTTP | `'http'` | Dashboard, CLI | `https://docs.example.com/api` |
| GitHub folder | `'github'` | Dashboard, CLI | `https://github.com/org/repo/tree/main/docs` |
| Local folder | `'local'` | **CLI only** | `/Users/jerry/projects/mylib/docs` |

**Local vs public distinction:** Derived from `source_type`. Any lib with `source_type = 'local'` is CLI-only — the dashboard shows "Re-index via CLI" instead of an active Re-index button. No separate column needed.

---

## Two-Phase Architecture

### Phase 1 — Fetch (immediate, no AI)

Runs on every "Add Library" or "Re-index" action from the dashboard.

1. Detect `source_type` from the URL/path input
2. Run the appropriate adapter → `DocPage[]`
3. Delete existing sections for this lib
4. Insert rows into `shared_lib_sections` — all fields **except** `embedding` (stored as NULL)
5. Update `shared_libs`: `index_status = 'ready'`, `section_count`, `indexed_at`

Sections are immediately browseable. Keyword search (ILIKE) works for Simulate and `getLibDocsTool` fallback.

### Phase 2 — Embed (optional, separate trigger)

Runs from the dashboard ("Build Index" button on library detail page) or `sensei embed-libs` CLI command. Uses Transformers.js — no server required.

1. Read all sections for this lib that have `embedding IS NULL`
2. For each section: `embed(description)` via `@xenova/transformers` (`all-MiniLM-L6-v2`, 384-dim)
3. Batch-update `embedding` column
4. Update `shared_libs`: `embed_status = 'ready'`

### Query Fallback

`getLibDocsTool` with a query string:
- If sections have embeddings → vector similarity search (`match_shared_lib_sections` RPC)
- If embeddings are NULL → ILIKE keyword fallback on `title || description`
- Simulate action on dashboard → always uses ILIKE (unchanged)

---

## GitHub Adapter

Handles URLs matching `https://github.com/{owner}/{repo}/tree/{branch}/{path}`.

**Steps:**
1. Parse URL → `{ owner, repo, branch, basePath }`
2. Call GitHub Trees API: `GET https://api.github.com/repos/{owner}/{repo}/git/trees/{branch}?recursive=1`
3. Filter: `blob` entries whose path starts with `basePath` and ends with `.md`
4. For each file:
   - Fetch raw content: `https://raw.githubusercontent.com/{owner}/{repo}/{branch}/{filePath}`
   - Parse: title = first H1 heading or filename stem, description = first non-heading paragraph (up to 200 chars), component = immediate parent directory name (if not `basePath` itself)
   - `url` = `https://github.com/{owner}/{repo}/blob/{branch}/{filePath}`
5. Returns `DocPage[]` with `sourceType: 'github'`

**Rate limits:** Unauthenticated GitHub API = 60 req/hour. Optional `GITHUB_TOKEN` env var support is a **future TODO**.

---

## Transformers.js Backend

New `TransformersBackend` class in `packages/engine` (or `packages/server`), implementing `ModelBackend.embed()` only.

- Package: `@xenova/transformers`
- Model: `Xenova/all-MiniLM-L6-v2` (384-dim, ~23MB ONNX, downloads on first use)
- Lazy-loaded (import on first call)
- Works in Node.js only (not browser — model download not suitable for web)
- Replaces `OllamaBackend` for the embed phase

---

## Database Changes

### `shared_lib_sections`
- Change `embedding vector(768)` → `embedding vector(384)`
- Change IVFFlat index to match new dimension
- Clear any existing embeddings (Ollama 768-dim vectors are incompatible)

### `shared_libs`
- Add `embed_status text CHECK (embed_status IN ('pending', 'embedding', 'ready'))` — nullable (NULL = never embedded)
- Add `'github'` to `source_type` CHECK constraint

### `match_shared_lib_sections` RPC
- Update `query_embedding vector(768)` → `query_embedding vector(384)`

---

## Dashboard UI Changes

**Add Library sidebar:**
- Remove local path input (web can't scan filesystem)
- `inferSourceType` detects github.com/*/tree/* URLs → `'github'`

**Library detail page (`/libraries/[id]`):**
- Re-index button: disabled (with tooltip "Re-index via CLI") when `source_type = 'local'`
- New "Build Index" button: triggers Phase 2 embed action; shows `embed_status` badge
- Stat row: add embed status indicator

---

## CLI Changes

- `sensei index` / `sensei update-registry`: Phase 1 only by default
- Add `--embed` flag (or separate `sensei embed-libs [name]` command): triggers Phase 2
- Local folder libs: continue to work as before via CLI

---

## What Does NOT Change

- `LlmsTxtAdapter`, `HttpAdapter`, `LocalAdapter` — unchanged interfaces
- `getLibDocsTool` query path — same external behaviour (semantic or keyword)
- `simulate` action — unchanged (ILIKE always)
- `lib_queries` logging — unchanged
- Skill generation — uses Claude API, unaffected
- Per-repo `lib_doc_sections` — out of scope (this redesign targets shared libs only)
