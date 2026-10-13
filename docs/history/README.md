# History — the readable index of superseded material

**Nothing is deleted.** When a doc is superseded or a dated snapshot ages out,
it moves here — stamped `status: superseded` where it was a claim, plain where
it was a dated report — and this page is its index. Git is the durable backstop
(`git log --follow`, the renames preserve it); this tree is the readable one.

The reorg that created this tree: [`reorg.md`](../reorg.md) (2026-10-13) —
now the authority for what moved where.

## What lives here

| Move | What it is | Read it when… |
|---|---|---|
| [`analysis/`](analysis/) | dated gap analyses and measurements (indexer coverage, metrics, governance, graph) | you need a measurement's original evidence |
| [`plans/`](plans/) | superseded plan dossiers (dojo-personal-zone, indexer sequence + superseded rust plan) | tracing an old sequencing decision |
| [`plan/`](plan/) | the entire old plan tree — **README** (G1–G10 gap analysis → now `../roadmap/`), operating-model, relay-engine, dated build plans | you need the old gap numbering or a relay design draft |
| [`blueprints/`](blueprints/) | dated blueprints (dojo full-surface, code-graph-meaningful, project-quality-metrics, …) | a blueprint is re-opened |
| [`reports/`](reports/) | dated reports (2026-10-13 indexer placement) | the 2026-10-13 placement numbers |
| [`verification/`](verification/) | dated surface verification | re-verifying a surface area |
| [`requirements/`](requirements/) | absorbed requirements (front-door, repository-sharing) — **repository-sharing is still the mandatory-first-read** for that model | the sharing/visibility model |
| [`design/`](design/) | dated design notes (relay RLS, metrics rebuild impact) | those decisions |
| [`roadmap/phases.md`](roadmap/phases.md) | the old six-phase roadmap — superseded by `../roadmap/` plane files | the old MVP→…→orchestrator framing |
| [`features/coverage-map.md`](features/coverage-map.md) | the 2026-07-25 built-vs-pending snapshot — superseded by `../roadmap/` | 2026-07-25 statuses |
| [`backlog/writeups.md`](backlog/writeups.md) | the full previous backlog body (indexer/dbd findings with evidence) — superseded by the live issue list at `../backlog.md` | a finding's full write-up |
| [`personas/`](personas/) | the stub personas README — superseded by `../prd/03-personas.md` | — |
| `CHECKPOINT.md`, `svelte5-runes.md`, `transcript-paths.md` | dated checkpoint and topic notes | those topics |

**Promoted, not archived:** `production-readiness.md` became
[`../roadmap/07-quality.md`](../roadmap/07-quality.md) (measured gaps A–H → the gap
register); `01-vision.md`/`02-objectives.md` moved to [`../prd/`](../prd/README.md).

## The rule

A file is **moved** (same bytes, new path), **folded** (merged elsewhere; the
original stamped and moved here), or **superseded** (stamped and moved, no
merge). No `rm`. If you are about to "clean up" a doc, it goes here instead.