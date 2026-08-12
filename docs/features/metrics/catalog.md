---
name: Metrics catalog — per-metric detail
type: feature-detail
kind: functional
module: project
parent: ./feature.md
status: consolidated
updated: 2026-08-08
source_of_truth: ../../analysis/2026-08-04-metrics-catalog.md
tags: [metrics, catalog, calculation, source, representation]
---

# Metrics catalog — per-metric detail

Every metric sensei computes, expanded uniformly: **how it's calculated, what it
reads, and how it's represented.** This is the detailed companion to
[`feature.md`](./feature.md) (the why + philosophy) and the queryable form of the
[metrics catalog analysis](../../analysis/2026-08-04-metrics-catalog.md) (the
buildable source of truth — formulas, source columns, live coverage).

**Entry template** — each metric carries:

- **Facets** — `family · type · direction · cadence` (the machine fields that
  seed `sensei.metrics`; the UI colours/groups from these, never from a hardcoded
  name).
- **Definition** — what it measures, in one line.
- **Calculation** — the formula.
- **Source** — the `table.column(s)` it reads + live coverage (2026-08-04).
- **How to read** — interpretation: the good/bad direction, its companion metric,
  thresholds, and the gotcha that makes it lie if read alone.
- **Representation** — how it's shown (the answer to "how do we represent this").
- **Status** — `live` (computable today) · `blocked` (empty column / missing table).

**Representation vocabulary** (shared so the surfaces stay consistent):

| Shape | Use for | Example |
|---|---|---|
| **stat chip + trend arrow** | a headline ratio/pct with direction | FTR "63% ▲ +8 pts" |
| **paired chip** | a velocity metric + its quality companion, always together | FTR ‖ rework ratio |
| **trend line** (day/week/month/quarter) | anything with a time series, apply-event annotated | FTR over 12 weeks |
| **gauge / threshold bar** | a bounded ratio with a "healthy band" | cache-hit %, run-completion |
| **Pareto bar** | concentration (few items dominate) | churn concentration |
| **heatmap (module × metric)** | per-module quality, to locate friction | rework density by module |
| **money strip** | currency, with measured/estimated tag | cost-of-rework "$45.8k" |
| **empty state** | a catalogued metric with no data yet | "not yet measured" |
| **health dial** | the composite 0–100 project score | "health 72 ▲ +4" |

> Direction legend: ▲ = higher is better, ▼ = lower is better, ● = neutral
> (context-dependent — read with its companion).

---

## Outcome — was the work right?

### First-turn resolution (FTR) `ftr`
- **Facets:** outcome · pct · ▲ · session, daily, project
- **Definition:** share of sessions completed without a correction turn.
- **Calculation:** `count(sessions where ftr=true) / count(sessions)` over the window.
- **Source:** `activity.sessions.ftr` — **96% populated** (66/69).
- **How to read:** the north-star. Everything is judged by whether it moves FTR.
  Never read alone — a high FTR with rising rework means work is being *deferred*,
  not done. Companion: **rework ratio**. Friction concentrates in big
  multi-surface repos, so always drill to the **per-module** FTR.
- **Representation:** paired stat chip (FTR ‖ rework) + a trend line annotated
  with apply-events (did adopting a memory/rule move it?).
- **Status:** live (the one metric that already works).

### Rework ratio `rework_ratio`
- **Facets:** outcome · ratio · ▼ · daily, project
- **Definition:** share of all tool-calls that came from corrected sessions.
- **Calculation:** `Σ tool_calls in sessions where outcome='corrected' / Σ tool_calls`.
- **Source:** `activity.sessions.outcome` + `activity.turns.tool_calls` — live (**0.76** today).
- **How to read:** FTR's mandatory companion — raw volume is an *inverted*
  productivity signal (8 of the top-10 sessions by volume are the worst). High +
  rising = effort is going into fixing, not building.
- **Representation:** the ▼ half of the FTR paired chip; trend line beneath FTR.
- **Status:** live.

### Cross-session reopen rate `reopen_rate`
- **Facets:** outcome · ratio · ▼ · project, per-module (folder)
- **Definition:** files/modules corrected in more than one session ÷ files touched.
- **Calculation:** `distinct files with is_correction in ≥2 sessions / distinct files touched`.
- **Source:** `activity.turns.is_correction` + `sessions.module`/`folder_id` — partial.
- **How to read:** a module that keeps reopening is an architecture/knowledge
  smell, not a one-off. Localizes friction to a module.
