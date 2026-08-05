# Metrics Catalog & Instrumentation Backlog

_2026-08-04 · the "what to measure and how" companion to
[metrics.md](2026-08-04-metrics.md) and [Observations.md](2026-08-04-Observations.md)_

This is the buildable spec: every proposed metric with a definition, a formula, the source
column, a cadence, and — honestly — whether the data exists today. It closes with the
token/cost model and the prioritized instrumentation backlog. The ordering principle is the
mandatory rule **"measure, then keep what helps"**: you cannot keep what you cannot compute,
so the empty columns come first.

## How to read the cadence + source columns

- **Cadence** — the grain the metric is rolled up at: `session` (per analyzed session),
  `daily`, `project`, `run` (autonomous run), `tool`, or `account`.
- **Source** — the table.column(s) it reads. `∅` means the column/table does not exist yet.
- **Coverage today** — % of the source populated, from the live DB on 2026-08-04.

---

## A. Measurability matrix (do we have the data?)

The gating question for every metric below. `Y` = computable now; `partial` = computable at
low coverage or after a cheap backfill; `N` = blocked on an empty column or a missing table.

| Metric family | Metric | Data present? | Blocking gap |
|---|---|---|---|
| Outcome | FTR | **Y** | — (`sessions.ftr`, 96% populated) |
| Outcome | Rework Ratio | **Y** | — (`sessions.outcome` + `turns.tool_calls`) |
| Outcome | Regression / reopen rate | **partial** | `drift_items` insert-only (no signature/`resolved_at` upkeep); no `module_stability` |
| Cost | tokens / session | **partial** | `sessions.tokens_*` 0/69 — but exact usage on disk (transcripts) |
| Cost | cost / feature, cost-of-rework | **N** | no price table (`gateway.model_prices` ∅); no feature unit |
| Velocity | complexity-weighted graph-delta | **N** | `nodes.degree` 0/476,988; no session→graph-delta join |
| Velocity | feature completion rate | **partial** | only 9 runs use the feature model; sessions carry no WorkItem |
| Quality | duplication ratio | **Y** | — (`get_duplicates` + `nodes` symbol reuse) but not run at write time |
| Quality | churn rate / concentration | **Y** | — (`task_executions`), though inflated by the rescan bug |
| Quality | drift MTTR | **N** | `drift_items.resolved_at` not maintained (insert-only) |
| Quality | quality-delta (lint/complexity/coverage) | **N** | no scanner in loop; `project_quality_signals.test_pass_rate` 0/128 |
| Autonomy | run completion / resume-success | **partial** | `runs` only 9 rows; resume never fires (`paused_on_limit` ×1) |
| Autonomy | interruption rate | **Y** | — (`assistant_events` Stop / UserPromptSubmit) |
| Knowledge | memory-promotion rate | **Y** (≈0) | pipeline stalled: 11 memories, `corrections` 1 row |
| Knowledge | recall-hit / repeat-mistake | **N** | `memory_loads.session_id` NULL on all 24 |
| Tool | outcome-based utility | **partial** | current verdict is fragment-overlap, not outcome; identity unstable |

**The pattern:** most red cells are *empty columns or insert-only tables*, not missing
concepts. sensei already has the schema; it isn't fed. Section D is the backlog to feed it.

---

## B. Metric definitions

### Outcome

| Metric | Definition / formula | Source | Cadence | Coverage |
|---|---|---|---|---|
| **FTR** | share of sessions completed without a correction turn | `sessions.ftr` | session, daily, project | 96% (66/69) |
| **Rework Ratio** | corrected-session tool-calls ÷ all tool-calls | `sessions.outcome`, `turns.tool_calls` | daily, project | Y — currently **0.76** |
| **Cross-session reopen rate** | files/modules corrected in >1 session ÷ files touched | `turns.is_correction`, `sessions.module`/`folder_id` | project | partial |
| **Regression rate** | drift pairs that flipped `current→broken` after being resolved ÷ resolved pairs | `drift_items(doc,code,status)` | project, weekly | N (needs upsert + history) |
| **Run completion rate** | runs reaching `done` ÷ runs started | `runs.status` | account, weekly | Y — currently **5/9** |

