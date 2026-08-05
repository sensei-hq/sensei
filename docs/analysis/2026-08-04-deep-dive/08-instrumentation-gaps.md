## Instrumentation Gaps — What We Cannot Measure Today
_Before we can trust any new metric, we have to admit which columns are empty. This section inventories the blind spots that block every downstream metric — model attribution, tool error-rate, verdict coverage, correction/memory promotion, cost, and code quality — and quantifies each from the live schema._

**User's observation.** The metrics doc asks for productivity/velocity, rework, and deterministic code-quality (qlty.sh) and coverage snapshots, "captured on a daily/session basis so that we can also look at trends." It also flags **Amnesia** — durable context loses facts the system already had. Both requests assume the raw signal exists. It largely does not. This section applies the mandatory rule — _measure first, then keep what helps_ — to the instrumentation layer itself: you cannot compute per-model FTR, tool error-rate, or a quality trend on columns that are 100% NULL.

**What the data shows.**

- **Model & provider attribution on sessions is 100% absent.** All 69 sessions have NULL `model` and NULL `provider`. Every per-model or per-provider metric (FTR-by-model, velocity-by-model, cost-by-model) is currently uncomputable at the session grain.
  ```sql
  SELECT count(*) total, count(model) model_nonnull, count(provider) provider_nonnull
  FROM activity.sessions;              -- 69 | 0 | 0
  ```

- **But the attribution EXISTS one table over.** `activity.transcript_turns` carries `model`+`provider` on **1378 / 1517 rows (90.8%)**, spanning **16 distinct models** across **4 providers** (anthropic, copilot_chat, ollama, openai). The daemon captures model per-turn and then throws it away at the session rollup. The 139 NULLs are all `family='claude'`; every `family='zed'` turn (340) is attributed.
  ```sql
  SELECT count(*), count(model), count(distinct model), count(distinct provider)
  FROM activity.transcript_turns;      -- 1517 | 1378 | 16 | 4
  ```

  | provider | model | turns |
  |---|---|---:|
  | anthropic | claude-opus-4-8 | 997 |
  | copilot_chat | claude-3.7-sonnet | 113 |
  | copilot_chat | claude-sonnet-4 | 55 |
  | copilot_chat | GPT-5 | 43 |
  | anthropic | claude-opus-4-7 | 37 |
  | _(NULL)_ | _(NULL)_ | 139 |

- **`sessions.model` is backfillable TODAY for 46 / 69 sessions (66.7%)** by joining `transcript_turns.session_id::text = sessions.client_session_id`. This is not a new pipeline — it is a `UPDATE ... FROM` the daemon can run on the next rollup. The remaining 23 sessions have no transcript rows to attribute from.
  ```sql
  SELECT count(DISTINCT s.id) FROM activity.sessions s
  WHERE EXISTS (SELECT 1 FROM activity.transcript_turns tt
                WHERE tt.session_id::text = s.client_session_id AND tt.model IS NOT NULL);  -- 46
  ```

- **`assistant_events.success` is NULL for 100% of rows — every event type, not just PostToolUse.** Across 131,690 events (including **60,119 PostToolUse** and 61,694 PreToolUse), `count(success) = 0`. There is no tool-level pass/fail signal anywhere, so tool error-rate, retry-rate, and "which tool fails most" are all uncomputable from the event stream.
  ```sql
  SELECT event_type, count(*), count(success) FROM activity.assistant_events
  GROUP BY event_type;   -- PreToolUse 61694|0, PostToolUse 60119|0, SubagentStop 4910|0, ...
  ```

- **Tool-call verdicts classify only 38.3% of tool calls, and coverage is worst on the highest-signal tools.** `sensei.tool_call_verdicts` has 23,638 rows against 61,698 PreToolUse calls. `Edit` (9,714 calls) is classified at 28.4%, `StructuredOutput` at 22.4%, `svelte-autofixer` at 23.8% — the edit/verification tools we most want a "used vs ignored" ruling on are the least covered.

  | tool_name | calls | verdicts | coverage % |
  |---|---:|---:|---:|
  | Bash | 26,750 | 10,521 | 39.3 |
  | Read | 16,659 | 6,380 | 38.3 |
  | Edit | 9,714 | 2,754 | **28.4** |
  | Write | 2,376 | 937 | 39.4 |
  | TaskCreate | 461 | 279 | 60.5 |
  | svelte-autofixer | 488 | 116 | **23.8** |
  | StructuredOutput | 357 | 80 | **22.4** |

  ```sql
  WITH pt AS (SELECT tool_name, count(*) calls FROM activity.assistant_events
              WHERE event_type='PreToolUse' GROUP BY tool_name),
       v  AS (SELECT tool_name, count(*) verdicts FROM sensei.tool_call_verdicts GROUP BY tool_name)
  SELECT pt.tool_name, pt.calls, coalesce(v.verdicts,0),
         round(100.0*coalesce(v.verdicts,0)/pt.calls,1)
  FROM pt LEFT JOIN v USING(tool_name) ORDER BY pt.calls DESC;
  ```

