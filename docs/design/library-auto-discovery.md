---
type: design
status: draft
date: 2026-08-18
---

# Library Auto-Discovery — module

How detected project dependencies automatically get their documentation, skills,
agents, and MCP tools indexed and available — without the user calling
`add_library`.

Pairs with [`phases.md`](phases.md) §3.3 (the Phase 1 story that ships this)
and the existing library system in `crates/senseid/src/libraries/`.

---

## 1. The gap

The library system has every piece — it just doesn't connect them automatically:

```
EXISTS (auto)                EXISTS (manual)           MISSING
─────────────                ───────────────           ───────
extract_deps detects    →    add_library fetches    →  (nothing connects them)
  project_libraries           llms.txt + stores
  from manifests              library_pages
```

A project with 50 npm dependencies has 50 libraries detected. None have docs
indexed until the user manually calls `add_library` 50 times. The AI assistant
doesn't know what libraries are available or what their APIs look like.

---

## 2. End-to-end flow

### 2.1 Detection → auto-index trigger

The `resolve_libs` barrier task runs after every scan reconcile. It calls
`extract_deps` which upserts into `sensei.libraries` + `sensei.referenced_libraries`
+ `sensei.project_libraries`.

**P1.7 adds one step** at the end of `extract_deps`: for each library where
`indexed_at IS NULL` (docs were never fetched), enqueue an `IndexLibrary` task
with `source = auto-detect`.

```
extract_deps
    │
    ├─ upsert_library("rokkit", "npm", "2.1.0", kind=detected)
    │   → indexed_at IS NULL  →  enqueue IndexLibrary
    │
    ├─ upsert_library("react", "npm", "19.0.0", kind=detected)
    │   → indexed_at IS NULL  →  enqueue IndexLibrary
    │
    └─ upsert_library("typescript", "npm", "5.5.0", kind=detected)
        → indexed_at IS NOT NULL (already indexed)  →  skip
```

### 2.2 Auto-index task

The existing `IndexLibrary` task handler (`tasks/handlers/libraries.rs`) already
does: detect source → fetch → parse → store pages → stamp version. P1.7 wires
the trigger; the task itself is unchanged.

