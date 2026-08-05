## Productivity & Velocity: Count Finished Work, Not Volume

_The system records how much an agent typed, never how much it delivered — so today the loudest sessions look the most productive when they are actually the most-reworked._

**User's observation.** From `docs/analysis/2026-08-04-metrics.md`: productivity should be measured as **"N features × complexity / time, not LOC."** The note asks how to compare volume/progress across heterogeneous work (db / api / ui / pure-fn), proposes velocity as complexity-weighted feature throughput, and defines **rework as the opposite of FTR** — repeated fixes on the same component. It also flags that offering to "discard this work" is dangerous/wasteful, i.e. sunk effort must be measured, not thrown away. This section asks: with the data sensei has today, can we build that metric — and what is missing?

**What the data shows.**

- **We have four raw output signals and zero unit of "work done."** Across n=69 sessions (2026-07-06 → 2026-08-04) the daemon records `turns`, per-turn `tool_calls`, and event-level `Edit`/`Write` counts — 9,586 `Edit` and 2,282 `Write` PostToolUse events — but no field anywhere says "this session shipped X features."<sup>1</sup> Every productivity signal we can compute today is a **volume** signal.

- **The productivity paradox: the reworked minority dominates every volume metric.** The 19 `corrected` sessions are 27.5% of all sessions but consume **76.3% of tool-calls, 75.4% of turns, 74.1% of Edit/Write ops, and 71.7% of wall-clock time.** Any velocity metric built on raw volume would rank the worst-behaving sessions as the most productive.

  ```sql
  WITH tc AS (SELECT session_id, sum(tool_calls) tool_calls FROM activity.turns GROUP BY session_id)
  SELECT round(100.0*sum(tc.tool_calls) FILTER (WHERE s.outcome='corrected')/sum(tc.tool_calls),1) pct_toolcalls,
         round(100.0*sum(s.turns)       FILTER (WHERE s.outcome='corrected')/sum(s.turns),1)      pct_turns,
         round(100.0*count(*)           FILTER (WHERE s.outcome='corrected')/count(*),1)          pct_sessions,
         round(100.0*sum(EXTRACT(epoch FROM s.duration)) FILTER (WHERE s.outcome='corrected')
               /NULLIF(sum(EXTRACT(epoch FROM s.duration)),0),1)                                  pct_duration
  FROM activity.sessions s LEFT JOIN tc ON tc.session_id=s.id;
  ```

- **Per-outcome, corrected sessions cost 5–7× more of everything.** A `corrected` session averages 2,405 tool-calls and 366.7 edits; a `completed` one averages 364 tool-calls and 65.4 edits. Reads are the sharpest divergence — 687 vs 82 — the agent thrashes the codebase looking for the thing it broke.

  | outcome | sessions | avg turns | avg tool_calls | avg Edit | avg Read | avg corrections |
  |---|---|---|---|---|---|---|
  | corrected | 19 | 69.9 | 2,405 | 366.7 | 687.4 | 2.00 |
  | completed | 42 | 10.3 | 364 | 65.4 | 82.4 | 0.00 |
  | abandoned | 4 | 0.8 | 21 | 1.0 | 4.3 | 0.00 |

  ```sql
  -- outcome × volume (turns table for tool_calls, assistant_events for Edit/Read)
  SELECT s.outcome, count(*), round(avg(s.turns),1), round(avg(tc.tool_calls)), round(avg(s.corrections),2)
  FROM activity.sessions s
  LEFT JOIN (SELECT session_id, sum(tool_calls) tool_calls FROM activity.turns GROUP BY session_id) tc
    ON tc.session_id=s.id GROUP BY s.outcome;
  ```

- **A volume leaderboard ranks the failures first.** The 10 highest tool-call sessions: 8 are `corrected`/`ftr=false`. The top three (8,284 / 6,794 / 6,566 tool-calls) are all corrected. The only two `completed`/`ftr=true` sessions in the top 10 sit at ranks 6 and 11.<sup>2</sup> LOC-style ranking is not just imperfect here — it is **inverted**.

- **Time confirms it: rework runs 5.5× longer.** `completed` sessions average 178.6 min of wall-clock; `corrected` average **977.1 min (16.3 h)**. Caveat: `activity.sessions.duration` is `completed_at − started_at`, i.e. wall-clock including idle gaps between turns — it overstates active effort and cannot be used as a labor denominator without a heartbeat-based active-time measure.<sup>3</sup>

