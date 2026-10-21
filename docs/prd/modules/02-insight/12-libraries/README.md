# Libraries · M12

**Plane:** insight · **Status:** [roadmap/02-insight.md](../../../../roadmap/02-insight.md) ·
**Serves:** solo developer ([persona 1](../../../03-personas.md#1--solo-developer--the-wedge)),
team developer ([persona 2](../../../03-personas.md#2--team-developer--the-inheritor))

## Purpose

Every workspace depends on things the assistant has never seen — private Cursor
plugins, an in-house `@acme/design-system` UI kit — and things it has vaguely seen at
wrong versions (React 19 vs React 17 API drift). Libraries closes that gap with a
detect → wrap → query → watch loop: sensei detects unmanaged dependencies, indexes
their docs (pinned to the workspace's lockfile version), lets the user **wrap** a
library to control how sensei tracks it, answers queries (`search_lib_docs`) against
the indexed pages, and watches for upgrades so docs and call sites don't silently
drift. Heavily-used libraries gain generated **skills** ("how do I set up kavach
auth?"). The user feels this module as *the assistant knows how to use this library,
correctly, for this workspace's version*.

> **Status honesty (2026-10-14).** The storage names in this doc were aspirational:
> pages, skills and agents are all rows in **`sensei.library_content`** (`kind`), not
> `library_pages` / `library_skills`; there is **no `wrap_library` MCP tool**; the
> library API lives in `api/handlers/libraries.rs` (not `mcp/library_pages.rs`). See
> gap `G-LIB-WRAP`.

## In scope / out of scope

- In: detection of unmanaged dependencies + wrap-me workflow; doc ingestion from
  `llms.txt`/`llms-full.txt`, repo trees, published API refs, registries, and source
  AST (for internal libs); version pinning from the workspace lockfile + re-index on
  bump; skill generation above a usage threshold; library-upgrade drift surfacing;
  `sensei.libraries`, `sensei.library_versions`, `sensei.library_content`
  (kind: `page | skill | agent`), `sensei.library_packages`.
- Out: library *pattern* extraction for the assistant (M10 — the `patterns.yaml` /
  llms.txt patterns pipeline); the underlying upgrade-drift scanner (M11 — reused);
  captures/watcher that trigger re-index (M3); narration-cache copy (M7-owned).

## Surfaces

- **MCP** — `add_library`, `search_lib_docs`, `get_lib_docs`, `list_library_skills`,
  `get_library_skill`, `list_library_agents`, `get_duplicates` (shared). **No
  `wrap_library` tool exists** (gap `G-LIB-WRAP`).
- **API** — `crates/senseid/src/api/handlers/libraries.rs`; Libraries screens
  (Observatory + workspace), add-library progress, version-pin chip.
- **Daemon** — manifest readers (`adapters/manifest*.rs`), version pinning
  (`libraries/version.rs`), doc/source ingestion (`libraries/docs_source.rs`), skill +
  agent generation (`libraries/advisory.rs`), drift watch (via M11).

## Data invariants

- `sensei.libraries` — one row per library (`kind` `detected|imported`, `ecosystem`,
  `embedding`); `sensei.library_versions` — one row per `(library_id, version)` pin;
  **`sensei.library_content`** — one row per indexed artifact, `kind`
  (`library_content_kind`: `page | skill | agent`), keyed on `library_version_id` +
  `name`, with `body`, `focus`, `tokens`, `source`/`source_path`, `version_range`;
  `sensei.library_packages`. There is **no `library_pages` / `library_skills` table** —
  both are `library_content` rows under different `kind`s.
- **Idempotent ingestion.** Re-ingesting a library **supersedes** pages on the
  composite key — never doubles them.
- **Version pin follows the lockfile** (`package-lock.json`, `Cargo.lock`,
  `poetry.lock`, `go.sum`, …). A workspace bump (via the M3 watcher) triggers re-index
  at the new version; older pages stay marked `superseded` for 30d retention so
  historical queries work.
- **Skill threshold.** `usage_count_14d ≥ SKILL_MIN` (default 25) before skill
  generation fires; `get_library_skill` honours the version pin on read.
- **Drift policy.** Surfaced **wrapped-only by default**; raw-import drift is an
  opt-in (avoids flooding Traceability).
- **Custom-lib ingestion fails loudly** on a missing docs directory — source-only
  fallback is explicit, never silent.
- Ingestion source preference: llms.txt → repo README+/docs → published API ref →
  registry mapping → repo source AST.

## Signals shown

| Element | Meaning | Worked example |
|---|---|---|
| Library pages | per-component indexed docs | search_lib_docs "Toggle disabled" → rokkit page (+) |
| Wrap-me card | unmanaged dependency with real usage | "kavach is imported in 12 files — wrap it?" |
| Version-pin chip | lockfile-resolved version per workspace | "rokkit 1.4.2 (superseded 1.3.x)" |
| Generated skills | how-to / convention / migration | "migrate kavach v0→v1" |
| Upgrade drift | call-site signature changes across versions | Traceability items on bump |

## Done gate

- `add_library("rokkit", "~/Developer/rokkit")` pulls every component page from the
  local dir; `add_library("dbd", "<llms-full.txt>")` splits `##`/`###` and stores
  per-component pages.
- Bumping a package in the lockfile triggers re-ingestion at the new version on the
  next tick; re-ingestion never duplicates pages.
- Libraries above the usage threshold generate skills reachable via
  `list_library_skills(library)`; `get_library_skill` returns the pinned-version
  skill, never the wrong version's.
- Custom/internal libraries add from a local repo path with source-only fallback; a
  missing docs directory is an explicit error.
- Upgrade drift items surface for wrapped libraries when call-site signatures change.

```bash
mcp_call add_library --name=rokkit --url=/Users/dev/Developer/rokkit
mcp_call search_lib_docs --query="Toggle disabled"
mcp_call list_library_skills --library=rokkit
```

## Wrong gate

- **Re-ingestion doubles pages instead of superseding** — upsert not idempotent on
  the composite key.
- **Version pin doesn't reflect the lockfile** — manifest adapter read the wrong
  version.
- **Skill generation fires for a library with 3 imports** — threshold too loose.
- **`get_library_skill` returns the wrong version's skill** — pin not honoured.
- **Raw-import drift floods Traceability** — wrapped-only default regressed.
- **Custom-lib ingestion fails silently** on missing docs.
- **Detect never suggests a genuinely unmanaged dependency** — the wrap-me surface
  starves.

## Scenarios

- [`scenarios/libraries.feature`](scenarios/libraries.feature) — detect/wrap/query/
  watch, pinning, skills, drift.

## Open gaps

- The operational `[[pipeline/libraries]]` spec (detect/wrap/query/watch workflow)
  was never authored separately — its substance lives in the ingestion internals spec
  and the observatory Libraries section; M12 folds both. Nominal gaps: none
  registered in [06-gaps.md](../../../../roadmap/06-gaps.md); watch the
  non-wrapped-drift opt-in policy and the skill threshold tuning.
- **`G-LIB-WRAP`** — there is no `wrap_library` MCP tool; the wrap-me workflow has no
  write surface today.

## Related

- [spec/pipeline/library-intelligence](../../../../spec/pipeline/library-intelligence.md) —
  ingestion internals, versioning, skills, drift (legacy depth, folds here).
- [spec/pipeline/traceability](../../../../spec/pipeline/traceability.md) — upgrade
  drift reuse; [spec/pipeline/patterns](../../../../spec/pipeline/patterns.md) —
  library patterns source.
- [features/03-observatory.md](../../../../features/03-observatory.md) — Libraries
  section status.
- [prd/02-objectives.md](../../../02-objectives.md) · [prd/01-vision.md §The core loop](../../../01-vision.md#the-core-loop).
- Crate: `crates/senseid/src/libraries/{version.rs,docs_source.rs,advisory.rs}`,
  `crates/senseid/src/adapters/manifest.rs`, `crates/senseid/src/api/handlers/libraries.rs`.