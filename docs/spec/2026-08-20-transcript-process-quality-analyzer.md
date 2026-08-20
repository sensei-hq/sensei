# Transcript process-quality analyzer (LLM-derived session judgments)

Status: SPEC. Follows [session token+model capture + token metrics](./2026-08-18-repo-grain-metrics-watermark-engine.md)
and [metric rating scales + health](./2026-08-20-metric-rating-scales-health.md).
This is "Phase 2" of the session-metrics work: the first LLM-derived judgment layer.

## 1. Problem

The shipped session metrics (FTR, tokens, duration, edit-before-read) are all
deterministic SQL. They answer "how much / how fast / how often", but not the
process-quality questions Jerry raised: **was the plan deep before building? did
the work deviate from it? was analysis incomplete? did the assistant assert a
finding it later refuted?** These need semantic judgment over the transcript, not
a count — but they must stay honest (evidence-backed, N/A when unjudgeable) and
cheap enough to run on the local model.

## 2. Objective

A daily, background, **local-LLM** pass that reads each session's persisted
transcript turns and emits, per session:
- four process-quality judgments (below), each with a **0–5 score + the quoting
  `transcript_turns` it is grounded in** ("referenceable transcript statements"),
- written to `activity.sessions.props.process` + an evidence table,
which a `session_outcomes`-style aggregator rolls into **day-grain metrics** that
chart alongside FTR/tokens and (for the scored ones) feed the 0–100 health score.

No fabrication: a session with nothing to judge for a signal gets that signal
`null` (N/A), never a defaulted 0/5. The pass never blocks or slows capture.

## 3. Decisions (locked)

- **D1 — Local, background, daily.** Runs on the embedded gateway (reasoning
  chain, gemma4; `cloud` only if the user routes it there), as a new
  `TaskKind::AnalyzeSessionProcess`, enqueued on the analyzer scheduler's existing
  **daily full-refresh window** (`DEFAULT_FULL_REFRESH_SECS = 86_400`). It is NOT
  part of transcript ingest — ingest stays deterministic + model-free; this is a
  downstream consumer of the settled corpus.
- **D2 — Spec anchor = transcript-internal.** The "spec" a session is judged
  against is the plan/intent stated in its OWN early turns (the LLM extracts the
  stated plan, then judges depth + deviation from within). No dependency on an
  external `docs/spec`/`plan_graph`. A session with no plan-like opening scores
  spec_depth / spec_deviation as **N/A**.
- **D3 — Four signals in v1:** `spec_depth`, `spec_deviation`, `refuted_findings`,
  `incomplete_analysis_llm`. The last two work on any session; the first two only
  when D2 finds a stated plan.
- **D4 — Coverage: incremental, all measurable.** Score every measurable session
  once (watermark-gated on a new `sessions.process_analyzed_at`), one-time
  historical backfill on first run, then only new/changed sessions each daily tick.
  Idempotent re-runs overwrite in place.
- **D5 — Evidence is mandatory.** Every non-null judgment carries ≥1 evidence row
  referencing the exact `transcript_turns(session_id, turn_index)` it rests on.
  A judgment the model can't ground in a quote is dropped, not stored.
- **D6 — Reuse, don't rebuild.** Aggregation into day-grain metrics reuses the
  `session_outcomes` computer pattern + the generic `metric_ratings`/health views
  (no view changes — a rated metric with a `rating_scale` flows automatically).
  The deterministic `incomplete_analysis_rate` (already shipped) stays; the LLM
  `incomplete_analysis_llm` is a distinct, richer companion, not a replacement.
- **D7 — Fail-open + honest-empty.** A model error / timeout / unparseable
  response on a session leaves it unscored (watermark NOT advanced, retried next
  tick) — never a fabricated judgment. A daily tick that scores nothing is a valid
  no-op.

## 4. The four signals

Each judgment is `{score: 0..5 | null, evidence: [{turn_index, quote}], note}`.
Direction is stated for the derived metric.

- **spec_depth** (higher_better). How complete/observable the stated plan was
  before implementation began: are there acceptance criteria, named
  inputs/outputs/deps, and an absence of unresolved TBDs? 5 = a plan you could
  hand to an autonomous run; 1 = "fix the thing". N/A when no plan-like opening.
- **spec_deviation** (lower_better, expressed as a rate metric). Did the
  implementation depart from the stated plan — unplanned scope, "instead of X,
  doing Y" pivots, silently dropped plan items? Evidence pairs the plan quote with
  the deviating action turn. N/A when no stated plan.
