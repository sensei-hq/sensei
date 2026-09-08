---
name: Metrics catalog — per-metric detail
type: feature-detail
kind: functional
module: project
parent: ./feature.md
status: consolidated
updated: 2026-09-08
source_of_truth: sensei.metrics (live registry), seeded from database/import/staging/metrics.jsonl
tags: [metrics, catalog, calculation, source, representation]
---

# Metrics catalog — per-metric detail

Every metric sensei computes, expanded uniformly: **how it's calculated, what it
reads, and how it's represented.** The detailed companion to
[`feature.md`](./feature.md) (the why + philosophy).

**The registry is the source of truth, not this page.** `sensei.metrics` is
seeded from `database/import/staging/metrics.jsonl` through `staging.metrics` +
`import_metrics()`. Every metric below carries the registry's own `description`,
`purpose`, `how_to_read` and `formula` — if this page and the registry disagree,
the registry wins and this page is the bug. A machine-readable export lives at
[`docs/design/metrics-chart-brief/registry.json`](../../design/metrics-chart-brief/registry.json),
with live coverage in `coverage.json` alongside it.

**Entry template** — each metric carries:

- **Facets** — `family · type · direction · weight` (the machine fields; the UI
  colours/groups from these, never from a hardcoded name).
- **Definition** — what it measures, in one line.
- **Calculation** — the formula.
- **Source** — what it reads + live coverage (2026-09-08).
- **How to read** — interpretation: the direction, its companion metric, and the
  gotcha that makes it lie if read alone.
- **Representation** — how it's shown.
- **Status** — `live` · `low-N` · `never computed` · `retired`.

**Representation vocabulary** (shared so the surfaces stay consistent):

| Shape | Use for | Example |
|---|---|---|
| **stat chip + trend arrow** | a headline ratio/pct with direction | FTR "63% ▲ +8 pts" |
| **paired chip** | a velocity metric + its quality companion, always together | FTR ‖ rework share |
| **trend line** (day/week/month/quarter) | anything with a time series | FTR over 12 weeks |
| **gauge / threshold bar** | a bounded ratio with a healthy band | context reuse, run completion |
| **Pareto bar** | concentration (few items dominate) | churn concentration |
| **heatmap (module × metric)** | per-module quality, to locate friction | rework density by module |
| **money strip** | currency, with measured/estimated tag | cost per result |
| **empty state** | a catalogued metric with no data yet | "not yet measured" |
| **health dial** | the composite 0–100 score | "health 65 ▲ +4" |

> Direction legend: ▲ = higher is better, ▼ = lower is better, ● = neutral
> (descriptive — read with its companion, never coloured good/bad).

