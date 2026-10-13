# Blueprint — project-level code-quality metrics (qlty integration)

> Status: **idea / blueprint** (follow-up to the code-graph indexing work). No
> code yet. Captures the vision, data model, ingestion path, grading, dashboard
> surface, and the module-wise feature-docs + Gherkin approach for review.

## Vision / scenario

A developer points sensei at an existing codebase. After indexing, they open the
project and its **dashboard shows quality grades** — coverage and maintainability
(A/B/C/D/F, the way qlty.sh grades them) — **combined with sensei's own graph
metrics** (community structure, doc/requirement coverage, god-nodes, hotspots).
The grades are **per project AND per module** (each `workspace_member` / crate /
package), so a monorepo shows where quality concentrates and where it decays.

The point: sensei already builds the *structural* graph; layering qlty's
*quality* signal on the same folder/module spine turns "here's your code" into
"here's the health of your code, module by module, and what to fix first."

## Data sources

| Signal | Tool | Granularity |
|---|---|---|
| Cyclomatic + cognitive **complexity** | `qlty metrics` | file / function / dir |
| **LCOM** (cohesion) | `qlty metrics` | class / file |
| **LOC / lines** | `qlty metrics` | file / dir |
| **Duplication** (mass, locations) | `qlty smells` | cross-file |
| **Test coverage %** | `qlty coverage` (ingests lcov/cobertura/etc.) | file / project |
| **Maintainability grade** (A–F) | derived from complexity + duplication + cohesion | file / dir / project |
| **Graph metrics** (already in sensei) | the code graph | project / module |

sensei-native graph metrics to combine (already computable post-indexing):
- **Doc/requirement coverage** — `section`/`rationale` node density vs code
  (how documented is each module).
- **God-nodes / hotspots** — `communities.god_node_ids` (top-degree hubs).
- **Community structure** — count, coverage %, cross-module edges (coupling).
- **Duplicate functions** — the existing embedding-cosine `get_duplicates`.
- **Doc drift** — existing `drift_items`.

## How sensei ingests qlty (the seam)

A new terminal analysis task, chained after a project's folders reach `indexed`
(mirrors how `DetectCommunities` chains today):

1. **`ComputeQualityMetrics(project)`** — runs `qlty metrics --json`,
   `qlty smells --sarif`, and (if a coverage report is present or configured)
   `qlty coverage` against the project's repo root on a `spawn_blocking` thread
   (never blocks the async pool — same pattern as `resolve_libs`).
2. Parse the JSON/SARIF into per-file rows, **roll up to each module** using the
   folder spine: a file's owning module = the nearest ancestor folder of kind
   `workspace_member` (else the repo root). This is exactly the boundary D5a now
   records — no new detection needed.
3. Compute grades per module + per project (replicate qlty.sh's grade formula, or
   shell `qlty` for its own grade if it exposes one) and store.
4. **Fail-open, honest-null** (per the never-fabricate rule): if `qlty` isn't
   installed or a report is absent, store `grade = NULL` / `source = 'null'` —
   never a fabricated grade. Coverage is `NULL` until a real report is ingested.

## Data model (proposed)

```
inference.quality_metrics
  id             uuid pk
  folder_id      uuid  -> sensei.folders(id)   -- project root OR a workspace_member
  scope          text  -- 'project' | 'module'
  coverage_pct   numeric        null            -- null until a real report
  maintainability_grade text     null            -- 'A'..'F' | null
  complexity_avg numeric        null
  duplication_mass integer      null
  lcom_avg       numeric        null
  loc            integer        null
  props          jsonb  not null default '{}'    -- source provenance + raw rollup
  computed_at    timestamptz
  unique(folder_id, scope)
```

Keyed on `folder_id` so a module's metrics attach to its `workspace_member`
folder row — the same spine the graph tree already renders.

## Dashboard surface

- **Project header**: two grade chips (Coverage B · Maintainability A), like the
  qlty.sh badge, plus a sensei "documentation" chip from section/rationale density.
- **Module table**: one row per `workspace_member` — grade chips + LOC +
  complexity + duplication + god-node count, sortable, so the worst module surfaces.
- **Drill-down**: a module → its files ranked by complexity/duplication, linked to
  the graph nodes (reuse `/api/graph/{repoId}/tree` + a metrics overlay).
- New read endpoints: `GET /api/quality/{project}` (project + module rollup),
  reusing the `scope_folder_ids` + honest-500 pattern from the graph handlers.

## Module-wise feature docs + Gherkin (the verification layer)

Each metric gets a short **feature doc** (`docs/spec/metrics/<metric>.md`): what
it measures, how it's computed, thresholds for each grade, and the honest-null
rule. Each carries **Gherkin scenarios** that double as the acceptance tests:

```gherkin
Feature: Maintainability grade per module
  Scenario: A monorepo member gets its own grade
    Given the project "dbd" is indexed
    And qlty metrics have been computed
    When I open the dbd project dashboard
    Then each workspace_member (dbd-cli, dbd-core, …) shows a maintainability grade
    And the grade is derived from that member's files only

  Scenario: No coverage report is honest, not fabricated
    Given a project with no ingested coverage report
    When quality metrics are computed
    Then coverage_pct is null and the dashboard shows "—", not a made-up number
```

The Gherkin maps 1:1 to a `sensei-acceptance-tester` / done-gate check, so the
feature doc IS the functional-verification contract.

## Open questions (for the follow-up)

- Coverage ingestion: auto-detect `lcov.info`/`coverage.xml` in the repo, or
  require the developer to run `qlty coverage publish`? (Lean: auto-detect + a
  config knob.)
- Grade formula: reimplement qlty.sh's weighting, or shell out to `qlty` for its
  own grade to stay consistent with the website? (Lean: shell out if it exposes a
  machine-readable grade; else document our formula.)
- Recompute cadence: on every re-index, or a slower analyzer tick (metrics are
  less volatile than the graph)? (Lean: analyzer tick + on-demand.)
- Multi-language: `qlty` covers many languages; confirm the grade is comparable
  across a polyglot monorepo before showing one project-level number.

## Why this is cheap to build on what exists

- The **module boundary** (`workspace_member`) is already recorded (D5a).
- The **terminal-barrier chaining** pattern (D4.1) is the model for the
  `ComputeQualityMetrics` task.
- The **honest-null / fail-open** discipline is already the house style.
- The **retrieval + scope** handlers (Phase 7) are the template for the read API.
- `qlty` is already installed + configured (`.qlty/qlty.toml`).
