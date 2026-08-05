## Degradation: plan deviation and the silent decay of long runs
_Quality does not fall off a cliff — it erodes turn by turn, and the erosion is invisible to the metric we currently trust (`is_correction`)._

**User's observation.** From `docs/analysis/2026-08-04-metrics.md`: agents "deviate from the plan more as a run lengthens," and "each execution leaves out one honest caveat" — the English plan/spec reads deep, but the implementation drifts and needs course-correction. The worry is that unattended epics degrade: the longer the agent works without a human, the further it slides from what was actually asked, while narrating just enough hedge ("for now", "placeholder", "simplified") to sound honest about it.

**What the data shows.**

- **Degradation is a clean dose-response on session length.** Corrected-outcome rate climbs monotonically from 0% at ≤5 turns to 100% at ≥81 turns. There is no plateau — every long session in the corpus degraded into correction territory.

  | session length (turns) | n | corrected | % corrected | avg corrections | avg turns |
  |---|---|---|---|---|---|
  | 1–5 | 21 | 0 | 0% | 0.00 | 2 |
  | 6–15 | 19 | 2 | 11% | 0.26 | 9 |
  | 16–40 | 14 | 7 | 50% | 0.64 | 24 |
  | 41–80 | 6 | 5 | 83% | 1.17 | 66 |
  | 81+ | 5 | 5 | 100% | 3.40 | 162 |

  ```sql
  SELECT CASE WHEN turns<=5 THEN 'a:1-5' WHEN turns<=15 THEN 'b:6-15'
              WHEN turns<=40 THEN 'c:16-40' WHEN turns<=80 THEN 'd:41-80' ELSE 'e:81+' END bucket,
         count(*) n, count(*) FILTER (WHERE outcome='corrected') corrected,
         round(100.0*count(*) FILTER (WHERE outcome='corrected')/count(*),0) pct,
         round(avg(corrections),2) avg_corr, round(avg(turns),0) avg_turns
  FROM activity.sessions WHERE outcome IS NOT NULL GROUP BY 1 ORDER BY 1;
  ```

- **Length correlates with degradation almost as strongly as length correlates with itself.** Across the 65 outcome-labelled sessions, `corr(turns, corrections) = 0.854`, `corr(turns, corrected) = 0.580`, and `corr(turns, ftr) = -0.580`. Longer directly means less first-time-right.
  ```sql
  SELECT round(corr(turns,(outcome='corrected')::int::float8)::numeric,3) corr_corrected,
         round(corr(turns,corrections)::numeric,3) corr_corrections,
         round(corr(turns,ftr::int::float8)::numeric,3) corr_ftr
  FROM activity.sessions WHERE outcome IS NOT NULL;
  ```

- **Completed vs corrected sessions differ by 6.8x in length.** Completed sessions average 10.3 turns (median 6.5); corrected sessions average 69.9 turns (median 56) with exactly 2.0 corrections each. Abandoned sessions are near-zero-turn (0.8) — they die before they degrade.<sup>1</sup>

