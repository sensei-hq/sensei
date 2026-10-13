# Traceability & Doc Drift · M11

**Plane:** insight · **Status:** [roadmap/02-insight.md](../../../../roadmap/02-insight.md) ·
**Serves:** solo developer ([persona 1](../../../03-personas.md#1--solo-developer--the-wedge)),
team developer ([persona 2](../../../03-personas.md#2--team-developer--the-inheritor))

## Purpose

Traceability answers *"do the docs still match the code?"* When a README says
`` `LoginService.authenticate()` `` and the codebase has since renamed it to
`authenticateWithCredentials`, the doc is lying. The drift scanner reads
backtick-wrapped identifier mentions in doc nodes and cross-references them against
the live code graph, emitting **drift items** with a confidence split: high (the code
identifier is gone; auto-fixable on an unambiguous rename), medium (it exists but the
signature changed), low (possible false positive, review-batch). The same machinery
produces bidirectional doc↔code links and doc-doctor lint findings (stale docs,
undocumented public, ambiguous references). The user feels this module as *sensei
catches the doc lie before the next reader does* — "Doc drift: 3".

> **Status honesty.** The deletion-of-symbol scanner is real and careful — but as of
> 2026-08-05 `traces_to` = 0 rows, `covers` is file-stem proximity (not
> identifier-level), drift is doc-**file** level (0 `section` nodes), and
> `expected/actual_signature` are NULL everywhere. Requirement-level traceability
> depends on the M4 code-graph fixes (`section` nodes, deterministic ids).

## In scope / out of scope

- In: `scan_project_doc_drift` (triggered on doc change, code change, or daily full
  refresh); identifier extraction + signature comparison; drift-item lifecycle
  (`open → fixed → resolved_auto`, dismiss); apply-fix endpoint; branch-scoped
  drift lists; bidirectional `trace_links`; doc-doctor lint findings + enforcement
  modes (audit-only / warn / block); auto-generated doc stubs.
- Out: `section`-node granularity of the code graph (M4 — the prerequisite for
  requirement-level traceability); library-upgrade drift surfacing (M12 — reuses the
  scanner); the code graph itself (M4); narration-cache copy (M7-owned pipeline).

## Surfaces

- **API** — `POST /api/traceability/{id}/apply-fix`; Project Overview `Doc drift`
  stat; Observatory + Project Traceability screens; expansion drawer via
  `get_trace_links(node)`.
- **Daemon** — `pg_store.rs::scan_project_doc_drift`, `analysis/doc_drift.rs`
  (identifier extraction — unit-tested, no DB round-trip).
- **App** — Traceability screens (list, filter by confidence/state, Expected-vs-Actual
  drawer); Insights violation cards for high-confidence drift; /sensei:validate
  optional gate.

## Data invariants

- `sensei.drift_items` — one row per broken reference: `doc_node_id` + `doc_path` +
  `line_number` + `mentioned_identifier`; `expected_signature` / `actual_signature`
  nullable; `confidence` enum `high | medium | low`; `state` enum
  `open | dismissed | fixed | resolved_auto`; `suggestion` (narration-cache
  `drift_fix`); `detected_at` / `resolved_at`.
- **Idempotent scan.** Same doc scanned twice produces the same rows.
- **Branch-scoped.** Drift items record against the active branch (M3 branch
  versioning); a branch switch flips the list.
- **Self-healing.** A row whose identifier resolves back to a live node transitions
  to `fixed` at the next scan without user action; rows persist briefly so Impact can
  credit the fix.
- **Auto-fix bar is strict.** Auto-fix only for high-confidence unambiguous renames:
  `follow_count ≥ N` AND `name_similarity ≥ 0.85`.
- `sensei.trace_links` — one row per doc↔code link (`direction`,
  `source_node_id`, `target_node_id`, `confidence`, `detected_at`), both directions.
- `sensei.doc_lint_findings` — `severity error | warning | info`; enforcement modes
  per project (default `warn`); compliance packs may raise specific rules to `block`.
- **Stopwords.** `` `TODO` ``/`` `TBD` `` must never become drift items.

## Signals shown

| Element | Meaning | Worked example |
|---|---|---|
| Project Overview `Doc drift` stat | `count(*) … where state='open' and project_id=X` | "Doc drift: 3" |
| High-confidence drift | identifier gone (or unambiguous rename) | README: `authenticate()` → `authenticateWithCredentials` |
| Medium-confidence drift | identifier exists, signature changed | Expected-vs-Actual drawer |
| `get_trace_links(node)` | both directions around a node | "3 docs reference `RefreshTokenStore`" |
| Doc-doctor findings | stale / undocumented / inconsistent / broken | severity-filtered on the Traceability screen |

## Done gate

- `scan_project_doc_drift` returns `{broken_added, resolved, docs_scanned}` and the
  counts stay stable across ticks when nothing changed.
- A README rename `foo()` → `bar()` produces a high-confidence drift within one tick
  of either doc or code update; with clear git-log evidence + auto-fix enabled, the
  doc is rewritten; otherwise it surfaces as a suggested-fix card.
- The Project Overview stat equals the Traceability screen's open-count for the same
  project — the aggregate and the list never diverge.
- Branch switch flips the drift list; a resolved row advances to `resolved_auto` on
  the next scan; `expected` and `actual` are never both NULL on a rendered card.
- False positives stay out: `` `TODO` ``-class words never drift.

```bash
psql -A -t -c "select confidence, state, count(*)
                 from sensei.drift_items
                 where project_id = (select id from sensei.projects where name='sensei')
                 group by confidence, state" -d sensei
```

## Wrong gate

- **Stat shows 3, list shows 5** — aggregate and list queries diverged.
- **Auto-fix rewrites a doc on weak git-log evidence** — threshold too loose.
- **Drift items persist after the identifier reappears** — resolve pass not running.
- **Branch switch leaves the previous branch's drift** — filter regressed.
- **Same identifier drifts in every doc that mentions it** — dedup across docs
  missing when the underlying change is uniform.
- **`expected`/`actual` both NULL on a "broken" card** — useless card.
- **Force-of-habit: section-level traceability claimed before M4 ships `section`
  nodes** — must read as partial (doc-file level) until the graph provides it.

## Scenarios

- [`scenarios/traceability.feature`](scenarios/traceability.feature) — scan triggers,
  confidence tiers, apply-fix, links, lint.

## Open gaps

- Requirement-level (`section`-node) traceability is gated on the M4 code-graph fixes
  — see [02-insight.md](../../../../roadmap/02-insight.md) and the indexer capability
  roadmap (`history/analysis/2026-08-05-indexer-capability-coverage.md`). No
  dedicated register row beyond that.

## Related

- [spec/pipeline/traceability](../../../../spec/pipeline/traceability.md) — the
  scanner, bidirectional links, doc-doctor, enforcement modes (legacy depth, folds
  here; carries the roadmap/partial banner).
- [spec/pipeline/capture](../../../../spec/pipeline/capture.md) — branch versioning;
  [spec/pipeline/analyzer](../../../../spec/pipeline/analyzer.md) — scan scheduling.
- [spec/pipeline/library-intelligence](../../../../spec/pipeline/library-intelligence.md) —
  library-upgrade drift reuse.
- [history/analysis/2026-08-05-indexer-capability-coverage](../../../../history/analysis/2026-08-05-indexer-capability-coverage.md).
- [features/03-observatory.md](../../../../features/03-observatory.md) — Traceability
  section status.
- [prd/02-objectives.md](../../../02-objectives.md) · [prd/01-vision.md §The core loop](../../../01-vision.md#the-core-loop).
- Crate: `crates/senseid/src/{db/pg_store.rs,analysis/doc_drift.rs}`.