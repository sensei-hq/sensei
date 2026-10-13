# Patterns & Conventions · M10

**Plane:** insight · **Status:** [roadmap/02-insight.md](../../../../roadmap/02-insight.md) ·
**Serves:** solo developer ([persona 1](../../../03-personas.md#1--solo-developer--the-wedge)),
team developer ([persona 2](../../../03-personas.md#2--team-developer--the-inheritor))

## Purpose

Patterns are how the assistant knows *how to build here*. Without pattern knowledge
the assistant makes suboptimal choices and the user corrects it — the exact loop
sensei exists to shorten. Five sources answer five questions: what shapes already
exist in this repo (**codebase**), the conventions for using this library correctly
(**library**), established patterns for problems like this (**registry**), the
tradeoff-laden choices the user should make deliberately (**architectural options**),
and what this project has consistently done — its implicit standard (**derived**).
They come in three modes: design/positive patterns to follow, anti-patterns to avoid,
and optimization opportunities. The user feels this module as *the assistant already
knows how we do things here*.

> **Status honesty.** As of 2026-08-05 the pattern *engine* is **not built**:
> `crates/senseid/src/patterns/` does not exist, and `inference.detected_patterns`
> holds behavioral churn (`rework:<path>`, `family` NULL) — not architectural
> patterns. This module's Done gate is therefore the *engine's* bar, and its Wrong
> gate is about not lying on the way there. The roadmap calls the pattern detector
> one of the two unbuilt feeds that starve Delivery and Team surfaces.

## In scope / out of scope

- In: the five-source detection/ingestion model; the three modes (design · anti ·
  opt); the detection runtime (`crates/senseid/src/patterns/` — to be built); the
  MCP surface (`get_patterns`, `get_pattern_for`, `match_pattern`,
  `get_pattern_options`, `get_project_conventions`); architectural choices as
  persisted `sensei.pattern_choices` that propagate as rules; derived-pattern
  auto-promotion (threshold); anti-pattern recognisers (duplication, spaghetti
  coupling, broken layering, god object, dead code); optimization opportunities
  (n+1, sync-on-hot-path, missing index, missing cache).
- Out: the FTR/adherence *evidence* that scores patterns (M7 analyzer
  `pattern_effectiveness`); promotion into the rule ladder (M13); the library
  `patterns.yaml`-style ingestion plumbing (M12 ingestion internals);
  `get_duplicates` detection internals (shared tool, reuse not reimplement).

## Surfaces

- **MCP** — `get_patterns(project, source?)` (default order
  derived > codebase > library > registry > option), `get_pattern_for(name)`,
  `match_pattern(description)`, `get_pattern_options(category)`,
  `get_project_conventions()` — the "know how you work here" call.
- **API** — `/api/patterns/{project}/duplicates` (reused by duplication detection);
  `GET /api/graph/…` community clusters feed custom codebase patterns.
- **Daemon** — pattern detection scheduled by the M7 analyzer tick (incremental gate
  on the affected folders), `crates/senseid/src/patterns/{detect,codebase,library,
  registry,options,derived}/`.
- **App** — Project Patterns review + promote; Insights violation cards (anti),
  Soon column (opt).

## Data invariants

- `inference.detected_patterns` — one row per detected instance: `pattern_id` text
  (`codebase.adapter`, `lib.rokkit.data_driven_component`, `registry.patterns_dev.…`,
  `option.string_handling`, `derived.rest_naming`, `anti.duplication`,
  `opt.n_plus_one`), `source` enum
  `codebase | library | registry | option | derived | anti | opt`, `family` enum
  `design | anti | opt`, `instances`, `example_nodes` (into `sensei.nodes`),
  `confidence` 0..1, `ftr_delta_observed` nullable, `state`
  `detected | promoted | dismissed`, `signature` (dedup by instance shape).
- `sensei.pattern_registries` — external registries (`patterns.dev`),
  `indexed`, `last_ingested_at`.
- `sensei.pattern_choices` — per project × `option_category` chosen pattern +
  `chosen_by` + `rationale`; a choice propagates as a rule with
  `source: chosen:option:{category}` (M13).
- **Ordering on read.** `get_patterns` default:
  derived > codebase > library > registry > option — a registry pattern must never
  outrank the project's own convention.
- **Dedup across sources.** Same pattern from two sources collapses to one entry.
- **Incremental detection.** Re-runs only on changed/affected folders — never every
  tick over the whole project.
- All human-facing pattern copy goes through narration-cache.

## Signals shown

| Element | Meaning | Worked example |
|---|---|---|
| `get_project_conventions()` | derived patterns + chosen options for this project | "SSR data-separation is the standard here" |
| `match_pattern(task)` | ranked matches with tradeoffs | adapter task → `custom.assistant_adapter_trait` + instances |
| Codebase custom pattern | named community cluster with shared shape | `custom.assistant_adapter_trait` |
| Anti-pattern violation | duplication / spaghetti / broken layering above threshold | Insights Now violation card |
| Optimization opportunity | n+1 / sync-on-hot-path / missing index | Insights Soon |
| FTR conformance | `pattern_effectiveness.ftr_delta` | promotion suggestion when delta crosses min |

## Done gate

- Every project with > 1000 nodes has codebase design patterns detected where they
  exist (adapter / plugin / observer / subscriber are the likeliest in this repo).
- Libraries with llms.txt-style pattern sections ingest their patterns when
  `add_library` runs; ≥1 industry registry is ingestible and reachable via
  `match_pattern`.
- An architectural option chosen once persists in `pattern_choices` and is read back
  by `get_project_conventions()` on the *next* session — never re-asked.
- Derived patterns surface after N consistent usages (default 5) and auto-promote
  with a threshold + user override.
- `get_patterns(project=sensei)` returns the assistant-adapter trait pattern with all
  its instances.
- The verifier (M13) blocks a commit introducing a new file matching an
  anti-pattern signature — with a "pending consolidation" expiry override.

## Wrong gate

- **Assistant builds a new adapter without calling `get_patterns` first** — the
  anti-spaghetti contract isn't enforced.
- **Registry pattern applied blindly over the codebase's own convention** — ordering
  wrong.
- **Same pattern from two sources reads twice** — dedup missing.
- **Chosen option re-asked the next session** — `pattern_choices` not read at
  session start.
- **Detection re-runs on every tick** — incremental gate missing.
- **A false "duplication" blocks a legit refactor mid-consolidation** — the
  pending-consolidation override with expiry is required.
- **On the way to built: `inference.detected_patterns` silently means "churn" while
  the surface claims architecture patterns** — the current state must render as
  partial/unbuilt, not as done.

## Scenarios

- [`scenarios/patterns.feature`](scenarios/patterns.feature) — the five sources, the
  MCP surface, choices, anti-patterns. Pins are `# gap:` until the engine lands.

## Open gaps

- The pattern detector is an **unbuilt feed** documented in
  [02-insight.md §Sequencing](../../../../roadmap/02-insight.md) — one of two that
  starve Delivery and Team surfaces. No dedicated register row beyond that; the
  auto-promotion policy (threshold vs opt-in) is an open decision with a stated
  bias. The assister-adapter trait seam is the canonical custom-pattern example.

## Related

- [spec/pipeline/patterns](../../../../spec/pipeline/patterns.md) — five sources,
  MCP surface, anti-patterns, options (legacy depth, folds here; carries the
  roadmap/not-built banner).
- [spec/pipeline/library-intelligence](../../../../spec/pipeline/library-intelligence.md) —
  library-pattern ingestion hooks.
- [spec/pipeline/analyzer](../../../../spec/pipeline/analyzer.md) — pattern
  detection scheduling + `pattern_effectiveness` scoring.
- [spec/pipeline/governance](../../../../spec/pipeline/governance.md) — promoted
  patterns → rules; verifier conformance.
- [prd/02-objectives.md](../../../02-objectives.md) · [prd/01-vision.md §The core loop](../../../01-vision.md#the-core-loop).