- **Cost-per-feature is uncomputable today — token accounting is entirely absent.** `tokens_in`/`tokens_out` are NULL for **all 69 sessions** (the earlier "tokens present" baseline is wrong). No payload carries usage either: **0 of 131,692** `assistant_events` payloads contain a `usage`/`tokens`/`input_tokens` key. `model` appears in only 98 payloads (all `claude-opus-4-8[1m]`); `activity.sessions.model`/`provider` are 100% NULL.<sup>4</sup> There is no way to price a unit of work.

- **The code graph cannot be attributed to a session.** `sensei.nodes.degree` is NULL for **all 476,978 nodes** and `props` is `{}` for all — so graph-degree as a complexity proxy is unavailable off-the-shelf. `nodes.modified_at` is an **indexing** timestamp, not authoring time: 7,150 nodes carry a 2026-08-04 stamp against only 6 sessions that day, and daily node counts (e.g. 3,333 on 07-23) track re-index sweeps, not sessions.<sup>5</sup> **No per-session graph delta is captured**, so "nodes/edges added by this session" — the natural complexity signal — does not exist yet.

- **The only "feature completed" events in the entire database live in autonomous runs, and there are 13 of them.** `activity.run_events` has 6 `feature_done` + 7 `phase_done` events, across 9 runs. **89% of run_events (715/800) are `housekeeping` heartbeats** — the babysitting pulse — leaving 11% substantive. The 69 interactive sessions, which do the bulk of the work, contribute **zero** feature/phase structure.

  | run_event kind | n | | run_event kind | n |
  |---|---|---|---|---|
  | housekeeping | 715 | | feature_done | 6 |
  | stalled | 25 | | crashed | 5 |
  | recovered | 19 | | feature_started | 5 |
  | phase_started | 9 | | paused_on_limit | 1 |
  | done / phase_done | 7 / 7 | | failed | 1 |

  ```sql
  SELECT kind, count(*) FROM activity.run_events GROUP BY kind ORDER BY 2 DESC;
  ```

- **There is no planned-work denominator either.** All **128 projects are `maturity='discovery'`**, `jsonb_array_length(backlog)=0` for every one, and `goal IS NULL` for every one; only 59/128 carry a `stack`. Runs are thin too: 5 runs have a `plan_graph` with 10 planned phases total, against 7 `phase_done`. So "% of plan completed" is computable only inside a handful of runs, never per project or per session.<sup>6</sup>

- **A node-kind-weighted complexity index is defensible and re-ranks the work.** Even without per-session deltas, weighting node kinds (component 5, hook/interface/class/struct 4, type/function/enum/extension 3, method/module 2, file/doc 1, const 0.2) yields a complexity surface per folder that separates dense code from boilerplate. `cx_per_node` ranges 1.89 (alert-platform — const/file heavy) to 2.45 (dbd — struct/type heavy); it is the ratio, not the raw count, that captures "this artifact was hard."

  | folder | raw nodes | cx_score | cx / node |
  |---|---|---|---|
  | sensei | 11,867 | 28,079 | 2.37 |
  | rokkit | 6,484 | 14,347 | 2.21 |
  | alert-platform | 4,465 | 8,428 | 1.89 |
  | dbd | 1,586 | 3,893 | 2.45 |
  | gateway | 1,452 | 3,439 | 2.37 |
  | kavach | 1,364 | 2,663 | 1.95 |

  ```sql
  WITH w(kind,wt) AS (VALUES ('component',5.0),('hook',4.0),('interface',4.0),('class',4.0),
     ('struct',4.0),('enum',3.0),('type',3.0),('function',3.0),('extension',3.0),('method',2.0),
     ('module',2.0),('file',1.0),('doc',1.0),('const',0.2))
  SELECT n.folder_id, count(*) raw_nodes, round(sum(COALESCE(w.wt,1.0))) cx_score
  FROM sensei.nodes n LEFT JOIN w ON w.kind=n.kind::text GROUP BY n.folder_id;
  ```

- **The graph's kind mix explains why LOC and node-count are noise.** `const` is 63% of all 476,978 nodes (302,290) and `file`+`method` another 20%. A raw node/LOC count is dominated by declarations and generated constants; the semantically expensive kinds (`component` 1,240, `hook` 2,399, `interface` 3,707) are <2% of nodes. Weighting is not a nicety — it is the only way the number means anything.