**Rating.** 19 of 29 metrics carry a 5-threshold `rating_scale`. The rating is
*how many thresholds the reading passed*, evaluated direction-aware — an integer
0–5, and the only scale on which different metrics can be compared or combined.
It is `null` for `neutral` metrics, `weight = 0` metrics, and metrics with no
scale. See [Composite health](#composite--overall-health).

**Registry at a glance:** 29 registered · 27 active · 26 with data · 2 retired ·
1 never computed · 7 active families · 17,026 rows across 68 repositories.

---

## Outcome — was the work right?

### First-turn resolution `ftr`
- **Facets:** outcome · pct · ▲ · weight **3** (the heaviest)
- **Definition:** share of sessions completed without a correction turn.
- **Calculation:** `count(sessions where ftr=true) / count(sessions)` over the window.
- **Source:** `activity.sessions.ftr` — live. **180 rows, 17 repos, 2025-05-07 → today.**
- **How to read:** the north-star. Never read alone — a high FTR with rising
  rework means work is being *deferred*, not done. Companion: **rework share**.
  Friction concentrates in big multi-surface repos, so always drill per-module.
- **Representation:** paired stat chip (FTR ‖ rework) + trend line.
- **Status:** live.

### Rework share `rework_ratio`
- **Facets:** outcome · ratio · ▼ · weight 1
- **Definition:** share of all tool-calls that came from corrected sessions.
- **Calculation:** `Σ tool_calls in sessions where outcome='corrected' / Σ tool_calls`.
- **Source:** `activity.sessions.outcome` + `activity.turns.tool_calls` — live. **173 rows, 17 repos.**
- **How to read:** FTR's mandatory companion — raw volume is an *inverted*
  productivity signal. High + rising = effort is going into fixing, not building.
- **Representation:** the ▼ half of the FTR paired chip; trend line beneath FTR.
- **Status:** live.

### Spec depth `spec_depth`
- **Facets:** outcome · score (0–5) · ▲ · weight **2**
- **Definition:** how complete/observable the plan stated in a session's opening
  turns was before implementation began (acceptance criteria, inputs/outputs/deps,
  no TBDs).
- **Calculation:** mean over scored sessions of the 0–5 spec-depth judgment
  (transcript-internal plan), per repository per day. An **LLM judgment**, not a
  deterministic count.
- **Source:** transcript opening turns, `session_process` task — live. **108 rows, 14 repos.**
- **How to read:** only sessions with a plan-like opening are scored; ad-hoc
  sessions are N/A and excluded — read it as "of the sessions that planned, how
  deep". Grounded in quoted plan turns. Read paired with FTR + deviation.
- **Representation:** trend line + the quoted plan turns as evidence.
- **Status:** live.

### Spec deviation `spec_deviation_rate`
- **Facets:** outcome · pct · ▼ · weight **2**
- **Definition:** share of planned sessions whose implementation departed from the
  stated plan (unplanned scope, instead-of-X pivots, dropped plan items).
- **Calculation:** `planned sessions flagged as deviating / planned sessions`, per repository per day.
- **Source:** transcript, `session_process` task — live. **111 rows, 14 repos.**
- **How to read:** denominator is sessions that had a stated plan (others N/A).
  Some deviation is healthy discovery — read the trend paired with FTR + rework,
  and open the evidence pairs (plan quote vs deviating action) before judging.
- **Representation:** trend line; evidence pairs on drill-down.
- **Status:** live.

### Context pressure `context_pressure_rate`
- **Facets:** outcome · pct · ▼ · weight 1
- **Definition:** share of measurable sessions that hit context pressure
  (compaction / low-context or start-fresh advice).
- **Calculation:** `count(sessions with a context-pressure trouble signal) / count(measurable sessions)`.
- **Source:** `activity.assistant_events` trouble signals — live. **180 rows, 17 repos.**
- **How to read:** rising means sessions hit compaction before finishing — lean on
  smaller tasks or checkpoints. Pressure that doesn't hurt FTR is tolerable.
- **Representation:** gauge; trend beneath FTR.
- **Status:** live.

### Run completion `run_completion`
- **Facets:** outcome · ratio · ▲ · weight 1
- **Definition:** autonomous runs reaching `done` ÷ runs started.
- **Calculation:** `count(runs status='done') / count(runs)`.
- **Source:** `activity.runs.status` — live but **low-N: 5 rows, 1 repo.** Sets `props.low_n = true`.
- **How to read:** autonomy health. Gate the display until enough runs exist.
- **Representation:** gauge with a low-N "not enough data" state.
- **Status:** live (low-N gated).

---

## Quality — is the code healthy?

The four git/qlty-sourced metrics here have the deepest and most trustworthy
history in the system: they are measured against a `git worktree` checked out to
each sampled commit, so they backfill over real history rather than snapshotting
today.

### Duplication `duplication_ratio`
- **Facets:** quality · ratio · ▼ · weight **3**
- **Definition:** duplicated source lines ÷ total source lines, from a `qlty` scan at each sampled commit.
- **Calculation:** `Σ duplicated lines (qlty smells) / total source lines (qlty metrics)`
  at a `git worktree` checked out to the commit as-of the sampled day. The
  numerator is the union (deduped per file, so the ratio stays in `[0,1]`) of the
  physical line ranges `qlty smells --sarif` flags as `identical-code`/
  `similar-code`; the denominator is the `qlty metrics --all` TOTAL `lines`.
  Task group `quality`.
- **Source:** `git worktree` + `qlty` — live. **257 rows, 11 repos, 2025-05-13 → 2026-09-02.** Carries `commit_sha`.
- **How to read:** rising duplication = DRY erosion, measured over REAL history.
  Sampled ~weekly (one commit-day per ISO week) to bound scan cost. A non-git
  project, an absent `qlty` CLI, or a commit predating the repo's `.qlty` config
  → no row (honest-empty, never fabricated). Project-level; per-module (folder)
  attribution is a deferred follow-up.
- **Representation:** trend line over sampled commit-days. **Multiple rows per
  day** (one per sampled commit × scope) — pick the latest commit or aggregate, and say which.
- **Status:** live. `qlty` is an OPTIONAL tool — absent → honest-empty.

### Maintainability `module_quality`
- **Facets:** quality · ratio · ▼ · weight **3**
- **Definition:** maintainability-smell burden ÷ total source lines, from a `qlty` scan at each sampled commit.
- **Calculation:** `count(qlty maintainability smells) / total source lines (qlty metrics)`
  at the sampled commit's worktree — file/function-complexity, deep-nesting,
  long-parameter-list findings (`qlty smells --sarif` minus the duplication findings).
- **Source:** `git worktree` + `qlty` — live. **257 rows, 11 repos.** Carries `commit_sha`.
- **How to read:** higher = more smells per line (lower is better). **Coverage is
  deliberately OUT of scope here** — a historical worktree has no lcov artifact,
  so coverage is left honest-empty (we do NOT run tests per-worktree).
  **The name misleads: this is project-level, not per-module and not
  architectural.** Per-module attribution and an A–F grade mapping are deferred.
- **Representation:** trend line over sampled commit-days.
- **Status:** live for maintainability; per-module attribution deferred.

### Coverage `coverage`
- **Facets:** quality · ratio · ▲ · weight **3**
- **Definition:** covered source lines ÷ instrumented source lines, from an lcov
  report the project's own test run produced.
- **Calculation:** `Σ lines hit (LH) / Σ lines found (LF)` from the ingested lcov report.
- **Source:** an lcov report **ingested** from the current checkout — the daemon
  never runs your tests. Live but thin: **60 rows, 3 repos, 2026-08-20 → today.**
- **How to read:** no report → no row, until your test run / CI writes one at a
  known path (or you set `metrics.coverage_lcov`). A **present-tense snapshot**;
  historical coverage is an opt-in backfill (checkout past commits + run the
  coverage command). `scope = repo` — whole-suite, never per-author.
- **Representation:** gauge with a healthy band; trend where history exists.
- **Status:** live (3 repos — the thinnest coverage of any live metric).

### Churn rate `churn_rate`
- **Facets:** quality · count · ▼ · weight 1 · target 3
- **Definition:** distinct source files changed per commit-day.
- **Calculation:** `count(distinct file paths touched by the day's commits)` via
  `git log --no-merges --numstat` per commit-day (committer date, `--date=short`).
- **Source:** `git log` for the project's git-root (`folders.abs_path` via
  `project_root_path`) — live. **6,176 rows, 64 repos, 2016-02-01 → today, 2,484
  distinct days — the deepest series in the system.** Non-git project / no commits
  that day → no row.
