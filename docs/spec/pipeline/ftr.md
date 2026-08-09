# 率 · Pipeline · FTR (first-turn resolution)

**Source of truth:** per-session boolean `activity.sessions.ftr`; the daily
FTR **aggregation** source of truth is `sensei.project_metrics` (metric='ftr')
**Roll-ups:** `sensei.project_metrics` daily `ftr` rows, read via
`sensei.project_metric_daily` and the weekly/monthly/quarterly roll-up views.
_(The legacy `sensei.ftr_daily` and `sensei.project_ftr_metrics` views were
retired in Phase 8 — FTR is now sourced entirely from `project_metrics`.)_
**Read endpoint:** `GET /api/projects/{id}/ftr` (per-project) · `GET /api/observatory/ftr` (across all active projects)
**Owner file:** `crates/senseid/src/tasks/handlers/analyze.rs` (per-session boolean derivation)

## Purpose

FTR — first-turn resolution — is the **north-star metric** for the
whole product. It answers one question: when the assistant made its
first attempt on a task, did it land? A session is FTR-true when the
developer's follow-up prompts contain no correction signal (no
"actually", no "revert", no "no, X"). A session is FTR-false when
at least one correction fires.

Every visible number in the app is judged by whether making it move
would make FTR go up, or expose the reason it went down. If a UI
change can't be justified by "this raises FTR or explains why it
fell", it doesn't ship.

Kanji is 率 — *rate*.

## Data invariants

- `activity.sessions` has one row per assistant session with
  `project_id`, `started_at`, `corrections` (int), and `ftr`
  (`bool` — derived as `corrections == 0` at enrichment time).
- Enrichment happens in
  `tasks::handlers::analyze::enrich_session` — it walks the
  assistant events for the session, runs `correction_signal(prompt)`
  on each user prompt, counts hits, and writes `ftr = corrections == 0`.
- `correction_signal()` is **precision-favouring** — a false positive
  wrongly tanks FTR, so only unambiguous phrases trip it (unit-tested
  list in `analyze.rs` `correction_prompt_drops_ftr_and_marks_corrected`).
- The daily `ftr` rows in `sensei.project_metrics` (project scope,
  `grain='daily'`, `folder_id` null) are the aggregation source of truth,
  written by the `session_outcomes` computer over the measurable base
  (`outcome is not null`): `value` = num/den, `props.numerator` = first-try
  sessions, `props.denominator` = session_count, `props.correction_count` =
  Σ corrections.
- `get_project_ftr` re-derives its headline from those rows (read via
  `sensei.project_metric_daily`, metric='ftr'):
  - `sessions_7d` — Σ `props.denominator` over the last 7 days
  - `ftr_14d` — Σ `props.numerator` / Σ `props.denominator` over the last
    14 days (session-weighted — the sum of the parts, never a mean of daily
    ratios)
  - `ftr_14d_prev` — same Σnum/Σden over the 14–28d window (comparison anchor)
  Because the store scopes to analyzed sessions (`outcome is not null`), the
  number can differ from the retired view (which scored in-flight sessions as
  0); the analyzed-base number is the intended consolidated FTR.
- The 14-day daily trend for the sparkline is still computed inline in
  `get_project_ftr` from `activity.sessions` — `date_trunc('day', started_at)`
  grouped, then `AVG(CASE WHEN ftr THEN 1.0 ELSE 0.0 END)` per bucket.
- **Ownership** of `sessions.ftr` and `sessions.corrections` sits with
  the enrichment task, not the writer. Raw session inserts leave
  those columns null; the analyzer fills them.

## Signals produced

| Signal | Source | Shape | Consumed by |
|---|---|---|---|
| `ftr14d` | `project_metrics` (metric='ftr') | float 0.0–1.0 | Today FTR chip, Projects card, Project overview, Impact before/after |
| `ftr14dPrev` | `project_metrics` (metric='ftr') | float 0.0–1.0 | Today FTR arrow (up/down/flat vs prior window) |
| `ftrTrend` | inline query (`activity.sessions`) | array of 14 floats | Today sparkline |
| `sessions7d` | `project_metrics` (metric='ftr') | integer | Today session count line, Projects card `7d` stat |
| `sessions.ftr` | table | bool per session | Sessions list FTR badge, Replay session-level badge |
| `sessions.corrections` | table | int per session | Sessions list "N corrections" chip, per-session detail pane |
| `correction_signal(prompt)` | function | Option<&str> ("correction"/"revert"/…) | tags the specific corrective turn in the Replay timeline |

## Done gate

