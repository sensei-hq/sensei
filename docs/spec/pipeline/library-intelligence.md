# 庫 · Pipeline · Library intelligence (ingestion internals)

> **Scope note.** This spec covers the **ingestion internals** —
> how docs come in (per-source), how version pins follow the
> project lockfile, how custom / internal libraries get parsed
> from source, how skills are generated, how drift-on-upgrade is
> detected. The user-visible tables, screens, and detect/wrap/
> query/watch workflow live in [[pipeline/libraries]]. Shared
> tables (`sensei.libraries`, `sensei.library_pages`) are declared
> there; new tables introduced by ingestion
> (`sensei.library_versions`, `sensei.library_skills`) are
> declared here.

**Owner files:**
- Manifest reading: `crates/senseid/src/adapters/manifest/*.rs`
- Doc ingestion: `crates/senseid/src/mcp/library_pages.rs`
- Version pinning: `crates/senseid/src/libraries/version.rs`
- Custom-lib source ingestion: `crates/senseid/src/libraries/custom.rs`
- Skill generation: `crates/senseid/src/libraries/skills.rs` (proposed)
- Drift watch: reuses [[pipeline/traceability]]

**Companion design doc:** `docs/archive/ideas/09-library-intelligence.md`. Also see the operational memory
(memory: project_mcp_libdocs_rokkit).

## Purpose

Sensei's assistant is only as good as its library knowledge. Every
project depends on things the assistant has never seen (private
Cursor plugins, in-house `@acme/design-system` UI kit) and things
it has vaguely seen at wrong versions (React 19 vs React 17 API
drift). Library intelligence closes that gap by ingesting docs,
pinning them to project versions, extracting usage patterns, and
generating focused skills — so the assistant knows *how to use
this library, correctly, for this project's version*.

Five capabilities:

1. **Doc ingestion** — from `llms.txt` / `llms-full.txt`,
   published API refs, local dirs, GitHub tree URLs.
2. **Version pinning** — index at the project's lockfile version;
   re-index when the project bumps.
3. **Custom / internal library indexing** — clone or crawl source
   and derive the same doc shape.
4. **Skill generation** — focused MCP skills that answer common
   library questions ("how do I set up kavach auth?").
5. **Drift detection** — the library evolved API; docs did not.
   Feeds [[pipeline/traceability]] and surfaces on the Libraries
   screen.

Kanji is 庫 — *repository*.

## Data invariants

- `sensei.libraries` — one row per detected library (see
  [[pipeline/libraries]] data shape).
- `sensei.library_pages` — per-component pages indexed. Keyed on
  `(library_id, component, version?, source_kind, source_url_or_path)`.
- `sensei.library_versions` — one row per project × library ×
  version pin — records the lockfile-derived version so the
  ingestion knows which docs are current.
- `sensei.library_skills` — one row per generated skill
  (name, library_id, focus, prompt_body, generated_at, tokens).
- Every human-facing text comes through [[pipeline/narration-cache]]
  where applicable.

## Ingestion sources

Ordered by preference:

1. **`llms.txt` / `llms-full.txt`** — the emerging standard;
   fastest, cleanest. Split by `##` / `###`. dbd uses this.
2. **README + `/docs` in the repo** — walk the repo tree;
   markdown gets indexed per file. Rokkit uses this.
3. **Published API ref** — `crates.io`, `pypi.org`, or the
   library's website. Scraped and structured.
4. **Registry / external doc source** — a small central
   registry maps package name → doc URL for the common cases
   the ingester can't auto-detect.
5. **Repo source** — for internal libraries with no docs, parse
   the source AST and generate signatures + summaries via
   [[pipeline/inferencing]].

`add_library(name, url_or_path)` accepts any of the above.

## Version pinning

For each project × library pair, the ingester reads the lockfile
(`package-lock.json`, `Cargo.lock`, `poetry.lock`, `go.sum`,
`Gemfile.lock`, `pnpm-lock.yaml`) and pins the doc index to the
resolved version.

- When a project bumps the version, sensei detects via the
  watcher ([[pipeline/capture]] root-watcher) and re-ingests
  at the new version.
- Older versions stay in the index (marked `superseded`) so
  historical queries work. Retention: 30 days after supersede
  by default; user can pin longer.

## Skill generation

For each library with `usage_count_14d >= SKILL_MIN` (default
25), sensei generates one or more MCP skills:

- **How-to skills** — "how to configure kavach auth" — a
  short curated prompt that reads the library docs and
  authoritative usage examples.
- **Convention skills** — "rokkit component conventions" —
  the extracted patterns ([[pipeline/patterns]] library source).
- **Migration skills** — "migrate from vX to vY" — auto-
  generated on version bumps by diffing the two doc snapshots.

Skills persist in `sensei.library_skills`. The MCP surface
exposes them:

- `get_library_skill(library, focus)` — returns the skill body.
- `list_library_skills(library)` — enumerate.

Skills feed the assistant's context at session start when the
project uses the library heavily.

## Drift detection

Reuses [[pipeline/traceability]] against the wrapper module
(when the user has wrapped the library — see
[[pipeline/libraries]] wrapping) OR against project code that
directly imports the library.

- On version bump, drift items surface for any call site whose
  signature changed between the two versions.
- On non-wrapped libraries, drift is per import — noisier but
  still useful.

## Signals produced

| Signal | Consumer |
|---|---|
| Library docs per component | [[screen/observatory-libraries]] · [[screen/project-libraries]] |
| Generated skills | Assistant context at session start · Playground |
| Version pin state | Libraries screen chip |
| Drift items on upgrade | [[screen/observatory-traceability]] |
| Ingestion progress | Libraries add-library UI |

## Done gate

- Ingesting `add_library("rokkit", "~/Developer/rokkit")` pulls
  every component page from the local dir.
- Ingesting `add_library("dbd", "https://…/llms-full.txt")`
  splits by `##`/`###` and stores per-component pages.
- Version pin follows the project's lockfile — bumping a package
  triggers re-ingestion on the next tick.
- Custom / internal libraries can be added from a local repo
  path with source-only fallback.
- Skills generate for libraries above the usage threshold and
  the assistant can reach them via
  `list_library_skills(library)`.
- Drift items surface after a library bump when call-site
  signatures change.

Optional check:
```
mcp_call add_library --name=rokkit --url=/Users/Jerry/Developer/rokkit
mcp_call search_lib_docs --query="Toggle disabled"

# What skills exist for rokkit?
mcp_call list_library_skills --library=rokkit
```

## Wrong gate

- **Re-ingestion doubles pages instead of superseding.**
  `upsert_library_page` isn't idempotent on the composite key.
- **Version pin doesn't reflect the lockfile.** Manifest adapter
  isn't reading the resolved version.
- **Skill generation fires for a library with 3 imports.**
  Threshold too loose.
- **`get_library_skill` returns a skill for the wrong library
  version.** Pin not honored on read.
- **Drift items on non-wrapped libraries flood the Traceability
  screen.** Need a policy: default to wrapped-only surfacing
  with an opt-in for raw-import drift.
- **Custom-lib ingestion from source fails silently on missing
  docs directory.** Should error specifically.

## Related

- [[pipeline/libraries]] — the operational screen + wrap-me
  workflow
- [[pipeline/traceability]] — drift detection
- [[pipeline/patterns]] — library patterns source
- [[pipeline/mcp-surface]] — skills as MCP tools
- [[pipeline/capture]] — lockfile watcher triggers re-index
- (memory: project_mcp_libdocs_rokkit) (memory) — the live ingestion
  work
- (archive: ideas/09-library-intelligence.md) — source design