- **refuted_findings** (lower_better). Count/'≥1' of the assistant's OWN
  assertions later reversed by the assistant ("the bug is X" … "actually X was
  fine, it's Z"). Distinct from a user correction (already `corrections`).
  Evidence pairs the assertion turn with the retraction turn.
- **incomplete_analysis_llm** (lower_better). Retraction-of-understanding signals
  the deterministic edit-before-read can't see: "I misread", "let me actually
  check", "I need to re-read". Evidence = the retraction turn. Companion to the
  shipped deterministic `incomplete_analysis_rate`.

## 5. Persistence

- `activity.sessions.props.process` (jsonb) — the per-session judgment object
  `{spec_depth, spec_deviation, refuted_findings, incomplete_analysis_llm}`, each
  the `{score, evidence, note}` shape above. Null-valued signals omitted.
- `activity.session_process_evidence` (new table) — one row per evidence quote:
  `(session_id, signal text, turn_index int, quote text, kind text, created_at)`.
  FK-free reference to `transcript_turns(session_id, turn_index)` (turns may be
  pruned; evidence keeps the quote verbatim so the drill-down survives). Powers the
  "referenceable transcript statements" UI.
- `activity.sessions.process_analyzed_at timestamptz` (additive col) — the D4
  watermark: NULL ⇒ never scored ⇒ eligible; set to `now()` after a successful
  pass. A later transcript re-ingest that bumps the session clears it (re-score).

## 6. Derived day-grain metrics (registry + aggregator)

New `session_outcomes`-family metrics (task computes from `sessions.props.process`,
per repository per day, pooled by `project_metric_daily`, honest-empty):

| key | type | family | direction | weight | scored? |
|-----|------|--------|-----------|--------|---------|
| `spec_depth` | score (0–5) | outcome | higher_better | 2 | yes |
| `spec_deviation_rate` | pct | outcome | lower_better | 2 | yes |
| `refuted_finding_rate` | pct | quality | lower_better | 1 | yes |
| `incomplete_analysis_llm_rate` | pct | quality | lower_better | 1 | yes |

- `spec_depth` day value = mean of scored sessions' 0–5 (only sessions with a
  stated plan contribute; the rest are N/A, excluded from the denominator).
- the `_rate` metrics = flagged sessions / measurable sessions that day.
- Scales seeded in `metrics.jsonl`; rated automatically by the generic
  `metric_ratings` view → feed the health score by weight (D6).

## 7. Task + scheduling

- `TaskKind::AnalyzeSessionProcess` — `path` carries the project id (mirrors
  `AnalyzeProject`). Per project: select measurable sessions with
  `process_analyzed_at IS NULL` (or transcript newer than it), cap batch per tick
  (config `process.batch_per_tick`, default e.g. 25) so one project can't dominate
  the queue; for each, load turns, one gateway reasoning call → parse → validate
  evidence grounding (D5) → write props + evidence + watermark.
- Enqueued by `analyzer_scheduler` on the daily full-refresh window, AFTER the
  project's `AnalyzeProject` (so signals/outcomes exist). On-demand endpoint
  `POST /api/projects/{id}/process/analyze` mirrors it (same task).
- A second global pass (or fold into the existing metrics scheduler) recomputes
  the four day-grain metrics from the now-updated `props.process`.

## 8. Prompt + parsing

- One structured prompt per session: system role = the rubric (the §4
  definitions + the transcript-internal-anchor instruction + "quote the turn_index
  you rely on; if you cannot quote it, return null for that signal"); user content
  = the compacted turn list (`turn_index`, role, text; tool calls summarized).
- Response = strict JSON matching the §5 object. Reuse `consolidate.rs`'s
  JSON-in-prose extraction. A parse failure → D7 fail-open (unscored, retried).
- Token budget from the reasoning chain; long transcripts truncated to a head+tail
  window with a note that the middle was elided (never silently).

## 9. Phases

- **P-A** schema: `session_process_evidence` table + `sessions.process_analyzed_at`
  + `props.process` convention; registry metrics + scales seeded (`dbd import`).
- **P-B** task: `AnalyzeSessionProcess` handler (prompt, gateway call, parse,
  evidence-grounding validation, watermark) + scheduler wiring + on-demand endpoint.
  Pure parse/validate logic unit-tested with fixture responses (no live model).
- **P-C** aggregator: extend/mirror `session_outcomes` to compute the four
  day-grain metrics from `props.process`; confirm they flow through
  `metric_ratings`/health with no view change.
- **P-D** surfaces: the metrics auto-chart (dynamic enumeration); session
  drill-down shows the process judgments + their quoted evidence turns
  ("referenceable transcript statements"). Radar/health picks up the scored ones.
- **P-E** deploy: build + install; first-run historical backfill; verify live.

## 10. Invariants

- Never fabricate: no judgment without an evidence quote (D5); N/A over a defaulted
  score; model error ⇒ unscored + retried, never a stored guess (D7).
- Ingest stays model-free; this is downstream + watermark-gated (D1, D4).
- Local-first: embedded gemma4 via the reasoning chain; cloud only if routed.
- Idempotent: re-scoring a session overwrites props + evidence in place.
- Cost-bounded: batch cap per tick; incremental after the one-time backfill.

## 11. Tests

- Parse/validate: fixture model responses → correct props object; a response
  quoting a non-existent turn_index has that signal dropped (D5); an unparseable
  response yields unscored (D7).
- Anchor: a transcript with a plan-like opening scores spec_depth; one without
  returns N/A (not 0).
- Aggregator: seeded `props.process` over sessions/day → correct day-grain metric
  values; a day with only N/A sessions writes no `spec_depth` row (honest-empty).
- Watermark: a scored session is skipped next tick; a re-ingest that bumps the
  session clears the watermark and re-scores.
- Ratings: the four scored metrics appear in `metric_ratings` with a rating and
  weight, and move the health score, with no view change.
