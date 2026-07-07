# 庫 · Pipeline · Libraries

**Owner files:**
- Detection: `crates/senseid/src/scan/library_detection.rs`
- Wrapping: `crates/senseid/src/adapters/libraries/`
- Query: MCP `search_lib_docs` / `get_lib_docs`
- Docs ingestion: `crates/senseid/src/mcp/library_pages.rs` (per-component ingestion from local dir / GitHub tree URL / website llms URL)
- Drift watch: reuses [[pipeline/traceability]] against library nodes

## Purpose

Libraries are the third-party dependencies the developer's
projects lean on — Rokkit, dbd, Tauri, Svelte, Postgres — plus
their internal libraries. Sensei tracks them for four reasons:

1. **Detect** — which libraries does each project actually use,
   and how often?
2. **Wrap** — high-usage libraries with no wrapper module get a
   proposed wrapper so future assistant work becomes uniform
   ("use the wrapper, not raw calls").
3. **Query** — the assistant can look up library docs
   through the sensei MCP surface, without leaving the loop.
4. **Watch** — a wrapped library that upgrades version should
   trigger drift detection against the wrapper's API surface.

Kanji is 庫 — *repository / library*.

## Data invariants

### Detection

- `sensei.libraries` — one row per detected library:
  - `id` uuid, `name` text, `version` text (nullable), `ecosystem`
    text (`npm | crates | pip | go | ruby | …`), `source` text
    (`npm-registry | crates-io | github | local | ...`),
    `usage_count_14d` int, `last_seen_at` timestamptz,
    `wrapped` bool, `wrapper_module_id` uuid nullable,
    `docs_source_kind` enum `local | github | website`,
    `docs_source_url` text (nullable).
- `sensei.project_libraries` — many-to-many join with per-project
  `usage_count`, `first_seen_at`, `last_seen_at`.
- Detection reads manifest files (`package.json`, `Cargo.toml`,
  `pyproject.toml`, `go.mod`, `Gemfile`, …) via the manifest
  adapter (see [[project_manifest_adapter_direction]] memory) and
  cross-references against code-level import counts. Manifest
  entry without imports = declared-not-used; imports without
  manifest = suspicious (git-vendored?).

### Docs ingestion

Docs come from **three source shapes**:

1. **local directory** — Jerry runs sensei against
   `~/Developer/rokkit`; the library's `.md` files under `docs/`
   or `content/` are indexed. Structure: per-component files
   (`docs/components/foo.md`).
2. **GitHub tree URL** — sensei clones or reads via GitHub API.
   Same per-component structure expected. Handles rokkit's
   layout (index + `components/*.txt`).
3. **website `llms.txt` / `llms-full.txt`** — for libraries that
   publish an LLM-friendly doc bundle (dbd does — `llms.txt`
   overview + `llms-full.txt` with `##` / `###` sections). The
   ingester splits by heading, one row per component.

`upsert_library_page` accepts a `local_path` to bind a page to
its source file so re-ingestion is deterministic.

Result: `sensei.library_pages` — one row per component/page with
`(library_id, component, title, content, source_kind, source_url,
resolved_at)`.

### Wrapping

- Detection surfaces "wrap this" candidates when
  `usage_count_14d >= WRAP_MIN` (default 12) AND `wrapped = false`.
- A wrapper is a generated module in the user's own repo (see
  the folder-role auto-detection in [[pipeline/capture]]).
- Sensei doesn't force the wrap — the recommendation surfaces via
  [[pipeline/insights]] source #4 (Library-tier detections). User
  applies from Observatory Insights or from
  [[screen/project-libraries]].

### Query

MCP tools (already live per
[[project_mcp_libdocs_rokkit]] memory):

- `get_lib_docs(name)` — library overview
- `get_lib_docs(name, component)` — specific component docs
- `search_lib_docs(query)` — cross-library semantic search
- `add_library(name, url)` — index a new library (accepts local
  path / GitHub URL / website URL)

## Signals produced

| Signal | Consumer |
|---|---|
| Library list per project | [[screen/observatory-libraries]] + [[screen/project-libraries]] |
| Wrap-me recommendation | [[pipeline/insights]] source #4 → Insights Now column |
| Library-drift item on wrapper upgrade | [[pipeline/traceability]] drift items scoped to wrapper |
| Doc lookup response | MCP query surface |
| Version-conflict warning | `sensei.project_dependencies` view already exposes conflicts (see [[project_p2_sweep_2026_07]]) |

## Done gate

- On Jerry's live data every library present in a
  `~/Developer/*` or `~/Work/*` project's manifest has a
  `sensei.libraries` row and a `sensei.project_libraries` join.
- MCP `add_library(name="rokkit", url="/Users/Jerry/Developer/rokkit")`
  ingests all component pages within `500ms/page` and reports
  the count.
- `search_lib_docs("$state")` returns a Svelte or Rokkit page
  citing `$state` when either is indexed.
- Wrap-me candidates surface as high-impact recommendations when
  `usage_count_14d` crosses `WRAP_MIN`.
- Wrapper upgrade (new version detected) creates drift items via
  [[pipeline/traceability]] against the wrapper's surface API.
- Version-conflict warnings show on Project Libraries when the
  underlying `project_dependencies` view has rows.

Optional check:
```
mcp_call add_library --name=dbd --url=https://dbd-docs.example/llms-full.txt \
  | jq '.pages_ingested'

curl -s 'http://localhost:7744/api/libraries' \
  | jq '.libraries[] | select(.usage_count_14d > 10) | {name, wrapped}'
```

## Wrong gate

- **A library declared in `package.json` but never imported shows
  as high-usage.** Manifest and code cross-reference divergent.
- **Wrap-me candidates never surface for a project that clearly
  uses one library heavily.** `WRAP_MIN` threshold or the
  recommendation pipeline is not consuming the signal.
- **Docs ingestion for a GitHub URL doubles pages on re-ingest.**
  `upsert_library_page` isn't idempotent — should be keyed on
  `(library_id, source_url)` or `(library_id, local_path)`.
- **`search_lib_docs` returns matches from an archived library
  version.** Retention policy needed: on re-ingest, older pages
  are marked superseded, not retained.
- **Wrapper drift after upgrade doesn't create drift items.** The
  wrapper's surface API is out of sync with the library but no
  signal fires — the wrapping story is broken.
- **Local `add_library("rokkit", "/Users/Jerry/Developer/rokkit")`
  fails silently on a missing docs directory.** Should error with
  a specific "no docs found at path" message.

## Related

- [[pipeline/capture]] — manifest reading + library detection
- [[pipeline/traceability]] — drift against wrapper surface
- [[pipeline/insights]] — wrap-me recommendation source
- [[pipeline/mcp-surface]] — the query tools
- [[screen/observatory-libraries]] — primary consumer
- [[screen/project-libraries]] — project-scoped one-click wrap
- [[project_mcp_libdocs_rokkit]] (memory) — the live ingestion
  work from 2026-06-30
- [[project_manifest_adapter_direction]] (memory) — the manifest
  adapter architecture