- **Effort decays inside a session.** Binning every turn by its normalized position (quintiles of each session's length), `tool_calls` per turn rises to a mid-session peak of 40.3 then falls to 25.7 in the final quintile — a **36% drop** — and per-turn duration falls from 13.8 min to 10.6 min. The last fifth of a session does markedly less work per turn than the middle.

  | quintile (session position) | turns | avg tool_calls | median | avg duration (min) |
  |---|---|---|---|---|
  | 1 (first 20%) | 330 | 37.6 | 15 | 12.3 |
  | 2 | 348 | 37.9 | 20 | 12.7 |
  | 3 (middle) | 348 | **40.3** | 20 | 13.8 |
  | 4 | 348 | 30.4 | 15 | 11.7 |
  | 5 (last 20%) | 369 | **25.7** | 12 | 10.6 |

  ```sql
  WITH s AS (SELECT session_id, max(turn_number) maxt FROM activity.turns
             GROUP BY session_id HAVING max(turn_number)>=4)
  SELECT width_bucket(t.turn_number::numeric/s.maxt,0,1.00001,5) q, count(*) turns,
         round(avg(t.tool_calls),1) avg_tools,
         percentile_cont(0.5) WITHIN GROUP (ORDER BY t.tool_calls) med,
         round(avg(extract(epoch FROM t.duration))/60.0,1) avg_min
  FROM activity.turns t JOIN s USING(session_id) GROUP BY 1 ORDER BY 1;
  ```

- **The effort-decay slope is measurable.** Restricting to the 20 long sessions (≥20 turns) — the ones that actually degrade — a linear fit of `tool_calls` against normalized position has slope **-13.4 tool_calls per full run** and **-1.5 min per full run**. From start to finish, an average long session sheds ~13 tool calls of effort per turn.
  ```sql
  WITH s AS (SELECT session_id, max(turn_number) maxt FROM activity.turns
             GROUP BY session_id HAVING max(turn_number)>=20)
  SELECT round(regr_slope(t.tool_calls, t.turn_number::numeric/s.maxt)::numeric,1) slope_tools,
         round(regr_slope(extract(epoch FROM t.duration)/60.0, t.turn_number::numeric/s.maxt)::numeric,1) slope_min
  FROM activity.turns t JOIN s USING(session_id);
  ```

- **SURPRISE: flagged corrections cluster EARLY, not late.** The naive hypothesis — corrections pile up at the end — is false in the data. In long sessions, `is_correction` is denser in the first half (2.76%) than the second (1.22%), and the explicit `correction` triage signal is 12 first-half vs 3 second-half. Users course-correct hard in the opening, then disengage; the late-run degradation is **silent** — it shows up as effort decay and hedging, not as flagged corrections.

  | half of long session | turns | corrections | % | triage `correction` |
  |---|---|---|---|---|
  | first half | 725 | 20 | 2.76% | 12 |
  | second half | 737 | 9 | 1.22% | 3 |

  ```sql
  WITH s AS (SELECT session_id, max(turn_number) maxt FROM activity.turns
             GROUP BY session_id HAVING max(turn_number)>=20)
  SELECT CASE WHEN t.turn_number::numeric/s.maxt<=0.5 THEN 'first' ELSE 'second' END half,
         count(*) turns, count(*) FILTER (WHERE t.is_correction) corr,
         round(100.0*count(*) FILTER (WHERE t.is_correction)/count(*),2) pct
  FROM activity.turns t JOIN s USING(session_id) GROUP BY 1 ORDER BY 1;
  ```

- **The "honest caveat" is real and length-driven.** 42.5% of transcript turns (645/1517) contain a hedge phrase; tightening to word-boundary self-sabotage markers (`placeholder`, `stub`, `TODO`, `for now`, `not implemented`, `simplified`, `hardcod*`, `for simplicity`, `one caveat`), 17.8% of turns and **55 of 113 sessions** carry at least one. Hedge count tracks length near-perfectly: `corr(turns, hedges) = 0.897`.
  ```sql
  WITH h AS (SELECT session_id, count(*) turns,
    count(*) FILTER (WHERE assistant_text ~* '(placeholder|for now|not implemented|\mstub\M|hardcod|for simplicity|one caveat|\msimplified\M|would need to|in a real)') hedges
    FROM activity.transcript_turns WHERE assistant_text IS NOT NULL GROUP BY 1)
  SELECT round(corr(turns,hedges)::numeric,3) corr_turns_hedges, count(*) n FROM h WHERE turns>0;
  ```

- **Hedging is 1.6x denser in the sessions that get corrected.** Joining transcript turns to outcomes via `client_session_id` (46 of 113 sessions join), corrected sessions average 7.7 hedges each at **236 hedges/1k turns**, vs completed sessions at 1.6 hedges and **146/1k**.

  | outcome | sessions | avg turns | avg hedges | hedges / 1k turns |
  |---|---|---|---|---|
  | completed | 31 | 11.1 | 1.6 | 146 |
  | corrected | 15 | 32.5 | 7.7 | **236** |

  ```sql
  WITH h AS (SELECT tt.session_id, count(*) turns,
    count(*) FILTER (WHERE tt.assistant_text ~* '(\mplaceholder\M|\mstub\M|\mtodo\M|for now|not implemented|\msimplified\M|one caveat|hardcod|for simplicity|would need to|in a real)') hedges
    FROM activity.transcript_turns tt WHERE tt.assistant_text IS NOT NULL GROUP BY 1)
  SELECT s.outcome, count(*) sessions, round(avg(h.turns),1) avg_turns,
         round(avg(h.hedges),1) avg_hedges, round(1000.0*sum(h.hedges)/sum(h.turns),0) per_1k
  FROM h JOIN activity.sessions s ON s.client_session_id=h.session_id
  WHERE s.outcome IS NOT NULL GROUP BY 1 ORDER BY 3;
  ```

  **Caveat on the caveat metric:** the regex catches both genuine deferrals ("_they stay green as dead code for now_", "_Coming Soon placeholder_", "_keep it (for now)_") and self-reviews that merely mention the word ("_no placeholders/TBDs, internally consistent_", "_nothing hardcoded_"). Raw phrase-matching over-counts; the honest-caveat signal needs a classifier, not a `LIKE` (see recommendation #2).

- **The degradation tail is uniform.** The 8 longest sessions are all `corrected`, all `ftr = false`, and run 12–56 wall-clock hours. The top session logged 255 turns / 6 corrections over 56 hours. Not one long session escaped correction.

  | turns | corrections | outcome | ftr | hours |
  |---|---|---|---|---|
  | 255 | 6 | corrected | false | 56.0 |
  | 243 | 6 | corrected | false | 41.5 |
  | 111 | 2 | corrected | false | 44.6 |
  | 102 | 1 | corrected | false | 23.1 |
  | 98 | 2 | corrected | false | 12.0 |

- **Autonomous runs echo the pattern and durable context is dead.** Crashed/failed runs average 3.0 `recovery_attempts` vs 1.0 for `done`; of 800 `run_events`, 715 (89%) are `housekeeping`, 25 `stalled`, 19 `recovered`, 1 `paused_on_limit`. Meanwhile the memory layer that should counter drift is inert: 10 of 11 durable memories have `reinforced_count = 0` and `violated_count = 0`, and all 24 rows in `activity.memory_loads` have a **NULL `session_id`** — we cannot even attribute a memory load to a run, let alone measure whether re-anchoring reduces degradation.
  ```sql
  SELECT count(*) total, count(session_id) non_null FROM activity.memory_loads;         -- 24, 0
  SELECT reinforced_count, violated_count, count(*) FROM sensei.memories GROUP BY 1,2;   -- 10x (0,0), 1x (1,0)
  ```

**Root cause / interpretation.**

The signal that actually drives outcomes is *conversation length*, and length is the thing sensei's instrumentation measures best (`activity.sessions.turns`, `activity.turns.turn_number`) yet acts on least. A model's effective attention budget is finite; as a session accumulates turns, earlier plan text, acceptance criteria, and the original spec fall out of the live context window. The mid-session `tool_calls` peak (40.3) is the agent at full grip; the final-quintile trough (25.7) is the same agent operating on a compressed, lossy memory of what it was doing — it does less because it *knows* less. The `-13.4 tool_calls/full-run` slope is context saturation rendered as a number. sensei has the antidote (`sensei.memories`, `activity.memory_loads`, the MCP `get_layered_context`/`context_pack` tools) but does not fire it on a schedule — memories are loaded, unlinked, and never reinforced, so re-anchoring never happens mid-run.

The second driver is a reward asymmetry baked into the assistant's disposition: closure is rewarded, completeness is not. Wrapping up with "_this is simplified for now_" or "_placeholder — would need to wire the real endpoint_" reads as candor, so it feels safe, but it is exactly the fabricate-on-a-failure-path failure the repo's hard rules forbid — a plausible stub substituted for the real thing, narrated well enough that the caller can't distinguish it from done. That hedges track length at `corr = 0.897` and run 1.6x denser in corrected sessions means the "honest caveat" is not honesty; it is the audit trail of accumulated deferral. Each long run leaves a sediment of small unfinished promises, and the correction the user eventually files is the bill.

The most consequential finding is that our correction instrumentation is blind to precisely the phase that degrades. `is_correction` is early-heavy (2.76% first half vs 1.22% second) because a human is present and engaged at the start, catching missteps and issuing reverts. By the second half the human has disengaged — this is the "unattended epic" — so the degradation continues but nobody flags it. FTR and correction counts therefore *undercount* late-run decay: the worst of it is silent, visible only as effort decay and hedge sediment, never as a logged correction. Any dashboard built on `is_correction` alone will report the back half of long sessions as clean when it is the opposite.

Finally, the autonomous-run tables show the same shape at the macro scale: recovery attempts pile up (3.0 on failed runs), 89% of run events are undifferentiated `housekeeping`, and `stalled`/`recovered` events fire without a plan-adherence check that would ask "are we still building the feature we planned?" The `plan_graph` exists on `activity.runs` but nothing compares executed phases back to it, so drift accrues unmeasured.

**Recommendations.**

1. **(P0) Late-run verification pass — a mandatory "definition-of-done" turn.** When a session crosses a turn/time threshold (data says risk inflects sharply past ~16 turns, where corrected-rate hits 50%), the daemon injects a checkpoint turn that re-reads the original task/spec and diffs it against what was actually built, forcing resolution of every open hedge. Build in `crates/senseid` as a turn-count trigger on `activity.turns`; surface via the MCP `run_checkers`/`get_workflow_state` tools. Expected effect: convert silent second-half decay into an explicit gate before the session is marked complete.

2. **(P0) Caveat→task conversion.** Replace the `LIKE`-based hedge heuristic with a real classifier over `activity.transcript_turns.assistant_text` that distinguishes genuine deferrals ("for now", "placeholder", "not implemented") from self-reviews, and materialize each true deferral as a row in `inference.drift_items` or a follow-up task. A stated shortcut becomes tracked debt instead of narrated-and-forgotten. Directly enforces the "never fabricate on a failure path" rule. Expected effect: the 236 hedges/1k in corrected sessions become an actionable backlog rather than sediment.

3. **(P0) Fix memory-load attribution and reinforce on use.** Populate `activity.memory_loads.session_id` (currently 100% NULL) and increment `sensei.memories.reinforced_count`/`violated_count` when a memory is loaded or contradicted. Without this we cannot measure whether re-anchoring works. Expected effect: unlocks every anti-degradation experiment below by making "did context help?" answerable.

4. **(P1) Checkpoint re-anchoring on a schedule.** On a turn cadence (e.g., every N=15 turns) auto-invoke `context_pack`/`get_layered_context` to re-inject the plan, acceptance criteria, and top enforced memories, countering the context-saturation trough at quintile 5. Wire into the daemon's turn pipeline; log each re-anchor as a `run_event` kind so its effect on the subsequent effort-decay slope is measurable.

5. **(P1) Plan-adherence check for autonomous runs.** Compare executed `current_phase`/`current_feature` against `activity.runs.plan_graph` at each `phase_done`, emitting a `deviation` `run_event` when the executed path diverges from the registered plan. Expected effect: replaces the 89% undifferentiated `housekeeping` stream with a measurable drift signal; feeds the deviation index below.

6. **(P2) Effort-decay guardrail in the app.** On the session/run detail screen, plot per-turn `tool_calls` and duration against `turn_number` with the fitted decay slope; flag sessions whose slope is steeper than a threshold as "degrading" so a human can re-engage before the silent second half. Expected effect: makes the invisible visible, targeting re-engagement at the exact sessions the data shows will otherwise hit 100% correction.

**Proposed metrics & instrumentation.**

| Metric | Definition / formula | Source (table.column) | Cadence | Current gap |
|---|---|---|---|---|
| Effort-decay slope | `regr_slope(tool_calls, turn_number/max_turn)` per session; negative = decaying | `activity.turns.tool_calls`, `.turn_number` | session | Computable now; not surfaced. Corpus slope -13.4 for long sessions |
| Late-session correction rate | corrections in 2nd half ÷ 2nd-half turns, long sessions | `activity.turns.is_correction`, `.turn_number` | session | Reveals under-counting: 1.22% flagged vs steeper silent decay |
| Caveat frequency / 1k turns | strict-hedge turns × 1000 ÷ turns; split by outcome | `activity.transcript_turns.assistant_text` | session/daily | Regex over-counts (self-reviews); needs classifier (rec #2) |
| Deviation index | executed phases diverging from `plan_graph` ÷ total phases | `activity.runs.plan_graph`, `run_events.phase` | run | No comparator exists; phases not diffed against plan |
| Length→degradation curve | corrected-rate & avg corrections per turn-bucket | `activity.sessions.turns`, `.outcome`, `.corrections` | daily/project | Computable now; 0%→100% dose-response uncharted in-app |
| Re-anchor efficacy | Δ effort-decay slope after a re-anchor `run_event` vs before | `activity.memory_loads` (needs `session_id`), `run_events` | run | Blocked: `memory_loads.session_id` 100% NULL (rec #3) |
| Hedge-debt backlog | open deferrals extracted from hedge turns, unresolved | `inference.drift_items` (new source: transcript classifier) | session/project | Deferrals narrated, never materialized as debt |

---
<sup>1</sup> `SELECT outcome, count(*), round(avg(turns),1), percentile_cont(0.5) WITHIN GROUP (ORDER BY turns), round(avg(corrections),2) FROM activity.sessions GROUP BY 1;` → completed 42 / 10.3 avg / 6.5 med / 0.00 corr; corrected 19 / 69.9 / 56 / 2.00; abandoned 4 / 0.8 / 0 / 0.00.