- **Verdict/session identity is leaky.** Verdicts reference **76 distinct session_ids, but 14 of them do not exist** in `activity.sessions` (joined on `client_session_id`; the columns are even different types — `verdicts.session_id` is `text`, `sessions.id` is `uuid`). Likewise `assistant_events` emits from 76 session_ids, 10 of which have no `activity.sessions` row. Sessions are being classified that the session table never recorded.
  ```sql
  SELECT count(DISTINCT v.session_id) FILTER (WHERE s.id IS NULL)
  FROM sensei.tool_call_verdicts v
  LEFT JOIN activity.sessions s ON s.client_session_id = v.session_id;   -- 14
  ```

- **`sessions.tokens_in/tokens_out` are 100% NULL — total cost blindness.** The columns exist (`integer`) but not one of 69 sessions carries a value, and `sum()` is NULL. No cost, no tokens/feature, no $/FTR is possible. (Note: this contradicts the working assumption that "tokens are present" — they are not.)
  ```sql
  SELECT count(tokens_in), count(tokens_out), sum(tokens_in) FROM activity.sessions;  -- 0 | 0 | NULL
  ```

- **The correction pipeline is effectively dead: `inference.corrections` has 1 row.** That single row aggregates 20 instances across 6 projects — yet there are **38 `is_correction` turns**, **13 correction/revert-named anti-patterns**, and 932 anti-patterns overall. The consolidation job that should mint correction signatures ran essentially once (last_seen 2026-08-02) and stopped.
  ```sql
  SELECT count(*) FROM inference.corrections;                          -- 1
  SELECT count(*) FILTER (WHERE is_correction) FROM activity.turns;    -- 38
  ```

- **Memory promotion is stalled and its counters are dead.** 11 memories total (9 `learned`, 2 `authored`, all `convention`). Across all 11, `sum(reinforced_count)=1` and `sum(violated_count)=0`, and **0 of 943 patterns carry a `memory_id` backlink**. So 943 patterns + 1478 recommendations + 1947 drift items funnel into 11 durable memories, and nothing tracks whether those memories are reinforced or violated in practice. This is the measurable core of **Amnesia**.
  ```sql
  SELECT origin, count(*), sum(reinforced_count), sum(violated_count)
  FROM sensei.memories GROUP BY origin;    -- learned 9|1|0 , authored 2|0|0
  ```

- **The recommendation feedback loop is instrumented but never fires.** 1478 recommendations, but only **1 has `acted_at`, 1 has `measured_at`, and 0 have `baseline_ftr`** — 1477 (99.9%) are `pending`. The columns to prove "did this advice raise FTR?" (`baseline_ftr`, `current_ftr`, `measured_at`) exist and are empty. We cannot show any recommendation moved a metric.
  ```sql
  SELECT count(*), count(acted_at), count(measured_at), count(baseline_ftr)
  FROM inference.recommendations;          -- 1478 | 1 | 1 | 0
  ```

- **Deterministic code-quality / test-coverage is NOT stored — and the table meant to hold it is unfed.** There is no qlty.sh/lint/complexity table. `sensei.project_quality_signals` (128 rows, one per project) is the intended home and has the right columns — but **`test_pass_rate` is NULL for all 128** and `pattern_compliance` is populated on 7 rows yet **zero on all of them**. Only the FTR/drift columns (derivable from existing tables) are filled. `sensei.doc_coverage` (1.7M edges) is **doc↔code traceability, not test coverage** — 1.33M of its edges (78%) are marked `drifted`. `sensei.scan_state` (44k rows) is the incremental indexer's file-hash tracker, not a quality scan.
  ```sql
  SELECT count(test_pass_rate) tpr, count(*) FILTER (WHERE pattern_compliance<>0) pc_nz
  FROM sensei.project_quality_signals;     -- 0 | 0
  ```

- **Autonomous runs cannot be attributed to sessions.** All 9 runs carry a `dojo_session_id`, but **0 of them match any `activity.sessions.id`**. Autonomous run work (done=5, crashed=3, failed=1) is a disconnected island — no way to roll a run's turns, tokens, or corrections up from the session table.
  ```sql
  SELECT count(*) FILTER (WHERE dojo_session_id IN (SELECT id FROM activity.sessions))
  FROM activity.runs;                      -- 0   (of 9)
  ```