**Root cause / interpretation.**

sensei was instrumented to answer *"is this session going well?"* (FTR, corrections, drift) and only incidentally *"how much got done?"* The volume columns — `turns`, `tool_calls`, `Edit`/`Write` events — are byproducts of the hook stream (`activity.assistant_events` → `activity.turns`), not deliberate output accounting. Because a struggling agent reads, edits, reverts, and re-edits the same files, **volume and struggle are positively correlated**: the 76% concentration of tool-calls in corrected sessions is not an anomaly, it is the mechanical signature of rework. This is exactly why the user's warning against LOC lands — LOC, edits, tool-calls, and wall-clock are all monotone in *toil*, and toil is what FTR is trying to suppress. Shipping the same metric that rewards toil would fight the product's own north star.

The second gap is structural: **there is no recorded unit between a session and a project.** The `runs`/`run_events` tables *do* model features and phases (`feature_started`/`feature_done`, `plan_graph.phases`), and that model is correct — but it only fires inside the 9 autonomous runs. The 69 interactive sessions have a free-text `task` and a `summary` and nothing else. So the denominator the user wants ("N features") exists in schema but is populated for ~1% of activity. Meanwhile the numerator's complexity term ("× complexity") is stranded because the two obvious sources are dead: `nodes.degree` is unpopulated and `nodes.modified_at` is an index clock, so we can neither read a node's graph-centrality nor diff the graph across a session boundary.

Third, **cost is invisible.** With `tokens_in/out` NULL everywhere and no `usage` in any payload, the daemon cannot compute cost-per-feature, tokens-per-edit, or an efficiency frontier — the very metrics that would let a lead say "this epic cost 2.3M tokens for 4 features." The `model`/`provider` NULLs compound it: even if tokens arrived, we could not attribute spend to a model family. The hook payload clearly *can* carry `model` and `duration_ms` (98 events prove it), so this is a capture gap in the ingest path, not a missing data source.

Finally, the **project layer is inert as a productivity substrate.** 128 projects all at `discovery` with empty `backlog` and null `goal` means there is no target to measure progress against and no maturity progression to track. Velocity without a backlog is speed without a destination — you can report throughput but never "% delivered." The pieces for the user's formula exist across four tables; none of them are wired to sessions, and the project spine that should aggregate them is blank.

**Recommendations.**

1. **(P0) Capture a per-session graph delta.** On session close, snapshot `(folder_id, nodes_added, nodes_modified, edges_added, cx_delta)` into a new `activity.session_graph_delta` table, computed by the indexer against the session's touched files (the file list is derivable from `Edit`/`Write` payloads in `assistant_events`). This is the single missing join that makes complexity-weighted velocity computable. Expected effect: turns "9,586 edits" into "sensei-repo gained 412 weighted-complexity points across 40 completed sessions."

2. **(P0) Backfill token/model capture in the hook ingest.** The Claude hook stream exposes usage on `Stop`/`SubagentStop`; populate `activity.sessions.tokens_in/out/model/provider` from it (module: `senseid` hook handler that writes `assistant_events`). Without this, cost-per-feature and efficiency metrics are unbuildable. Expected effect: unlocks tokens-per-completed-feature and a spend-vs-FTR frontier.

3. **(P0) Report velocity outcome-adjusted, never raw.** Any productivity surface (app "Insights"/velocity screen) must exclude or discount rework: define **Effective Velocity = Σ cx_delta over `completed` sessions / active-time**, and show **Rework Ratio** (corrected volume ÷ total volume, currently 0.76) beside it. Building the raw version would actively mislead — the top-10 leaderboard proves it inverts.

4. **(P1) Promote the `run` feature/phase model to interactive sessions.** Let a session optionally bind to a `run`/feature so `feature_done` fires for interactive work too — or derive a lightweight WorkItem from `TaskCreate`/`TaskUpdate` MCP events (459 `TaskCreate`, 876 `TaskUpdate` already flow through `assistant_events`). Expected effect: a real "N features" numerator for the 69 sessions that today have none.

5. **(P1) Populate `nodes.degree` in the community/edge pass.** Degree is already the natural per-node complexity weight and the column exists; the resolve-edges task (166k executions) has the adjacency to fill it. Expected effect: replaces the hand-tuned kind-weights table with measured centrality, and feeds `cx_delta`.