Notes. FTR is the one metric that already works; keep it. Rework Ratio is its most
important companion because raw volume is *inverted* as a productivity signal
([metrics §7](2026-08-04-metrics.md#7-productivity--velocity)) — always show the two together.

### Cost / effort

| Metric | Definition / formula | Source | Cadence | Coverage |
|---|---|---|---|---|
| **Tokens / session** | Σ `usage.{input,output,cache_creation,cache_read}` over the session's transcript messages (+ subagent sidechains) | transcript JSONL → `sessions.tokens_in/out` (+ new `cache_*`) | session | 0/69 in DB; **58/67 recoverable from disk** |
| **Equivalent cost** | Σ `tokens_x / 1e6 × price_x` | tokens × `gateway.model_prices` (∅) | session, project, account | N (no price table) |
| **Cost-of-rework** | Σ cost over `outcome='corrected'` sessions | above + `sessions.outcome` | daily, project | recoverable — **$45.8k of $64.8k (71%)** |
| **Cache-hit ratio** | `cache_read ÷ total_tokens` | transcript usage | session | recoverable — **97.6%** overall |
| **Cost / FTR-point** | project cost ÷ FTR | above + `sessions.ftr` | project | after price table |

The headline the data already supports: **non-FTR sessions cost 4.7× more** ($2,411 vs $512).
Rework is not just a quality story — it is the dominant cost line.

### Velocity / complexity

| Metric | Definition / formula | Source | Cadence | Coverage |
|---|---|---|---|---|
| **Effective Velocity** | Σ(complexity-weighted graph-delta over *completed* sessions) ÷ active-time | new `session_graph_delta` × node-kind/`degree` weights | session, project | N (delta join + `degree` missing) |
| **Complexity of a change** | Σ over touched nodes of `kind_weight × (1+degree)` | `nodes.kind`, `nodes.degree` | per change | N (`degree` 0/476,988) |
| **Feature completion rate** | `feature_done` events ÷ features planned | `run_events.kind`, `runs.plan_graph` | run, project | partial (9 runs) |
| **Throughput** | completed sessions/day, features/day | `sessions`, `run_events` | daily | Y for sessions |

Never report LOC or edit-count as velocity — 8 of the top-10 sessions by volume are the
*worst* (corrected/`ftr=false`).

### Quality

| Metric | Definition / formula | Source | Cadence | Coverage |
|---|---|---|---|---|
| **Duplication ratio** | new symbols matching an existing symbol/embedding ÷ new symbols | `nodes` + `get_duplicates` | session, project | Y (not enforced at write) |
| **Churn rate** | `process_file` executions/day per file (source only) | `task_executions` | daily, project | Y (inflated by rescan bug) |
| **Churn concentration** | share of churn from busiest 20% of files (Pareto) | `task_executions` | project | Y — currently **94%** |
| **Rework density** | `rework:` files ÷ project files | `detected_patterns` | project | Y |
| **Drift MTTR** | mean `resolved_at − detected_at` over drift pairs | `drift_items` | project | N (insert-only) |
| **Quality-delta** | scanner score at session-end − session-start | new `quality_snapshots` (qlty.sh/scc) | session | N (no scanner) |

### Autonomy

| Metric | Definition / formula | Source | Cadence | Coverage |
|---|---|---|---|---|
| **Resume-success rate** | runs resumed after a limit ÷ runs that hit a limit | `run_events(paused_on_limit, resumed)` | account | ≈0 (fires ×1) |
| **Interruption rate** | Stop events ÷ UserPromptSubmit | `assistant_events.event_type` | session | Y — currently **0.96** |
| **Autonomy ratio** | turns advanced without a human prompt ÷ total turns | `assistant_events`, `run_events` | run, session | partial |
| **False-crash rate** | runs killed at `recovery_attempts` cap that were actually waiting | `runs.recovery_attempts`, `run_events.detail` | account | Y — currently **4/4 non-done** |

### Knowledge

| Metric | Definition / formula | Source | Cadence | Coverage |
|---|---|---|---|---|
| **Memory-promotion rate** | memories created ÷ eligible patterns/corrections (`instance_count ≥ 3`) | `memories`, `detected_patterns`, `corrections` | weekly | Y (≈0 today) |
| **Recall-hit rate** | sessions loading ≥1 relevant memory ÷ sessions | `memory_loads.session_id` | session | N (session_id NULL) |
| **Repeat-mistake rate** | corrections whose signature recurs across sessions ÷ corrections | `corrections`, `turns.is_correction` | project | N (extractor stalled) |
| **Guidance-adherence** | `used` tool-verdicts ÷ classified, on recalled guidance | `tool_call_verdicts` | session | partial (see Tool) |

### Tool / content utility

| Metric | Definition / formula | Source | Cadence | Coverage |
|---|---|---|---|---|
| **Outcome utility** | tool calls followed by an edit/state-change/phase-unblock ÷ calls | `assistant_events`, `run_events` (new join) | tool, weekly | N (current metric = fragment overlap) |
| **Unused-tool count** | registered tools with 0 successful outcomes in window | `assistant_tools` + verdicts | weekly | Y |
| **Registry coverage** | content items with provenance+version+utility+leak-status ÷ items | new `content_registry` | project | N (5 disjoint silos) |
| **Leak-scan pass rate** | manifests/skills/agents passing secret+grant scan ÷ scanned | new `leak_scan_status` | on re-probe | N (no column) |

---

## C. Token / cost model

The user's idea was to *infer* tokens from limit-reset messages. The data says something
better: **the exact tokens are already on disk** — no inference needed for 87% of sessions.
Deep-dive [12](2026-08-04-deep-dive/12-token-cost-inference.md).

**Tier 1 — capture (truth).** Every assistant message in the 1,341 Claude Code transcript
JSONL files carries a full `usage` block
(`input_tokens`, `output_tokens`, `cache_creation_input_tokens`, `cache_read_input_tokens`).
The daemon's transcript adapter already opens these files to build `transcript_turns` and
capture the model — it just has no `usage` field on its struct. Add it, sum per session
(join key `sessions.client_session_id` = transcript filename), include subagent sidechains,
and write `sessions.tokens_in/out` + new `cache_write/cache_read` columns. Exact for 58/67
sessions today; 100% for all future sessions. **~1 day of work.**

**Tier 2 — price (fail closed).** No cost table exists anywhere (`gateway.models.props` is
empty). Add `gateway.model_prices(model, input, output, cache_write, cache_read, currency,
effective_from)` and compute `equiv_cost = Σ tokens_x/1e6 × price_x`. Do **not** hardcode
rates in Rust, and **fail closed on a price miss** (money-facing rule — return an error/NULL
state, never a defaulted price).

**Tier 3 — char proxy (gap-filler only).** For sessions with no transcript on disk, estimate
`output_tokens ≈ Σ char_count / 4` from `transcript_turns`, scaled per model family, and tag
the row with a `tokens_source ∈ {measured, estimated}` enum so an estimate is never confused
with truth. **Caveat, logged:** the char proxy lands ~20,000× low against total tokens (it
cannot see cache reads, which are 97.6% of consumption) — it can *rank* sessions, never
*size* them.

**Tier 4 — limit-reset calibration (account grain).** 172 five-hour session-limit hits and
14 weekly-limit hits (fixed Sat-11am-CT boundaries) are in the transcripts. Use them for two
things, not per-session sizing: (a) an account-level equivalent-budget calibration between
reset boundaries; (b) a **run-scheduling signal** — write `limit_reset` rows to `run_events`,
set `runs.pause_reason='usage_limit'` + `paused_until`, so runs reschedule instead of
stalling (ties to [metrics §1](2026-08-04-metrics.md#1-babysitting--roadblocks-autonomy)).

---

## D. Instrumentation backlog (do these first)

Ordered by how many downstream metrics each unblocks. Each item is "populate an existing
column / connect an existing pipeline," not new architecture.

**P0 — unblock the most metrics, cheapest.**
1. **Capture transcript usage** → `sessions.tokens_in/out` + `cache_*`. Unblocks all cost,
   cost-of-rework, cache, cost/FTR. (transcript adapter)
2. **Backfill `sessions.model/provider`** from `SessionStart.model` + transcript capture
   (66.7% recoverable now, wired to never regress). Unblocks all per-model analysis.
3. **Populate `assistant_events.success`** on PostToolUse from the hook payload (0/131,690
   today). Unblocks tool error-rate and reliability.
4. **Make `covers` idempotent** (`ON CONFLICT` + unique index) and **debounce version-rescan**.
   Removes the 918× edge duplication and 8.6× indexing churn — fixes the graph *and* stops
   the largest inert workload. (`pg_store.rs`, `version_rescan.rs`, `classifiers.rs`)

**P1 — close the confirmation loops.**
5. **`drift_items` → upsert** on `(doc_node_id, code_node_id)` with `resolved_at` +
   `break_count`. Unblocks regression rate + drift MTTR + kills UI repetition.
6. **Revive the corrections extractor** (seed from `turns.is_correction`, not the
   notification scraper) and the **memory-promotion sweep**. Unblocks knowledge metrics.
7. **Fix `memory_loads` session linkage** (NULL on all 24). Unblocks recall-hit / repeat-mistake.
8. **Recommendation loop**: add `created_at`; stamp `baseline_ftr`/`current_ftr`/`measured_at`
   on accept/measure. Unblocks insight-action rate + the (currently fabricated) FTR-delta.
9. **Populate `nodes.degree`** during the existing `resolve_edges` pass + add a per-session
   `graph_delta`. Unblocks complexity-weighted velocity.

**P2 — the deterministic anchors.**
10. **`quality_snapshots`** table fed by a qlty.sh/scc scan at session start+end → quality-delta.
11. **`content_registry`** unifying the 5 silos with provenance/version/utility/`leak_scan_status`.
12. **Lifecycle/Health screen** (Churn · Regression · Run-health) with low-N alert gating.

> None of this is speculative. Every P0 is an empty column with a real setter one call away,
> or a missing `ON CONFLICT`. The fastest path to "better metrics" is to stop dropping the
> data sensei already collects.