- **Pattern semantics are empty where the UI needs them.** `inference.detected_patterns.description` is **NULL for all 943 rows**, `family` NULL for all 943, and `lifecycle` collapses to a single distinct value. This is the direct cause of the Observations note that the Patterns page "can't make out what the pattern is / what to do" — the human-readable field is simply never written. Separately, `inference.reasoning_traces` has consensus on all 88 rows but `action_proposed` NULL on all 88.

**Root cause / interpretation.**

The failure mode is consistent: **the schema is ahead of the producers.** Almost every gap above is a column or table that was designed correctly (right name, right type, right grain) and then never written by any job. `sessions.model`, `sessions.tokens_*`, `assistant_events.success`, `project_quality_signals.test_pass_rate`, `recommendations.baseline_ftr`, `corrections`, and `patterns.description` are all live columns holding NULL — not missing schema. The daemon captures the richer signal upstream (model per transcript turn, verdict per some tool calls, consensus per reasoning trace) but the enrichment/rollup step that would fold it back into the queryable grain either doesn't run or runs once and stops (`corrections` last touched 2026-08-02; recommendations 1477/1478 pending).

Two architectural seams produce most of this. First, **the event ingest path and the session-rollup path disagree on identity.** `assistant_events` and `tool_call_verdicts` key on a `text` client-session id; `activity.sessions` keys on a `uuid` `id` with `client_session_id` as a secondary text column; `activity.runs` keys on a `dojo_session_id uuid` that matches neither. So 14 verdict sessions and 10 event sessions have no session row, and 0 runs join to sessions. Any metric that needs to travel from a raw event to a session to a project has to cross a join that silently drops rows. This is also why the rollup drops `model` — the code that computes per-session aggregates evidently reads from a source that doesn't carry it, when `transcript_turns` (which does) is one join away.

Second, **the "did it work?" instruments were built as columns, not as a job.** `success`, `baseline_ftr`/`current_ftr`/`measured_at`, `reinforced_count`/`violated_count` are all outcome-measurement fields that require a second pass — observe the tool result, re-measure FTR N days after acting, watch a memory get honored or broken. None of those second passes exist as a scheduled producer, so the fields stay at their insert-time default of NULL/0. The consequence is that sensei can _detect_ (943 patterns, 1478 recommendations) far faster than it can _confirm_ (1 correction, 1 measured recommendation, 11 memories, 1 reinforcement). The detection:confirmation ratio is the quantified shape of every "unattended epic degrades" and "amnesia" complaint in the source docs.

The practical upshot for the rest of this analysis: **any metric proposed elsewhere that needs model, tokens, tool-success, correction-signatures, or code-quality is blocked at the source.** The good news is that most fixes are cheap because the columns already exist and one of them (model) is 66.7% backfillable from data already on disk.

**Recommendations.**

1. **(P0) Backfill and forward-populate `sessions.model` / `sessions.provider`.** Producer: the session-rollup job in `senseid` (activity ingest). Source already present — `activity.transcript_turns.model/provider`, joined on `client_session_id`. Backfill 46/69 immediately with a single `UPDATE`; wire the same derivation into the rollup so new sessions attribute on close. Effect: unlocks per-model FTR, velocity, and cost the instant tokens land. Take the modal (most-frequent) model per session to handle mid-session model switches.

2. **(P0) Populate `assistant_events.success` on PostToolUse.** Producer: the MCP/hook event writer that emits PostToolUse. The Claude Code hook payload carries tool result/error; map it to `success` at write time (and backfill from `payload` jsonb where the error field is present). Effect: enables tool error-rate, retry-rate, and per-tool reliability — none of which exist today.

3. **(P0) Raise verdict coverage above ~90% and fix session identity.** Producer: the verdict classifier (`sensei.tool_call_verdicts`). Two fixes: (a) run it over the full PreToolUse backlog (currently 38.3%, worst on `Edit`/`StructuredOutput`/`svelte-autofixer` at ~22–28%); (b) reconcile the `text` session id against `activity.sessions` and either enforce a FK or record why 14 verdict-sessions have no session row. Effect: "used vs ignored" tool-value metrics become trustworthy instead of sampled.

4. **(P0) Populate `sessions.tokens_in/tokens_out`.** Producer: session rollup, from the per-turn token usage the transcript pipeline already sees. Effect: removes total cost blindness; enables $/session, tokens/feature, and cost/FTR. Blocks nothing else but is cheap and high-leverage.