- On Jerry's live data, `GET /api/projects/sensei/ftr` returns
  `ftr14d` in `[0.0, 1.0]` with `sessions7d ≥ 0` — real number,
  matches what the sessions view shows.
- The 14-day trend array has one entry per day the project had ≥ 1
  session in the window, in ascending date order.
- `ftr14dPrev` is computed from the 14–28d window, not from the 0–14d
  window minus one day. This is what makes the arrow "vs prior
  window" instead of "vs yesterday".
- `activity.sessions` never carries `ftr = null` on a session with
  `analyzed_at IS NOT NULL` — analyzer either writes both `ftr` and
  `corrections` or writes neither.
- The correction-signal list is unit-tested — every phrase we
  believe corrects must be in `correction_prompt_drops_ftr_and_marks_corrected`
  or the analogous positive test.
- The Today screen's FTR chip and the Projects card FTR stat show
  the **same number** for the same project on the same day
  (both re-derive from the same `project_metrics` `ftr` rows — but drift
  here has been a real bug).

Optional check:
```
curl -s http://localhost:7744/api/projects/sensei/ftr | jq
# expected: { ftr14d, ftr14dPrev, ftrTrend: [...], sessions7d }
# expected: ftr14d in [0,1]; ftrTrend length == number of distinct days with sessions in last 14d
```

## Wrong gate

- **`ftr14d` is 0 despite FTR-true sessions in the last 14 days.** Either
  the `session_outcomes` computer never wrote the daily `ftr` rows, or the
  14d window filter (`d.date > current_date - 14`) didn't fire — likely a
  date-type mismatch or missing `props.numerator/denominator`.
- **Trend array has 14 entries with all zeros while `ftr14d > 0`.**
  The daily rollup is casting the boolean wrong; `CASE WHEN ftr THEN
  1.0 ELSE 0.0 END` must accept the actual boolean column type.
- **Sessions list shows `FTR ✓` next to a session whose Replay
  panel says "no tool calls".** This is the session-id resolution
  bug — Replay is looking up `activity.assistant_events` by
  `sessions.id` but the events are keyed on `client_session_id`.
  Fixed once already; the fix must survive.
- **`corrections` counted but the corresponding turn is not tagged
  as `is_correction` in the enrichment output.** Two derivations
  diverged.
- **`ftr14dPrev` mirrors `ftr14d`.** Windows collapsed to the same
  filter.
- **A prompt like "actually let's keep it" is counted as a
  correction.** Precision failure — add to the false-positive
  regression list. `correction_signal` is precision-favouring.
- **New assistant that doesn't emit UserPromptSubmit events shows
  FTR = 1.0 always.** The signal source is a specific event kind;
  without it we can't distinguish "no correction" from "no signal".
  Should surface as `ftr = null` with a UI note, not `ftr = true`.

## Related

- [[pipeline/analyzer]] — where `enrich_session` runs
- [[pipeline/capture]] — where the raw assistant events come from
- [[pipeline/insights]] — surfaces FTR changes as Today insights
- [[pipeline/impact]] — before/after FTR when a memory is adopted
- [[screen/observatory-today]] — primary consumer
- [[screen/observatory-projects]] — per-project FTR stat
- [[screen/project-impact]] — scoped before/after FTR

## Locked decisions (2026-07-07)

- **Per-model FTR — yes, calculate it.** Not as the north-star
  (that stays per-project so the user sees whether the pair is
  getting better here). Per-model FTR is a **decision aid**: which
  model works best for this project / this stack. Surfaces on:
  - Project Overview (secondary chip: "Opus 4.6 is scoring 0.82
    here — try it")
  - Model effectiveness pane
  - Autonomous model preference in the analyzer's
    `model_insight` step (already shipped — see
    (memory: project_standalone_completion_plan))
  Computed from `sessions.provider` + `sessions.model` +
  `sessions.ftr`, same view logic scoped by (provider, model).

- **Low-turn sessions.** Keep the 1-turn minimum for the FTR
  numerator. Surface a `low_signal` chip on projects where
  median turns < 3 so the reader sees the number is thin.
- **Subagent side-chains.** Subagent corrections do **not** count
  toward the main session's FTR — they're not the user's
  correction, so bundling them tanks the parent unfairly. But the
  *cause* of a subagent deviation IS worth learning from: what
  prompt / tool response / context made the subagent course-
  correct? Those events feed the memory pipeline as candidates
  the same way top-level corrections do — see
  [[pipeline/memory]] formation. Storage: add
  `sessions.parent_session_id` (nullable) so subagent sessions
  are queryable but keep their corrections out of the parent's
  `corrections` counter and FTR calc.