For auto-detected libraries, the source is always `Website` (not `LocalDir` —
the library lives in `node_modules`, not in the user's source tree). The
`discover_lib_url` function probes the 7 common llms.txt patterns:

1. `https://{name}.com/llms.txt`
2. `https://{name}.dev/llms.txt`
3. `https://{name}.com/llms-full.txt`
4. `https://{name}.io/llms.txt`
5. `https://www.{name}.com/llms.txt`
6. `https://raw.githubusercontent.com/{name}/main/llms.txt`
7. `https://raw.githubusercontent.com/{name}/master/README.md`

First hit >50 bytes wins. All probes have 5s timeouts. If none succeed, the
library is silently skipped (not an error — most libraries don't have llms.txt
yet).

### 2.3 Skills/agents from manifests

After docs are indexed, the task checks for a `sensei.library.json` manifest.
Two paths:

**Local source** (existing behavior): `load_manifest_from_root()` reads the
manifest from the filesystem. Works for libraries developed alongside the
project.

**Website source** (new in P1.7): probe for `sensei.library.json` at the
llms.txt root URL (e.g. `https://rokkit.dev/sensei.library.json`). If present,
parse and store via `replace_library_capabilities()`. This is the extension that
makes skills/agents discoverable from library websites.

**Convention:** libraries that want to provide sensei capabilities host
`llms.txt` (docs) and optionally `sensei.library.json` (skills/agents) at the
same URL root. The auto-index pipeline discovers both.

### 2.4 What the user sees

**Nothing.** That's the point.

After a project scan, within a few minutes:
- `search_lib_docs("drizzle schema")` returns real Drizzle ORM docs
- `list_library_skills("rokkit")` returns styling/component skills
- `get_lib_docs("supabase", "auth")` returns Supabase auth docs
- The session start hook (P1.8) includes "This project uses: rokkit, drizzle, supabase"

---

## 3. Session start injection (P1.8)

After P1.7 indexes library docs, P1.8 pushes library awareness into every
session.

The SessionStart hook (S4) already injects rules + patterns + memories. P1.8
adds a library block:

```
<sensei-libraries>
Project dependencies with docs available:
- rokkit (v2.1.0): component library for Svelte 5.
  Skills: rokkit-components, rokkit-styling, semantic-styles-rokkit
  Use `get_lib_docs("rokkit", "<component>")` for API details.
- drizzle-orm (v3.2.0): TypeScript ORM for Postgres/SQLite.
  Use `search_lib_docs("drizzle")` for schema/query patterns.
- supabase (v2.45): client library for Supabase.
  Use `get_lib_docs("supabase")` for API details.
</sensei-libraries>
```

**How many libraries to inject?** Top 5 by import frequency (how many source
files import from the library). This avoids flooding the context with every
transitive dependency. The full list is always available via
`get_lib_docs`/`search_lib_docs` — the session-start block is a preview, not
the complete set.

---

## 4. Rate limiting and failure handling

| Concern | Handling |
|---------|----------|
| **Too many concurrent fetches** | Max 5 `IndexLibrary` tasks from auto-detect running simultaneously. The existing task queue already supports concurrency limits. |
| **Per-library timeout** | 5s per URL probe (existing `discover_lib_url` behavior). Total per-library: max 7 probes × 5s = 35s worst case; typically 1-2 probes hit. |
| **Library has no llms.txt** | Silent skip. Log at INFO level ("no llms.txt found for {name}"). Never surface as an error to the user or block the scan pipeline. |
| **Fetch fails (network error)** | Silent skip. Log at WARN level. Retry on next scan reconcile (the library still has `indexed_at IS NULL`). |
| **Manifest parse fails** | Skip skills/agents only. Docs are still indexed. Log at WARN level. |
| **Rate limit from a host** | Respect 429 responses. Back off per-host. Don't re-probe the same host within 60s. |

---

## 5. User control

| Control | Mechanism | Scope |
|---------|-----------|-------|
| **Disable auto-index for a library** | Set `tags = ["auto-index-skipped"]` on `project_libraries` row | Per-library, per-project |
| **Disable auto-index globally** | `SENSEI_AUTO_INDEX_LIBS=0` env var | System-wide |
| **Re-index a specific library** | `POST /api/libs/index` with `name` (existing endpoint) | Per-library |
| **See what was auto-indexed** | `GET /api/libs` returns `indexed_at` + `source_type` per library | Observable |

The `auto-index-skipped` tag is checked before enqueueing. If present, the
library is skipped on every scan. The user can remove the tag to re-enable.

---

## 6. Data model changes

**None.** The existing tables are sufficient:

- `sensei.libraries.indexed_at` — NULL means "needs auto-index" (the trigger)
- `sensei.libraries.source_type` — `llms.txt` for auto-discovered, `http` for explicit URL
- `sensei.library_skills.source` — `"manifest"` for skills from `sensei.library.json`
- `sensei.project_libraries.tags` — `["auto-index-skipped"]` for opt-out

The only new code is the trigger at the end of `extract_deps` and the manifest
probe for website sources. Everything else rides the existing pipeline.

---

## 7. What this enables (downstream)

Once library docs are auto-indexed:

- **P1.2 (context-pack hook):** PreToolUse can inject relevant library API docs
  when the model works with files that import from indexed libraries.
- **P1.8 (session start):** The model knows what libraries are available
  without discovering them.
- **Phase 4 (governance plane):** Library-specific skills/agents become part of
  the team's shared capability set — a Dōjō can mandate "all projects using
  rokkit must use the rokkit-components skill."
- **The marketplace:** Library-provided skills/agents complement the
  sensei-native marketplace catalog — two sources of capability, one serving
  surface.