5. **(P1) Revive the correction-consolidation job.** Producer: `inference.corrections` writer. It has run once (1 row, last_seen 2026-08-02) while 38 correction turns and 13 correction-named anti-patterns accumulated. Schedule it (daily) to fold `is_correction` turns + revert/correction anti-patterns into signatures with `count`/`instances`/`project_ids`. Effect: makes **rework** (the metrics doc's "opposite of FTR") actually computable.

6. **(P1) Close the recommendation measurement loop.** Producer: a scheduled job that stamps `baseline_ftr` at recommendation creation and `current_ftr`/`measured_at` N days after `acted_at`. Today 1477/1478 are pending and `baseline_ftr` is 100% NULL. Effect: lets us prove (or kill) recommendations by FTR delta — the only honest way to "keep what helps."

7. **(P1) Wire memory reinforcement/violation counters + pattern→memory backlinks.** Producer: the promotion pipeline. `reinforced_count` sums to 1 across 11 memories, `violated_count` to 0, and 0/943 patterns carry `memory_id`. Increment on each honored/violated observation and set `memory_id` when a pattern is promoted. Effect: directly instruments **Amnesia** — a decaying/violated memory becomes visible and re-surfaceable.

8. **(P1) Write `patterns.description` (and `family`).** Producer: the pattern detector. 943/943 are NULL, which is why the Patterns screen is unreadable. Populate a one-line human description + a family at detection time. Effect: the app can render "what/why/what-to-do" without a second lookup.

9. **(P2) Feed `project_quality_signals.test_pass_rate` + a new deterministic quality snapshot.** Producer: a post-review "quality scan" job (qlty.sh/coverage) writing per-project, session-start and session-end. The table and `pattern_compliance`/`test_pass_rate` columns already exist and are unfed. Effect: enables the quality-delta and coverage trends the metrics doc asks for. Snapshot at session boundaries so "was a duplicate introduced this session" is a diff, not a guess.

10. **(P2) Give autonomous runs a session join.** Producer: run orchestrator. Add a resolvable link from `runs.dojo_session_id` to `activity.sessions` (0/9 currently join). Effect: run-level turns/tokens/corrections roll up from sessions instead of living on an island.

**Proposed metrics & instrumentation.**

Measurability matrix — for each metric proposed across this analysis, is the source data present today, and at what coverage.

| Metric | Definition / formula | Source (table.column) | Cadence | Present today? | Coverage |
|---|---|---|---|---|---|
| Per-model FTR | ftr grouped by model | `sessions.ftr` × `sessions.model` | session | **N** (model 0/69) | 0% (66.7% backfillable) |
| Per-provider velocity | features / time by provider | `sessions.provider` + turns | session | **N** | 0% |
| Cost / session, $/FTR | tokens × price | `sessions.tokens_in/out` | session | **N** | 0% |
| Tool error-rate | 1 − success on PostToolUse | `assistant_events.success` | daily | **N** | 0% (60,119 PostToolUse, 0 populated) |
| Tool used-vs-ignored | verdict distribution | `tool_call_verdicts.verdict` | session | **Partial** | 38.3% of calls (Edit 28.4%) |
| Rework rate | repeat fixes / component | `inference.corrections` + `turns.is_correction` | session | **Partial** | corrections 1 row; turns.is_correction 38 |
| FTR trend (per project) | ftr_14d vs prev | `project_ftr_metrics.ftr_14d` | daily | **Y** | 10/128 projects |
| Pattern compliance | conforming / detected | `project_quality_signals.pattern_compliance` | daily | **N** | 0 non-zero of 128 |
| Test pass rate | passing / total tests | `project_quality_signals.test_pass_rate` | session×2 | **N** | 0/128 |
| Code-quality score | qlty.sh scan delta | _(no table)_ | session×2 | **N** | 0% |
| Test coverage % | covered / total lines | _(no table; doc_coverage is traceability)_ | session×2 | **N** | 0% |
| Doc→code drift | drifted edges / total | `doc_coverage.drifted` | daily | **Y** | 78% edges drifted (1.33M/1.71M) |
| Recommendation impact | current_ftr − baseline_ftr | `recommendations.baseline_ftr/current_ftr` | on-act +Nd | **N** | 1/1478 measured |
| Memory health | reinforced − violated | `memories.reinforced_count/violated_count` | daily | **N** | reinforced Σ=1, violated Σ=0 |
| Autonomous-run FTR | run outcome × session rollup | `runs.dojo_session_id`→`sessions` | run | **N** | 0/9 runs join |
| Model attribution (turn) | model per turn | `transcript_turns.model` | turn | **Y** | 90.8% (1378/1517) |

Reading: of 16 candidate metrics, **3 are computable today** (turn-level model 90.8%, project FTR trend on 10 projects, doc drift), **2 are partial** (verdict coverage 38.3%, rework via a stalled corrections job), and **11 are blocked on empty columns** — 9 of which are P0/P1 fixes to producers, not schema changes. The single highest-leverage move is #1: model attribution is 66.7% recoverable with one `UPDATE` and unblocks the entire per-model analysis the metrics doc is built around.