6. **(P1) Give projects a backlog + maturity progression.** Populate `projects.backlog`/`goal` from `sensei:plan` output and let maturity advance past `discovery`. Expected effect: enables **% of backlog delivered** and per-project velocity roll-ups; today all 128 are un-measurable.

7. **(P2) Separate active-time from wall-clock.** Derive `active_seconds` from gaps between consecutive turn timestamps (cap idle at e.g. 5 min) so velocity denominators stop counting the 16 h of idle in corrected sessions. Expected effect: the throughput denominator becomes labor, not calendar.

**Proposed metrics & instrumentation.**

| Metric | Definition / formula | Source (table.column) | Cadence | Current gap |
|---|---|---|---|---|
| Complexity-weighted velocity | Σ `cx_delta` over `completed` sessions ÷ active-hours | `activity.session_graph_delta.cx_delta` (new) ÷ derived active-time | daily / project | delta table not built; degree NULL |
| Rework ratio | corrected-session volume ÷ total volume (tool_calls) | `activity.sessions.outcome` × `activity.turns.tool_calls` | session / daily | **computable now = 0.76** |
| Graph delta per session | nodes_added + edges_added, kind-weighted | `sensei.nodes`/`edges` diffed at session boundary | session | `modified_at` is index-time, not authoring; no snapshot |
| Feature completion rate | `feature_done` ÷ features planned | `activity.run_events.kind`, `runs.plan_graph.phases` | run / project | only 6 `feature_done` exist; sessions carry none |
| Cost per feature | tokens_out ÷ features completed | `activity.sessions.tokens_out` (NULL) ÷ feature count | run / project | tokens NULL for all 69; no `usage` in 131,692 payloads |
| Tokens per weighted edit | tokens_out ÷ cx_delta | `sessions.tokens_out` ÷ `session_graph_delta.cx_delta` | session | both inputs missing |
| Effective throughput | `completed` sessions ÷ active-hours per day | `activity.sessions.outcome`, `started_at` | daily | wall-clock only; no active-time |
| Backlog burn-down | Δ open backlog items over week | `sensei.projects.backlog` (jsonb len) | weekly / project | backlog empty for all 128 projects |
| Artifact-mix balance | weighted node share by kind (db/api/ui/pure-fn) | `sensei.nodes.kind` × weight table | project | folder `role` set on only 124/6,426 folders |

---
<sup>1</sup> `SELECT count(*) FROM activity.sessions;` → 69. Edit/Write: `SELECT tool_name,count(*) FROM activity.assistant_events WHERE event_type='PostToolUse' AND tool_name IN ('Edit','Write') GROUP BY 1;` → Edit 9,586, Write 2,282.
<sup>2</sup> `SELECT s.outcome,s.ftr,sum(t.tool_calls) tc FROM activity.sessions s JOIN activity.turns t ON t.session_id=s.id GROUP BY s.id,s.outcome,s.ftr ORDER BY tc DESC LIMIT 12;` → top three 8,284/6,794/6,566 all corrected/ftr=f.
<sup>3</sup> `SELECT outcome, round(avg(EXTRACT(epoch FROM duration)/60.0),1) FROM activity.sessions WHERE duration IS NOT NULL GROUP BY outcome;` → completed 178.6, corrected 977.1.
<sup>4</sup> `SELECT count(tokens_in),count(tokens_out) FROM activity.sessions;` → 0,0. `SELECT count(*) FROM activity.assistant_events WHERE payload ?| array['usage','tokens','input_tokens','total_tokens'];` → 0. `SELECT payload->>'model',count(*) FROM activity.assistant_events WHERE payload ? 'model' GROUP BY 1;` → claude-opus-4-8[1m]: 98.
<sup>5</sup> `SELECT count(degree),count(*) FROM sensei.nodes;` → 0 / 476,978. `SELECT date_trunc('day',modified_at)::date,count(*) FROM sensei.nodes GROUP BY 1 ORDER BY 1 DESC;` → 2026-08-04: 7,150.
<sup>6</sup> `SELECT maturity,count(*),max(jsonb_array_length(backlog)),count(goal) FROM sensei.projects GROUP BY 1;` → discovery 128, max_backlog 0, goals 0.
