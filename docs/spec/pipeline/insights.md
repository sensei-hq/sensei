# 今 · Pipeline · Insights (recommendations)

**Owner files:**
- Generation: `crates/senseid/src/tasks/handlers/generate.rs` — produces recommendation candidates
- Ranking: `crates/senseid/src/tasks/handlers/rank.rs` (or the `rank` step in `analyze_project`)
- Triage decisions: `crates/senseid/src/api/handlers/insights.rs`
- Impact follow-up: `crates/senseid/src/tasks/handlers/measure_verdicts.rs` (see [[pipeline/impact]])

## Purpose

Insights turns *"the pipeline saw a pattern of corrections"* into
*"here's the one thing you can do about it right now."* Every
recommendation is a bundle:

- **Title** — the thing to do, model-generated.
- **Why** — the causal chain, model-generated with evidence.
- **Impact** — projected FTR movement, computed.
- **Action verb** — one of the universal three: **Apply · Review ·
  Dismiss**, with one highlighted as recommended.
- **Scope** — where it applies (project / user / org / stack).
- **Evidence** — session ids and corrections that produced it.

The user's decision on each recommendation writes back into the
pipeline: `Apply` schedules a [[pipeline/impact]] measurement to
verify FTR moved; `Dismiss` records the "no" so we don't
re-propose the same thing; `Review` parks it in the Soon column.

Kanji is 今 — *now*.

## Data invariants

- `inference.recommendations` — one row per candidate:
  - `id` uuid
  - `title`, `why`, `impact_projection`, `action_hint` text
  - `impact_level` enum `high | medium | low`
  - `scope` jsonb (`{ project_id?, user_id?, org_id?, stack? }`)
  - `evidence` jsonb (`{ sessions: [...], corrections: [...] }`)
  - `state` enum `proposed | reviewed | applied | dismissed | measured`
  - `default_acp` text (the assistant family / adapter the
    Apply verb sends this to)
  - `strength` numeric 0..1 (ranking signal)
  - `created_at`, `state_changed_at` timestamptz
  - `signature` text (dedup key — same shape from the same
    project doesn't re-fire until state != dismissed AND
    materially different evidence)
- Bucketing at read time:
  - **Now** = `state = proposed AND impact_level = high`
  - **Soon** = `state IN (proposed, reviewed) AND impact_level = medium`
  - **Settled** — recommendations don't live in Settled;
    battle-tested memories do (see [[screen/observatory-insights]]).
- Every recommendation's `title` + `why` + `impact_projection`
  goes through [[pipeline/insight-copy]] with kinds
  `rec_title` / `rec_why` / `rec_impact`. Static fallback strings.

## Generation

`generate.rs` runs per project on the analyzer tick (see
[[pipeline/analyzer]]). Two related lists: **sources** are the
signals that seed a candidate; **types** are the shape of the
resulting `inference.recommendations.type` value. The mapping:

| Source (this doc) | Produces type (see [[pipeline/analyzer]]) |
|---|---|
| Correction clusters | `create_persona` OR `extract_helper` (which depends on cluster shape) |
| Pattern effectiveness deltas | `promote_pattern` |
| Persona / skill gaps | `create_persona` OR `enable_skill` |
| Library-tier detections | `wrap_library` OR `enable_skill` |
| Drift blockers | `fix_drift` |
| (auto) Memory age without evidence | `audit_stale_memory` |

Sources of candidates, ordered:

1. **Correction clusters** — the dominant path. Signatures with
   count >= threshold across >= 2 sessions become a recommendation
   ("N sessions corrected this same shape — consider adopting
   pattern X"). Same signal that feeds the memory pipeline; the
   difference is memories are the *statement*, recommendations are
   the *action*.
2. **Pattern effectiveness deltas** — when a promoted pattern's
   observed FTR delta crosses `PATTERN_EFFECTIVE_MIN`, the
   pipeline recommends promoting it further (project → user, etc.).
3. **Persona / skill gaps** — when a project has recurring
   correction signature X and no persona/skill mentions X, a
   recommendation surfaces "add persona Y".
4. **Library-tier detections** — when repeated calls target a
   library that has no wrapper, recommend wrapping it (feeds
   [[pipeline/libraries]]).
5. **Drift blockers** — when doc-drift crosses a threshold on a
   file referenced by many sessions, recommend fixing.

Each candidate is scored by:

- `impact_projection` — expected FTR movement, computed from
  historical pattern effectiveness at this scope.
- `evidence.sessions` count — the more sessions supporting the
  cluster, the stronger.
- `recency` — recent evidence outweighs stale.
- `deduplication` — signatures already applied or dismissed are
  suppressed until materially different evidence arrives.

## Actions

`POST /api/insights/recommendations/{id}/apply`

- `state → applied`
- Emits a "send to {default_acp}" instruction (see the mockup —
  the mechanism varies by assistant family; for Claude Code, it's
  a plugin skill invocation).
- **Schedules a `MeasureVerdicts` follow-up** at
  `apply_time + measurement_window` (default 7 days) so the
  before/after FTR is measured. See [[pipeline/impact]].
- Records the applied recommendation in `sensei.applied_recommendations`
  for the Impact screen.

`POST /api/insights/recommendations/{id}/review`

- `state → reviewed`
- Parks the recommendation in the Soon column. No impact
  measurement scheduled.
- Reviewed recommendations decay to `state = dismissed`
  automatically after `REVIEW_DECAY_DAYS` (default 14) unless
  reinforced by new evidence.

`POST /api/insights/recommendations/{id}/dismiss`

- `state → dismissed`
- Records dismissal reason (optional text).
- The signature is suppressed until materially different evidence
  fires (new correction cluster in a different module, etc.).

## Signals produced

| Signal | Consumer |
|---|---|
| Now-column rows | [[screen/observatory-insights]] |
| Soon-column rows | [[screen/observatory-insights]] |
| Today hero koan | [[screen/observatory-today]] (top-1 by strength) |
| Project overview hero | [[screen/project-overview]] (top-1 for the project scope) |
| Applied → MeasureVerdicts | [[pipeline/impact]] |
| Dismissed signature | suppression table used by `generate.rs` on next tick |

## Done gate

- On Jerry's live data, the generator produces a stable set of
  candidates per project across ticks — same evidence → same
  candidate id.
- Every `proposed` recommendation with `impact_level = high` shows
  up in the Now column at the correct scope.
- `Apply` on a recommendation triggers a `MeasureVerdicts`
  follow-up scheduled at `now + measurement_window`.
- Reviewed recommendations decay to `dismissed` after
  `REVIEW_DECAY_DAYS` without new evidence.
- Dismissed signatures don't re-fire on the next tick.
- Every recommendation's user-visible strings come through
  insight-copy when the model is available.
- No recommendation is duplicated across scopes (project + user)
  for the same signature — dedup by signature honors the scope
  hierarchy.
- Every apply is attributable to a user in the log
  (`state_changed_by`).

Optional check:
```
curl -s http://localhost:7744/api/insights | jq '{
  now: .counts.now,
  soon: .counts.soon,
  proposed_high: [.memories, .recommendations, .patterns, .corrections | length] | add
}'

# After an apply, is MeasureVerdicts scheduled?
psql -A -t -c "select count(*) from sensei.task_queue
                where kind = 'MeasureVerdicts'
                  and created_at > now() - interval '5 minutes'" -d sensei
```

## Wrong gate

- **A signature reappears in Now the tick after dismissal.**
  Dismissal suppression table not consulted.
- **Applied recommendation never triggers `MeasureVerdicts`.**
  Follow-up scheduling regressed (a recurring bug — see
  [[pipeline/analyzer]] wrong-gate).
- **Every recommendation title reads identically** ("Consider
  adopting pattern X"). Insight-copy cache-key collision, OR
  fallback template used even when the model was available.
- **Reviewed recommendations sit in Soon forever.** Decay task
  isn't running.
- **A high-impact recommendation isn't in Now.** Bucketing rule
  divergence between generator and reader.
- **Apply verb sends to a specific assistant but the user's
  active assistant is different.** `default_acp` should
  respect the user's active-assistant setting when known.
- **Same recommendation exists at project AND user scope for
  the same signature.** Scope hierarchy dedup missing (tighter
  scope should win, matching [[pipeline/memory]] rules).
- **`impact_projection` says "+15% FTR" but the follow-up
  measurement never lands within the measurement window.**
  Measurement path broken; the number becomes noise.

## Effectiveness correlation

Not every rec is created equal. Some tools / patterns / memories
predict FTR movement more than others. The pipeline runs
effectiveness correlation on the enriched session corpus:

- For each promoted memory, for each pattern, for each tool: FTR
  when applied vs FTR when absent. Delta persisted in
  `sensei.effectiveness_correlations` (keyed by
  `(subject_type, subject_id)` — memory/pattern/tool).
- Feeds:
  - Rec ranking (recommendations whose subject has high
    effectiveness bubble up).
  - Insights hint on landing cards ("this pattern lifted FTR
    +18% in similar sessions").
  - Model-effectiveness view ((memory: project_standalone_completion_plan)
    already shipped a per-model version).

## Change-impact tracking

Every accepted recommendation is a change — sensei measures its
impact over a bounded window. See [[pipeline/impact]] for the
verdict machinery; the pipeline hooks it up:

- On accept, snapshot baseline (`ftr_14d`, `sessions_7d`,
  `corrections_7d`) into `sensei.applied_recommendations`.
- Schedule `MeasureVerdicts` for `now + measurement_window`.
- On verdict landing:
  - `positive` → reinforce underlying memory / promote pattern.
  - `negative` → open a regression alert (see
    [[pipeline/impact]] regression alerts).
  - `insufficient_data` → re-schedule with a longer window; cap
    on retries.

## MOE reasoning integration

For high-stakes recommendations (a memory that might contradict
existing state, a pattern promotion, a negative-verdict
analysis), the generator calls
[[pipeline/inferencing]] `consensus` chain and stores the
reasoning trace:

- `sensei.reasoning_traces` — one row per MOE run (see
  [[screen/insights-reasoning]]).
- The user can open the reasoning drawer from the rec card to
  see the propose/challenge/synthesize debate + disagreements.
- Confidence label from MOE feeds the rec's presentation —
  `low` confidence recs render with a "sensei is uncertain
  about this — the reasoning is worth checking" note.

## Playground + Replay integration

Two related surfaces that feed effectiveness signals:

- **Playground** ([[screen/observatory-instruments-playground]])
  — user-triggered tool executions. Attributed to the user (not
  the assistant) so they don't corrupt behavioural signals; but
  DO count for "which tools does the developer find useful?"
  effectiveness. Feeds hint copy: "the Playground got 12 hits
  on `search_lib_docs` this week — consider surfacing it
  in the assistant's default context".
- **Replay** ([[screen/observatory-instruments-replay]]) — the
  audit trail. Not a data producer but the consumer of
  effectiveness data: each tool call row in a replay carries
  its effectiveness chip ("used", "partial", "ignored") from
  [[pipeline/signals]] verdict classifier.

## Negative-impact detection

When a verdict returns `negative`, the pipeline runs a follow-up
analysis:

1. Cluster subsequent corrections by signature.
2. Compare against the rec's expected effect.
3. Via MOE reasoning, propose a revision candidate:
   - "keep, but adjust scope"
   - "roll back the memory"
   - "the correlation was spurious; different variable at play"
4. Presents the analysis on
   [[screen/observatory-impact]] regression detail.

The user acts on the revision (Revert / Keep / Investigate).

## Related

- [[pipeline/analyzer]] — schedules `generate` per project tick
- [[pipeline/memory]] — parallel state machine for the *statement*
  version of the same evidence
- [[pipeline/impact]] — where `Apply` decisions get their verdict
- [[pipeline/insight-copy]] — user-visible strings
- [[pipeline/signals]] — correction signature source
- [[screen/observatory-insights]] — the triage surface
- [[screen/observatory-today]] — the top-1 landing surface
- [[screen/project-overview]] — the top-1 project-scope surface
- [[screen/observatory-impact]] — where applied recs' verdicts land