- **How to read:** GitClear's churn *definition*, measured first-party from git.
  Pair with duplication. **Legitimate extreme outliers** from bulk commits —
  15,975 files in one day (`spec`, 2026-01-22) against a median of 6 — so a linear
  axis over full history is one spike and a flat line.
- **Representation:** trend line, log scale or winsorised with the raw value in the tooltip.
- **Status:** live. ⚠️ **The registry's `unit`, `formula` and `description` still
  describe the retired `activity.task_executions` indexing feed** — see
  [#153](https://github.com/sensei-hq/sensei/issues/153). Real unit is *files changed per day*.

### Churn concentration `churn_concentration`
- **Facets:** quality · pct · ● · weight **0** (descriptive)
- **Definition:** share of the day's line-churn absorbed by the busiest 20% of files (Pareto).
- **Calculation:** `Σ line-churn(top 20% files) / Σ line-churn(all files)` per
  commit-day, where a file's daily line-churn is `Σ(added + deleted)` across that
  day's commits (`git --numstat`). Top set = the busiest `ceil(20%)` files. A
  commit-day with zero line-churn (only binary/mode changes) has no denominator ⇒
  NO row (never a 0/0).
- **Source:** `git log --numstat` — live. **6,166 rows, 64 repos, back to 2016.**
- **How to read:** high concentration = a few hotspots absorb the change — the
  files to refactor first. **Neutral: do not colour it good/bad.** Actionable as a target list.
- **Representation:** **Pareto bar** + the hotspot file list ("refactor these first").
- **Status:** live, unscored by design.

### Rework density `rework_density`
- **Facets:** quality · ratio · ▼ · weight 1
- **Definition:** files flagged `rework:` ÷ project files.
- **Calculation:** `count(files in detected_patterns rework:) / count(project files)`.
- **Source:** `inference.detected_patterns` — live. **1,361 rows, 66 repos (the
  widest repo reach of any metric), 2026-08-19 → today.**
- **How to read:** where correction-proneness lives; the per-module friction map.
- **Representation:** **module × metric heatmap** — the primary "locate friction" visual.
- **Status:** live.

### Edit before read `incomplete_analysis_rate`
- **Facets:** quality · pct · ▼ · weight 1
- **Definition:** share of code-editing sessions that modified a file before any
  read or search — a proxy for building before understanding.
- **Calculation:** `sessions whose first edit precedes any read/search (or that
  edit with no read at all) / sessions with >=1 edit`, per repository per day.
- **Source:** tool-event ordering in `activity.assistant_events`, `session_outcomes` task — live. **163 rows, 16 repos.**
- **How to read:** a heuristic from event ordering, **not intent** — brand-new
  files or trivial known tweaks inflate it. Read the trend, not a single day.
  High + rising alongside rework suggests diving in before analysis.
- **Representation:** trend line, paired with its LLM companion below.
- **Status:** live.

### Shallow analysis `incomplete_analysis_llm_rate`
- **Facets:** quality · pct · ▼ · weight 1
- **Definition:** share of sessions showing a retraction-of-understanding
  ("I misread", "let me actually check", "re-read") that the deterministic
  edit-before-read signal can't see.
- **Calculation:** `sessions with >=1 understanding-retraction / measurable sessions`, per repository per day.
- **Source:** transcript, `session_process` task (LLM) — live. **124 rows, 14 repos.**
- **How to read:** read **alongside** edit-before-read — the deterministic and
  semantic signals together triangulate "dived in before analysis". Evidence = the
  retraction turn. Heuristic, not intent.
- **Representation:** paired trend with `incomplete_analysis_rate`; the retraction turn as evidence.
- **Status:** live.

### Refuted findings `refuted_finding_rate`
- **Facets:** quality · pct · ▼ · weight 1
- **Definition:** share of sessions where the assistant asserted a finding it
  later reversed itself on (distinct from a user correction).
- **Calculation:** `sessions with >=1 self-refuted finding / measurable sessions`, per repository per day.
- **Source:** transcript, `session_process` task (LLM) — live. **124 rows, 14 repos.**
- **How to read:** **not** the same as user corrections (that's rework/FTR) — this
  is the assistant refuting itself. A little is normal exploration. Evidence pairs
  the assertion turn with the retraction turn.
- **Representation:** trend line; assertion↔retraction evidence pair on drill-down.
- **Status:** live.

---

## Velocity — how much *right* work?

### Throughput `throughput`
- **Facets:** velocity · count · ▲ · weight 1 · target 5
- **Definition:** completed sessions per day.
- **Calculation:** `count(sessions) per day`.
- **Source:** `activity.sessions` — live. **180 rows, 17 repos.**
- **How to read:** only meaningful **paired with FTR** — throughput of corrected
  work is negative value. Never report LOC or edit-count as velocity.
- **Representation:** trend line, always stacked under the FTR trend.
- **Status:** live.

### Time to useful result `time_to_useful_result`
- **Facets:** velocity · duration (seconds) · ▼ · weight 1
- **Definition:** median seconds from session start to the first non-correction
  turn — how fast a session produced its first usable output.
- **Calculation:** `median over sessions of ((first turn where is_correction=false).ended_at - session.started_at)`, per day.
- **Source:** `activity.turns` — live. **173 rows, 17 repos.**
- **How to read:** read paired with FTR — fast-but-wrong is not value. A rising
  median means the first usable output is taking longer. Idle time between turns
  inflates it.
- **Representation:** trend line (duration axis).
- **Status:** live, but ⚠️ **its rating is meaningless — the `rating_scale`
  `[60,30,15,8,4]` is in minutes while the value is in seconds, so 87.3% of
  readings score 0.** Do not present as a graded signal until
  [#154](https://github.com/sensei-hq/sensei/issues/154) lands.

### Session duration `session_duration`
- **Facets:** velocity · duration (seconds) · ● · weight **0** (descriptive)
- **Definition:** average active working time per session (gap-aware, excludes long idle gaps).
- **Calculation:** `avg(active duration seconds) over sessions with a recorded duration`, per repository per day.
- **Source:** `activity.sessions` — live. **181 rows, 18 repos (the widest session-family reach).**
- **How to read:** **descriptive, not good/bad** — a long session that delivered a
  big feature is fine. Read paired with throughput + tokens. Active work time, not wall-clock.
- **Representation:** trend line; context beneath throughput. Never coloured good/bad.
- **Status:** live, unscored by design.

---

## Usage — token and context volume

These replaced the catalog's former speculative cost family. Under a flat
subscription the marginal cost of a token is zero, so these measure **context
consumed**, not money — see `cost_per_result` for the money question.

### Tokens per result `tokens_per_result`
- **Facets:** usage · ratio · ▼ · weight 1
- **Definition:** output tokens generated per completed session — the token cost of a delivered result.
- **Calculation:** `sum(tokens_out over completed sessions) / count(completed sessions)`, per repository per day.
- **Source:** transcript `usage` → `sessions.tokens_out` — live. **112 rows, 16 repos.**
- **How to read:** read paired with throughput + FTR — cheap-but-wrong is not
  value, and a big feature legitimately costs more tokens. Output-based, so not
  inflated by input cache re-reads.
- **Representation:** trend line; stat chip beside throughput.
- **Status:** live.

### Context reuse `cache_reuse`
- **Facets:** usage · pct · ▲ · weight 1 · no rating scale
- **Definition:** mean share of a session's input tokens served from cache rather than re-sent fresh.
- **Calculation:** `mean over sessions of (cache_read / (fresh_input + cache_write + cache_read))`, per repository per day.
- **Source:** transcript `usage` — live. **73 rows, 10 repos, 2026-07-17 → today.**
- **How to read:** averaged **per SESSION, not per request** — per request it is a
  flat 99.8% because the prefix is cached on nearly every call. The spread comes
  from session **length**: long sessions sit at 96–99%, 2–3-turn sessions fall to
  ~83%, because a short session never amortises its cold start. Low = many short
  sessions re-paying for context. Read with session duration.
- **Representation:** gauge with a healthy band (narrow real range: 0.66–0.99).
- **Status:** live, unscored (no rating scale).

### Tokens per day `tokens_per_day` · Input `tokens_in_per_day` · Output `tokens_out_per_day`
- **Facets:** usage · count · ● · weight **0** (descriptive)
- **Definition:** total / input / output model tokens consumed per day.
- **Calculation:** `sum(tokens_in + tokens_out)`, `sum(tokens_in)`, `sum(tokens_out)`
  over sessions with token usage, per repository per day.
- **Source:** transcript `usage` — live. **129 rows each, 16 repos.**
- **How to read:** a volume signal, **not a quality one**, and `neutral` — up is
  not bad. Input is dominated by cache re-reads that grow with session length, so
  a spike usually means longer sessions, not waste. **Output is the cleaner "work
  produced" proxy.** Read the trend, not a single day.
- **Representation:** stacked area (in vs out) or dual trend. Never coloured good/bad.
  **Values reach 2.34 × 10⁹ — six orders of magnitude above the ratio metrics, so
  never share a linear axis with them.**
- **Status:** live, unscored by design.

---

## Cost — what did it cost?

### Cost per result `cost_per_result`
- **Facets:** cost · currency · ▼ · weight 1 · no rating scale
- **Definition:** subscription fee for the trailing 30 days divided by what
  shipped in it (merged runs + accepted recommendations).
- **Calculation:** `subscription fee over trailing 30 days / (merged runs + accepted recommendations)` in the same window.
- **Source:** the configured plan + `runs` + accepted `recommendations` — live but
  thin: **28 rows, 2 repos, 2026-08-23 → today.** `props` carries `plan`,
  `currency`, `window_days`, `merged_runs`, `accepted_recommendations`.
- **How to read:** the only money question a flat subscription can answer. Read
  with throughput — it falls when you ship more for the same fee, so a drop is
  genuine efficiency. **It is NOT a token metric.** Blank until you configure your
  plan. **Fails closed** on a missing plan (money-facing — never a defaulted rate).
- **Representation:** money strip with the plan named; comparative bar across projects.
- **Status:** live (2 repos — needs a configured plan).

---

## Autonomy — how much babysitting?

### Interruptions `interruption_rate`
- **Facets:** autonomy · ratio · ▼ · weight 1
- **Definition:** Stop events ÷ UserPromptSubmit events.
- **Calculation:** `count(Stop) / count(UserPromptSubmit)` from the event stream.
- **Source:** `activity.assistant_events.event_type` — live. **300 rows, 17 repos —
  the densest session-family metric.** Sets `props.low_n` on thin days.
- **How to read:** high = the human keeps stepping in — the babysitting signal.
  **Not bounded by 1** — real readings reach 5.0.
- **Representation:** gauge (domain to the real max, not `[0,1]`); trend.
- **Status:** live.

### False crashes `false_crash_rate`
- **Facets:** autonomy · ratio · ▼ · weight **0**
- **Definition:** runs killed at the recovery cap that were actually just waiting.
- **Calculation:** `count(runs killed-at-cap but waiting) / count(non-done runs)`.
- **Source:** `runs.recovery_attempts`, `run_events.detail` — **never computed: 0 rows.**
- **How to read:** n/a until it produces a row.
- **Representation:** **empty state, not a chart.**
- **Status:** never computed.

---

## Knowledge — is it learning?

### Memory promotion `memory_promotion`
- **Facets:** knowledge · ratio · ▲ · weight 1
- **Definition:** memories created ÷ eligible patterns/corrections (`instance_count ≥ 3`).
- **Calculation:** `count(memories created) / count(eligible patterns+corrections)`.
- **Source:** `memories`, `detected_patterns`, `corrections` — live. **141 rows, 7
  repos.** `props` carries `eligible_patterns` + `eligible_corrections`.
- **How to read:** **≈0 is the signal** — it proves sensei's own distill loop is
  silent. This is the metric that shows whether the **measure→distill→govern loop**
  (see `feature.md`) is closing. Not bounded by 1 — readings reach 4.0.
- **Representation:** trend + a "loop health" callout when it flatlines at 0.
- **Status:** live (measures a stalled pipeline — the point).

---

## Composite — overall health

### Project health — **live, as a VIEW over ratings**

The stored `project_health` **metric** is retired (`effective_until:
2026-08-12`) — a plain mean of normalized components moved 46→44 purely because
`throughput` dipped on a quiet day. It produced zero rows and is not scheduled.

**The capability was not retired — it moved.** Project health is now derived,
not stored:

| view | what it gives |
|---|---|
| `sensei.metric_ratings` | latest daily reading per (project, metric) with its 0–5 `rating` |
| `sensei.project_health_score` | `round(20 × Σ(weight × rating) / Σ(weight))` → 0–100, plus `rated_metrics`, `total_weight`, and a `components` jsonb (per-metric `rating`/`weight`/`name`) |
| `sensei.project_health_trend` | the same score per `grain` × `period` (daily/weekly/monthly), plus **`coverage`** — the share of total model weight that backed the score |

Served by `GET /api/projects/{id}/health`. Live: **67 projects scored**, trend
has 1,806 daily / 449 weekly / 228 monthly rows back to 2018.

- **How to read:** the "is this project healthy?" glance; drill into `components`
  to see what's dragging it. Not a substitute for FTR — a summary of it and its
  companions. **Always read with `coverage`/`rated_metrics`** — see the defect below.
- **Representation:** **health dial** + component breakdown (the radar plots
  `rating`, never raw values — raw scales are unplottable together) + trend line.
  The daemon owns the rating so a spoke and the score cannot disagree; never
  re-derive a rating client-side.
- **Status:** live. ⚠️ **The score is not gated on coverage, and it inverts:**
  projects with 1 rated metric average **98**, projects with 18 average **58**.
  50 of 67 scored projects rest on 1–2 metrics. A repo sensei knows almost nothing
  about looks perfect. See [#155](https://github.com/sensei-hq/sensei/issues/155).

---

## Retired

Both remain in `sensei.metrics` with an `effective_until` in the past, so they are
not scheduled and write no rows. Kept registered — and documented here — so a
reader who encounters an old row or an old chart can find out what it was.

### Tools used `unused_tools` — retired 2026-08-20
- **Was:** `tool · count · ▼ · weight 1 · target 10` — registered tools with 0
  successful outcomes in a 14-day window (`count(tools with 0 outcome-positive calls in 14d)`).
- **Source:** `assistant_tools` + `tool_call_verdicts`.
- **Why retired:** the verdict it rested on was fragment-overlap, not outcome, so
  "unused" did not mean unused. **Produced 0 rows.** The knowledge-family successor
  work (outcome-based verdicts) is unregistered.
- **Reviving it** means clearing `effective_until` in the seed — but not before the
  verdict is outcome-based, or it measures the same nothing.

### Project health `project_health` — retired 2026-08-12
- **Was:** `composite · score · ▲ · weight 1` — a single 0–100 roll-up.
- **Why retired:** a plain mean of ~9 normalized components is not a meaningful
  signal — it moved 46→44 purely because `throughput` dipped on a quiet day, while
  FTR and rework were perfect. `retire_reason`: *composite mean is not a meaningful
  signal; FTR is the north star, and a qlty-based code-quality family replaces the
  code-health role.* **Produced 0 rows.**
- **Not a loss of capability** — the composite moved to the
  [`project_health_score` view](#composite--overall-health), which is live and
  scores 67 projects. The schema + handler are kept and it is revivable, but there
  is no reason to: the view is strictly better, since it composes ratings rather
  than raw normalized values.

---

## Coverage summary

| Family | Live | Thin / gated | Absent |
|---|---|---|---|
| Outcome | ftr, rework_ratio, spec_depth, spec_deviation_rate, context_pressure_rate | run_completion (low-N, 1 repo) | |
| Quality | duplication_ratio, module_quality, churn_rate, churn_concentration, rework_density, incomplete_analysis_rate, incomplete_analysis_llm_rate, refuted_finding_rate | coverage (3 repos — needs an lcov report) | per-module attribution for duplication/maintainability |
| Velocity | throughput, session_duration | time_to_useful_result (rating broken, [#154](https://github.com/sensei-hq/sensei/issues/154)) | |
| Usage | tokens_per_day, tokens_in_per_day, tokens_out_per_day, tokens_per_result, cache_reuse | | |
| Cost | | cost_per_result (2 repos — needs a configured plan) | |
| Autonomy | interruption_rate | | false_crash_rate (never computed) |
| Knowledge | memory_promotion (≈0 = the signal) | | recall-hit, repeat-mistake, guidance-adherence — all unregistered |
| Composite | project health (via `project_health_score` view) | | coverage gating ([#155](https://github.com/sensei-hq/sensei/issues/155)) |
| **Architecture** | — | — | **the whole family — see below** |

### The architecture gap

**No registered metric reads the code graph.** `sensei.edges` holds 811,071
edges, `inference.communities` 32,495 clusters, and nothing consumes them. There
is no coupling, cohesion, fan-in/fan-out, dependency-cycle, layering-violation or
instability metric. `module_quality` sounds architectural but is a project-level
qlty smell density.

Three signals are computable **today** (public-surface ratio, symbol-size
distribution, graph resolution confidence); the module-level ones are **blocked on
edge resolution**, not on metric design — sensei's own repo yields only 13 distinct
module→module dependency edges because Rust call resolution is broken
([#146](https://github.com/sensei-hq/sensei/issues/146),
[#151](https://github.com/sensei-hq/sensei/issues/151),
[#152](https://github.com/sensei-hq/sensei/issues/152)).

The full proposal, with prototype queries and measured feasibility per metric,
is in [`architecture-metrics.md`](./architecture-metrics.md) and tracked as
[#156](https://github.com/sensei-hq/sensei/issues/156).

## Related

- [[features/metrics/feature]] (why + philosophy) · [[features/metrics/architecture-metrics]] (the proposed architecture family)
- [`docs/design/metrics-chart-brief/`](../../design/metrics-chart-brief/) — machine-readable registry + real sample data for chart work
- [[spec/pipeline/metrics]] (design) · [[spec/pipeline/ftr]]