- **Representation:** module × reopen heatmap; feeds the atlas "needs attention."
- **Status:** blocked (partial — needs module attribution on turns).

### Regression / reopen rate `regression_rate`
- **Facets:** outcome · ratio · ▼ · project, weekly
- **Definition:** resolved doc↔code drift pairs that flipped back to broken ÷ resolved pairs.
- **Calculation:** `count(drift pairs current→broken after resolved) / count(resolved pairs)`.
- **Source:** `inference.drift_items(doc,code,status)` — **blocked** (insert-only; no `resolved_at`/history).
- **How to read:** the DORA change-failure analogue, first-party. Needs the drift
  table to upsert with `resolved_at` + `break_count` first.
- **Representation:** trend line; a spike is an alert on the impact surface.
- **Status:** blocked (drift table insert-only — catalog P1 #5).

### Run-completion rate `run_completion`
- **Facets:** outcome · ratio · ▲ · account, weekly
- **Definition:** autonomous runs reaching `done` ÷ runs started.
- **Calculation:** `count(runs status='done') / count(runs)`.
- **Source:** `activity.runs.status` — live (**5/9** today, low-N).
- **How to read:** autonomy health. Low-N today — gate the display until enough runs.
- **Representation:** gauge with a low-N "not enough data" state.
- **Status:** live (low-N gated).

---

## Cost — what did it cost?

### Tokens per session `tokens_per_session`
- **Facets:** cost · count · ▼ · session
- **Definition:** Σ input/output/cache tokens over the session's transcript.
- **Calculation:** `Σ usage.{input,output,cache_creation,cache_read}` per session (+ subagent sidechains).
- **Source:** transcript JSONL `usage` → `sessions.tokens_in/out` + new `cache_*` — **0/69 in DB; 58/67 recoverable from disk.**
- **How to read:** cache reads are ~97.6% of consumption — a char proxy lands
  ~20,000× low, so it can rank but never size. Tag every value measured/estimated.
- **Representation:** stat chip with a measured/estimated badge; feeds cost.
- **Status:** blocked (transcript-usage capture — catalog P0 #1).

### Cost-of-rework `cost_of_rework`
- **Facets:** cost · currency · ▼ · daily, project
- **Definition:** total equivalent cost incurred by corrected sessions.
- **Calculation:** `Σ (tokens_x/1e6 × price_x) over sessions where outcome='corrected'`.
- **Source:** tokens (above) + `gateway.model_prices` (∅) + `sessions.outcome` — **71% recoverable ($45.8k of $64.8k).**
- **How to read:** the headline the data already supports — **non-FTR sessions
  cost 4.7×** ($2,411 vs $512). Rework is the dominant *cost* line, not just a
  quality story. **Fail closed on a price miss** (money-facing — never default a rate).
- **Representation:** money strip beside the FTR trend ("rework cost this month, ↓/↑").
- **Status:** blocked (needs token capture + a `gateway.model_prices` table — P0).

### Cache-hit ratio `cache_hit_ratio`
- **Facets:** cost · pct · ▲ · session
- **Definition:** cache-read tokens ÷ total tokens.
- **Calculation:** `cache_read / total_tokens`.
- **Source:** transcript usage — recoverable (**97.6%** overall).
- **How to read:** high is good (cheap reuse); a drop signals context churn.
- **Representation:** gauge with a healthy band.
- **Status:** blocked (transcript-usage capture — P0).

### Cost per FTR-point `cost_per_ftr`
- **Facets:** cost · currency · ▼ · project
- **Definition:** project cost ÷ FTR — what a unit of "right work" costs.
- **Calculation:** `Σ equiv_cost / ftr`.
- **Source:** cost (above) + `sessions.ftr`.
- **How to read:** ties spend to outcome; the client-facing efficiency number.
- **Representation:** money strip; comparative bar across projects.
- **Status:** blocked (after the price table).

---

## Velocity — how much *right* work?

### Throughput `throughput`
- **Facets:** velocity · count · ▲ · daily
- **Definition:** completed sessions per day (and, later, features/day).
- **Calculation:** `count(sessions) per day`; `count(feature_done events) per day`.
- **Source:** `activity.sessions`, `run_events` — live for sessions.
- **How to read:** only meaningful **paired with FTR** — throughput of corrected
  work is negative value. Never report LOC/edit-count as velocity.
- **Representation:** trend line, always stacked under the FTR trend.
- **Status:** live (sessions); features blocked on run adoption.

### Complexity-weighted velocity `effective_velocity`
- **Facets:** velocity · value · ▲ · session, project
- **Definition:** complexity-weighted graph-delta over *completed* sessions ÷ active time.
- **Calculation:** `Σ (kind_weight × (1+degree)) over touched nodes in completed sessions / active_time`.
- **Source:** new `session_graph_delta` × `nodes.kind`/`nodes.degree` — **blocked** (`degree` 0/476,988; no delta join).
- **How to read:** rewards meaningful change, not volume. The honest velocity number.
- **Representation:** trend line; per-session sparkline in the sessions digest.
- **Status:** blocked (populate `nodes.degree` + a per-session graph-delta — catalog P1 #9).

### Feature-completion rate `feature_completion`
- **Facets:** velocity · ratio · ▲ · run, project
- **Definition:** `feature_done` events ÷ features planned.
- **Calculation:** `count(run_events feature_done) / count(plan_graph features)`.
- **Source:** `run_events.kind`, `runs.plan_graph` — partial (9 runs).
- **Representation:** progress bar per run; roll-up trend per project.
- **Status:** blocked (run adoption).

---

## Quality — is the code healthy?

### Duplication ratio `duplication_ratio`
- **Facets:** quality · ratio · ▼ · project, sampled/date
- **Definition:** distinct duplicated source lines ÷ total source lines, from a `qlty` scan at each sampled commit.
- **Calculation:** `Σ distinct duplicated lines / total physical source lines` at a `git worktree` checked out to the commit as-of each sampled day. The numerator is the union (deduped per file, so the ratio stays in `[0,1]`) of the physical line ranges `qlty smells --sarif` flags as `identical-code`/`similar-code`; the denominator is the `qlty metrics --all` TOTAL `lines`. Task group `quality`.
- **Source:** a `git worktree` at the commit as-of the sampled day + `qlty smells` (numerator) & `qlty metrics` (denominator). **Supersedes** the former own-graph `find_duplicates_scoped` embedding snapshot (retired — no second live source for this key).
- **How to read:** rising duplication = DRY erosion, now measured over REAL history
  (backfilled per sampled commit-day) rather than a current-graph snapshot. Sampled
  ~weekly (one commit-day per ISO week) to bound scan cost. A non-git project, an
  absent `qlty` CLI, or a commit predating the repo's `.qlty` config → no row
  (honest-empty, never fabricated). Project-level; per-module (folder) attribution is
  a deferred follow-up.
- **Representation:** trend line over sampled commit-days.
- **Status:** live (git-worktree + qlty, sampled cadence; superseded the own-graph symbol snapshot). `qlty` is an OPTIONAL tool — absent → honest-empty.

### Churn concentration `churn_concentration`
- **Facets:** quality · pct · ● · project
- **Definition:** share of the day's line-churn absorbed by the busiest 20% of files (Pareto).
- **Calculation:** `Σ line-churn(top 20% files) / Σ line-churn(all files)` per commit-day,
  where a file's daily line-churn is `Σ(added + deleted)` across that day's commits
  (git `--numstat`). Top set = the busiest `ceil(20%)` files. A commit-day with zero
  line-churn (only binary/mode changes) has no denominator ⇒ NO row (never a 0/0).
- **Source:** `git log --numstat` per commit-day for the project's git-root
  (`folders.abs_path` via `project_root_path`) — live.
- **How to read:** high concentration = a few hotspots absorb the change — the
  files to refactor first. Neutral in isolation; actionable as a target list.
- **Representation:** **Pareto bar** + the hotspot file list ("refactor these first").
- **Status:** live (git-sourced, backfilled per commit-day; the earlier
  rescan-inflation caveat no longer applies — git, not the indexing feed, is the source).

### Churn rate `churn_rate`
- **Facets:** quality · count · ▼ · daily, project
- **Definition:** distinct source files changed per day (files-changed-over-time, from git).
- **Calculation:** `count(distinct file paths touched by the day's commits)` via
  `git log --no-merges --numstat` per commit-day (committer date, `--date=short`).
- **Source:** `git log` for the project's git-root (`folders.abs_path` via
  `project_root_path`) — live. Non-git project / no commits that day → no row (honest-empty).
- **How to read:** GitClear's churn *definition*, measured first-party from git.
  Pair with duplication. A per-day count; a real files-changed timeline, backfilled
  over the whole git history.
- **Representation:** trend line.
- **Status:** live (git-sourced, per commit-day; the version-rescan inflation no
  longer applies — the old `activity.task_executions` indexing-feed source is retired).

### Rework density `rework_density`
- **Facets:** quality · ratio · ▼ · project, per-module (folder)
- **Definition:** files flagged `rework:` ÷ project files.
- **Calculation:** `count(files in detected_patterns rework:) / count(project files)`.
- **Source:** `inference.detected_patterns` — live.
- **How to read:** where correction-proneness lives; the per-module friction map
  (multi-surface repos concentrate it — the friction-analysis finding).
- **Representation:** **module × metric heatmap** — the primary "locate friction" visual.
- **Status:** live.

### Drift MTTR `drift_mttr`
- **Facets:** quality · duration · ▼ · project
- **Definition:** mean time from drift detected to resolved.
- **Calculation:** `avg(resolved_at − detected_at)` over drift pairs.
- **Source:** `inference.drift_items` — **blocked** (insert-only; no `resolved_at`).
- **Representation:** duration stat + trend.
- **Status:** blocked (drift upsert — P1 #5).

### Quality-delta `quality_delta`
- **Facets:** quality · value · ▲ · session
- **Definition:** scanner score at session-end − session-start (lint/complexity/coverage).
- **Calculation:** `score(end) − score(start)` from a qlty.sh/scc scan.
- **Source:** new `quality_snapshots` table — **blocked** (no scanner in loop).
- **Representation:** per-session delta chip (green/red); trend.
- **Status:** blocked (P2 #10; see the quality-metrics blueprint).

### Maintainability `module_quality`
- **Facets:** quality · ratio · ▼ · project, sampled/date
- **Definition:** maintainability-smell burden ÷ total source lines, from a `qlty` scan at each sampled commit.
- **Calculation:** `count(qlty non-duplication smells) / total physical source lines`
  at a `git worktree` checked out to the commit as-of each sampled day — the smells are
  `qlty`'s file/function-complexity, deep-nesting, long-parameter-list, … findings
  (`qlty smells --sarif` minus the duplication findings); the denominator is the
  `qlty metrics --all` TOTAL `lines`. Task group `quality`, alongside `duplication_ratio`.
- **Source:** a `git worktree` at the sampled commit + `qlty smells` & `qlty metrics`.
- **How to read:** higher = more maintainability smells per line (lower is better),
  backfilled over real history at a sampled (~weekly) cadence. **Coverage is
  deliberately OUT of scope here:** a historical worktree has no coverage artifact
  (lcov), so coverage is left honest-empty (no row) — never a synthesized number (we do
  NOT run tests per-worktree). History rows are project-level; per-module (folder)
  attribution and an A–F grade mapping are deferred follow-ups.
- **Representation:** trend line over sampled commit-days; (future) module × metric heatmap.
- **Status:** live for maintainability (git-worktree + qlty, sampled cadence); coverage
  stays out-of-history-scope (honest-empty); per-module attribution deferred. `qlty` is
  an OPTIONAL tool — absent → honest-empty.

---

## Autonomy — how much babysitting?

### Interruption rate `interruption_rate`
- **Facets:** autonomy · ratio · ▼ · session
- **Definition:** Stop events ÷ UserPromptSubmit events.
- **Calculation:** `count(Stop) / count(UserPromptSubmit)` from the event stream.
- **Source:** `activity.assistant_events.event_type` — live (**0.96** today).
- **How to read:** high = the human keeps stepping in — the "babysitting" signal.
- **Representation:** gauge; trend.
- **Status:** live.

### Resume-success rate `resume_success`
- **Facets:** autonomy · ratio · ▲ · account
- **Definition:** runs resumed after a limit ÷ runs that hit a limit.
- **Calculation:** `count(runs resumed) / count(runs paused_on_limit)`.
- **Source:** `run_events(paused_on_limit, resumed)` — ≈0 (fires ×1).
- **Representation:** gauge, low-N gated.
- **Status:** blocked (run/limit signals — catalog P0/§C Tier-4).

### Autonomy ratio `autonomy_ratio`
- **Facets:** autonomy · ratio · ▲ · run, session
- **Definition:** turns advanced without a human prompt ÷ total turns.
- **Calculation:** `count(turns with no preceding UserPromptSubmit) / count(turns)`.
- **Source:** `assistant_events`, `run_events` — partial.
- **Representation:** gauge; per-run sparkline.
- **Status:** blocked (partial).

### False-crash rate `false_crash_rate`
- **Facets:** autonomy · ratio · ▼ · account
- **Definition:** runs killed at the recovery cap that were actually just waiting.
- **Calculation:** `count(runs killed-at-cap but waiting) / count(non-done runs)`.
- **Source:** `runs.recovery_attempts`, `run_events.detail` — live (**4/4** non-done).
- **Representation:** count + a "why" drill.
- **Status:** live (low-N).

---

## Knowledge — is it learning?

### Memory-promotion rate `memory_promotion`
- **Facets:** knowledge · ratio · ▲ · weekly
- **Definition:** memories created ÷ eligible patterns/corrections (`instance_count ≥ 3`).
- **Calculation:** `count(memories created) / count(eligible patterns+corrections)`.
- **Source:** `memories`, `detected_patterns`, `corrections` — live but **≈0 today**.
- **How to read:** ≈0 **is the signal** — sensei's own distill loop is silent (1
  correction cluster, 0 learned rules from 21 corrections). This is the metric that
  proves the **measure→distill→govern loop** (see `feature.md`) is or isn't closing.
- **Representation:** trend + a "loop health" callout when it flatlines at 0.
- **Status:** live (measures a stalled pipeline — the point).

### Recall-hit rate `recall_hit`
- **Facets:** knowledge · ratio · ▲ · session
- **Definition:** sessions loading ≥1 relevant memory ÷ sessions.
- **Calculation:** `count(sessions with a memory_load) / count(sessions)`.
- **Source:** `activity.memory_loads.session_id` — **blocked** (NULL on all 24).
- **Representation:** gauge.
- **Status:** blocked (fix memory_loads session linkage — P1 #7).

### Repeat-mistake rate `repeat_mistake`
- **Facets:** knowledge · ratio · ▼ · project
- **Definition:** corrections whose signature recurs across sessions ÷ corrections.
- **Calculation:** `count(corrections with recurring signature) / count(corrections)`.
- **Source:** `inference.corrections`, `turns.is_correction` — **blocked** (extractor stalled).
- **How to read:** the same miss twice = a rule that should exist (maps to the six
  friction miss-patterns). Directly feeds candidate rules.
- **Representation:** ranked list of recurring signatures → "promote to rule."
- **Status:** blocked (revive corrections extractor — P1 #6).

### Guidance-adherence `guidance_adherence`
- **Facets:** knowledge · ratio · ▲ · session
- **Definition:** `used` tool-verdicts ÷ classified, on recalled guidance.
- **Calculation:** `count(verdict='used') / count(classified verdicts)`.
- **Source:** `tool_call_verdicts` — partial (verdict is fragment-overlap today).
- **Representation:** gauge; per-guidance drill.
- **Status:** blocked (outcome-based verdicts — see Tool).

---

## Tool / content utility — is the tooling used?

### Unused-tool count `unused_tools`
- **Facets:** tool · count · ▼ · weekly
- **Definition:** registered tools with 0 successful outcomes in the window.
- **Calculation:** `count(tools with 0 outcome-positive calls in 14d)`.
- **Source:** `assistant_tools` + verdicts — live.
- **How to read:** dead surface area; aggregate ("40 tools dormant"), never 40 cards.
- **Representation:** single summary stat (aggregated), not a list of cards.
- **Status:** live.

### Outcome utility `outcome_utility`
- **Facets:** tool · ratio · ▲ · tool, weekly
- **Definition:** tool calls followed by an edit/state-change/phase-unblock ÷ calls.
- **Calculation:** `count(calls with a following state-change) / count(calls)`.
- **Source:** `assistant_events`, `run_events` (new join) — **blocked** (current metric = fragment overlap).
- **How to read:** replaces the weak fragment-overlap verdict with a real outcome.
- **Representation:** per-tool utility bar.
- **Status:** blocked (event-join — catalog A/Tool).

### Registry coverage `registry_coverage`
- **Facets:** tool · pct · ▲ · project
- **Definition:** content items with provenance+version+utility+leak-status ÷ items.
- **Source:** new `content_registry` — blocked (5 disjoint silos).
- **Status:** blocked (P2 #11).

### Leak-scan pass rate `leak_scan_pass`
- **Facets:** tool · pct · ▲ · on re-probe
- **Definition:** manifests/skills/agents passing secret+grant scan ÷ scanned.
- **Source:** new `leak_scan_status` — blocked.
- **Status:** blocked (P2 #11).

---

## Composite — overall health

### Project health score `project_health` — RETIRED
- **Facets:** composite · score · ▲ · daily
- **Definition:** a single 0–100 roll-up of the project's active metrics.
- **Calculation:** each active metric's latest daily value normalized to [0,1] by
  its `direction` (`higher_better` → v; `lower_better` → 1−v; counts/durations
  against a registry `target`), combined by `metrics.weight`, ×100.
- **Source:** `sensei.project_metrics` (the day's rows) — computed by the `health`
  task after the base tasks; written only when ≥1 component has a value.
- **How to read:** the "is this project healthy?" glance; drill into the
  components to see what's dragging it. Not a substitute for FTR — a summary of it
  and its companions.
- **Representation:** **health dial** + a component breakdown; trend line.
- **Status:** **retired** (`effective_until: 2026-08-12`). A plain mean of ~9
  normalized components is not a meaningful signal — it moved 46→44 purely because
  `throughput` dipped on a quiet day and it was dragged by a bogus `churn_rate`
  proxy while FTR/rework were perfect. **`retire_reason`:** *composite mean is not a
  meaningful signal; FTR is the north star, and a qlty-based code-quality family
  replaces the code-health role.* The metric's schema + handler are kept — it is
  revivable by clearing `effective_until` in the seed. Retirement takes effect via
  the registry's `effective_until` window (seeded from this catalog through
  `staging.metrics` + `import_metrics()`); once inactive it is not scheduled, and
  `compute_health` writes no `project_health` row (honest-empty, never fabricated).
  Historical rows already in `sensei.project_metrics` are left intact.

---

## Coverage summary

| Family | Live today | Blocked (and on what) |
|---|---|---|
| Outcome | FTR, rework ratio, run-completion | reopen (module attribution), regression (drift upsert) |
| Cost | (cost-of-rework 71% recoverable) | tokens/price/cache/cost-per-FTR — transcript capture + price table (P0) |
| Velocity | throughput (sessions) | effective velocity (`degree`+delta), feature-completion (runs) |
| Quality | duplication (qlty), maintainability (qlty), churn, churn-concentration, rework-density | drift-MTTR (drift upsert), quality-delta (scanner), module_quality coverage + per-module (out-of-history-scope / deferred) |
| Autonomy | interruption, false-crash | resume-success, autonomy-ratio |
| Knowledge | memory-promotion (≈0 = the signal) | recall-hit (memory_loads), repeat-mistake (extractor), guidance-adherence |
| Tool | unused-tools | outcome-utility, registry-coverage, leak-scan |

The headline: **most blocked metrics are empty columns, not missing
concepts** — sensei has the schema, it isn't fed. The P0 instrumentation backlog
(feed transcript usage, populate `degree`, make drift upsert, revive the
corrections/memory loop) unblocks the majority — see the
[catalog analysis §D](../../analysis/2026-08-04-metrics-catalog.md).

## Related

- [[features/metrics/feature]] (why + philosophy) · [[spec/pipeline/metrics]] (design) ·
  [[analysis/2026-08-04-metrics-catalog]] · [[spec/pipeline/ftr]]
