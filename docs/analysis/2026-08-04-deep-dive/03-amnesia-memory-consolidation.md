## Amnesia: The Durable-Memory Consolidation Gap
_Sensei observes a great deal and remembers almost none of it: 943 patterns, 1,478 recommendations and 1,947 drift items funnel into just 11 durable memories — 8 of which have never once been recalled._

**User's observation.** From `docs/analysis/2026-08-04-metrics.md` (line 9):
> "Amnesia or forgetfulness. An example is tooling/harness set up for E2E testing using tauri + playwright, however planner reporting one 'honest' caveat that it cannot drive e2e tests on tauri app. And this is not the first time either, same issue surfacing, without even consulting what is available and this information is being lost in durable context."

This section proves the observation is not anecdotal. There is a working Tauri+Playwright harness, it has been exercised 1,003 times, and the agent still declares it "can't visually verify" — because the fact never made it into durable memory, and even the memories that exist are almost never recalled.

---

### What the data shows

**1. The consolidation funnel collapses by three orders of magnitude.** Every inference table is large; the durable store is tiny. Corrections — the one artifact meant to become memory — number exactly **one**.

| Table | Rows |
|---|---:|
| `inference.drift_items` | 1,947 |
| `inference.recommendations` | 1,478 |
| `inference.detected_patterns` | 943 |
| `inference.reasoning_traces` | 88 |
| `history.past_memories` | 43 |
| `activity.memory_loads` | 24 |
| **`sensei.memories`** | **11** |
| `inference.corrections` | **1** |

```sql
SELECT 'memories' t, count(*) n FROM sensei.memories
UNION ALL SELECT 'detected_patterns', count(*) FROM inference.detected_patterns
UNION ALL SELECT 'recommendations', count(*) FROM inference.recommendations
UNION ALL SELECT 'drift_items', count(*) FROM inference.drift_items
UNION ALL SELECT 'corrections', count(*) FROM inference.corrections
UNION ALL SELECT 'reasoning_traces', count(*) FROM inference.reasoning_traces
ORDER BY n DESC;
```

**2. The 11 memories are all one type, all unslotted, and mostly raw user utterances.** `sensei.memory_type` has 6 values (`decision, pattern, convention, preference, continuity, question`) — **only `convention` is used**. `spine_slot` is NULL for all 11, so nothing is pinned into the durable-context "spine." Two are authored/mandatory principles ("Done means verified against live data," "Verify the outcome, never a masked wrapper"); the other 9 are learned/project/recommended and read like pasted chat lines ("Hey hey, why are you using regex? we moved away from that…", "See nigel only comes into the story way after the encounter with death…").

```sql
SELECT type, count(*) FROM sensei.memories GROUP BY 1;              -- convention | 11
SELECT spine_slot, count(*) FROM sensei.memories GROUP BY 1;         -- NULL | 11
SELECT origin, enforcement, scope, count(*) FROM sensei.memories
GROUP BY 1,2,3;                                    -- learned/recommended/project 9; authored/mandatory/global 2
```

**3. Memory creation stopped six weeks ago.** 9 of 11 memories were created 2026-06-24/25; the last 2 (the authored principles) on 2026-07-31. Across the entire 68-session analysis window (2026-07-06 … 2026-08-04) of heavy activity, **zero project memories were learned.**

```sql
SELECT created_at::date d, count(*) FROM sensei.memories GROUP BY 1 ORDER BY 1;
-- 2026-06-24 | 1 ; 2026-06-25 | 8 ; 2026-07-31 | 2
```

**4. 8 of 11 memories have never been recalled; only 3 ever loaded.** Every load routes through `get_layered_context`. The single most-loaded memory (the keychain convention, 14 loads) went cold after 2026-08-02. The two mandatory principles account for 10 of the 24 loads.

| Memory (title, truncated) | Enforcement | Loads | Last load |
|---|---|---:|---|
| I like the way you added the keychain key store… | recommended | 14 | 2026-08-02 |
| Done means verified against live data, not the code path | mandatory | 5 | 2026-08-04 |
| Verify the outcome, never a masked wrapper | mandatory | 5 | 2026-08-04 |
| _(other 8 memories)_ | recommended | **0** | — |

```sql
SELECT count(*) FILTER (WHERE loads=0) never_loaded,
       count(*) FILTER (WHERE loads>0) ever_loaded
FROM (SELECT m.id, count(ml.id) loads FROM sensei.memories m
      LEFT JOIN activity.memory_loads ml ON ml.memory_id=m.id GROUP BY m.id) s;
-- never_loaded | 8 ; ever_loaded | 3
```

**5. The write-side of the memory pipeline is nearly silent.** Across 131,692 assistant events, the tools that create durable memory fired a combined **8 times**: `save_memory` 5, `propose_memory` 2, `promote_memory` 1. Recall tools are also rare relative to 68 sessions: `get_layered_context` 19, `get_patterns` 24, `get_rules` 27, `context_pack` 1.

```sql
SELECT tool_name, count(*) calls FROM activity.assistant_events
WHERE event_type='PreToolUse'
  AND tool_name ~ 'save_memory|propose_memory|promote_memory|layered_context|context_pack|get_patterns|get_rules'
GROUP BY 1 ORDER BY 2 DESC;
```

**6. 943 patterns are frozen at `lifecycle='suggested'`; none are promoted, enforced, or graded.** `enforcement`, `severity` and `confidence` are NULL for all 943. The patterns literally named `correction-prone` recur up to **9 times** each — textbook repeat-mistakes — yet not one has been consolidated. There is even a `fix_pattern_id` column (a promotion edge) that sits unused.

| Pattern name | instance_count |
|---|---:|
| correction-prone | 9 |
| correction-prone | 9 |
| correction-prone | 7 |
| correction-prone | 6 |
| correction-prone | 5 |
| rule-candidates | 4 |

```sql
SELECT lifecycle, count(*) FROM inference.detected_patterns GROUP BY 1;   -- suggested | 943
SELECT name, instance_count FROM inference.detected_patterns
WHERE instance_count>=3 ORDER BY instance_count DESC LIMIT 6;
```

**7. The one `corrections` row is noise, not a lesson.** Its `signature` is `corr-ce7ffc353d20819b`, `count=20`, spanning 6 projects — but the `text` is a mangled tool-notification dump ("task notification task id … tool use id toolu_01… output file private tmp claude…"), `suggestion` is NULL, and `memory_id` is NULL (never promoted). The correction extractor is aggregating telemetry chrome instead of real user corrections.

```sql
SELECT signature, left(text,80) text, suggestion, count,
       array_length(project_ids,1) nproj, memory_id FROM inference.corrections;
```

**8. 1,477 of 1,478 recommendations are `pending`; exactly one was ever acted on.** The recommendation stream never closes the loop back into memory. Worse, it re-emits the same repeat-mistake per folder: **91** recommendations titled `High rework: …/crates/`, 44 for `strategos/monorepo/docs/`, 42 for `marketplace/`. This is the "rework" metric the user asked for — already computed 1,478 times and thrown away.

```sql
SELECT status, count(*) FROM inference.recommendations GROUP BY 1;   -- pending 1477; accepted 1
SELECT left(title,44) title, count(*) n FROM inference.recommendations
GROUP BY 1 HAVING count(*)>3 ORDER BY 2 DESC LIMIT 5;
```

**9. The Tauri/E2E amnesia is real, repeated, and self-admitted.** The agent says it "can't visually verify" **9 times**, while browser/Playwright tools were actually invoked **1,003 times across 18 sessions**. In session `cc24f9a7` (turn 9) the agent finally corrects itself:
> "you're right — there IS a Playwright + Tauri e2e harness, and I've wrongly claimed 'can't visually verify' repeatedly. That's a real error on my part, not a one-off. … the harness is real and works — `app/e2e/` (Playwright + Tauri, `--project=tauri`), run via `make test-app-e2e`."

The harness is verifiable on disk (`app/e2e/`, `node_modules/@srsholmes/tauri-playwright`) and in the registries (`sensei.assistant_tools` has 24 Playwright/browser tools; `project_commands` has `test:e2e`, `test:e2e:cold`). The capability was never absent — it was un-recalled.

```sql
SELECT count(*) FILTER (WHERE assistant_text ILIKE '%can''t visually%'
                         OR assistant_text ILIKE '%cannot visually%') cant_visual
FROM activity.transcript_turns;                                             -- 9
SELECT count(*) browser_calls, count(DISTINCT session_id) sessions
FROM activity.assistant_events
WHERE event_type='PreToolUse' AND tool_name ILIKE '%browser%';              -- 1003 | 18
```

**10. Reasoning reaches consensus but proposes no durable action.** All 88 `reasoning_traces` carry a `consensus`; **0** carry an `action_proposed`. Multi-model deliberation happens and then evaporates.

```sql
SELECT count(*) traces,
       count(*) FILTER (WHERE action_proposed IS NOT NULL
                          AND action_proposed::text<>'null') w_action
FROM inference.reasoning_traces;                                            -- 88 | 0
```

**11. Even recalled guidance is mostly ignored.** `sensei.tool_call_verdicts`: **70.0%** of 23,638 classified tool calls are `ignored` (16,547), 29.8% `used`, 0.2% `partial`. So on the rare occasions context is surfaced, the majority is discarded — amnesia downstream of recall, not just upstream of it.

```sql
SELECT verdict, count(*), round(100.0*count(*)/sum(count(*)) over(),1) pct
FROM sensei.tool_call_verdicts GROUP BY 1 ORDER BY 2 DESC;
```

**12. The durable store also churns.** `history.past_memories` holds 21 distinct `memory_id`s (vs 11 live) with 32 rows closed (`effective_to` set) — roughly 10 memories were created and later superseded/deleted. Small *and* leaky.

```sql
SELECT count(DISTINCT memory_id) past_distinct FROM history.past_memories;   -- 21
SELECT count(*) FILTER (WHERE effective_to IS NOT NULL) closed FROM history.past_memories; -- 32
```

**Surprises.** (a) The `corrections` table's only row is telemetry garbage, not a correction. (b) `memory_loads.session_id` and `client_session_id` are **NULL on all 24 rows** — recall cannot be attributed to a session, so knowledge-retention can't currently be measured end-to-end. (c) The agent *quantifiably used* the Playwright harness 1,003 times while claiming it couldn't.

---

### Root cause / interpretation

**The promotion pipeline is built but never fires.** Sensei has every stage of a consolidation loop — `detected_patterns` → `corrections` → `promote_memory`/`save_memory` → `memories` → `get_layered_context` recall — and every join key to wire it (`corrections.memory_id`, `detected_patterns.fix_pattern_id`, `memory_type` with 6 grades, `spine_slot`). But promotion is **manual and MCP-triggered**, and the agent almost never calls it: 8 write-tool invocations across 131,692 events. Nothing runs on a schedule to sweep the 14 recurring `correction-prone` patterns (instance_count up to 9) or the 91 `High rework: …/crates/` recommendations into a memory. So the daemon accumulates 4,300+ inference artifacts and distills 0 of them. The 11 memories that exist are a June 24–25 seeding batch plus two hand-authored principles — not a living, growing store.

**Recall isn't wired into the moment of claiming.** The Tauri example is the archetype: the model asserts an incapacity ("can't visually verify," "cannot drive e2e on tauri") as a plausible caveat, without first consulting the tool/command registries that would refute it. `get_layered_context` (19 calls) and `context_pack` (1 call) are opt-in tools the model chooses to skip under time pressure, and there is no guard that intercepts a negative capability claim and forces a registry lookup before it is emitted. The capability data exists — 24 Playwright tools in `assistant_tools`, `test:e2e` in `project_commands`, `app/e2e/` on disk — but nothing surfaces it at plan time or gates the "honest caveat." The result is a confabulated limitation: the model invents `p-{cannot}` for its own abilities exactly the way the DRY/anti-fabrication rules forbid for data.

**When recall does happen, its payload is thin and mostly ignored.** Only 3 of 11 memories are ever loaded and 70% of classified tool calls are `ignored`, so even a firing recall path delivers little signal and gets discarded. Two compounding reasons: (1) the memories are low-value — raw utterances, all `convention`, none slotted into the durable spine, so ranking/injection has nothing high-strength to prefer; and (2) `memory_loads` has no session linkage, so the system can't learn which memory helped which outcome and can't strengthen or prune. The historization churn (21 historical vs 11 live) shows the store is edited but not curated toward durability.

**Net:** amnesia is not a model limitation, it's a missing daemon job plus a missing prompt guard. The observations, corrections, and recommendations are all captured; they simply never become durable, never get slotted, and never get consulted at the one moment — emitting a caveat or a plan — where they'd change behavior.

---

### Recommendations

1. **(P0) Memory-consolidation sweep — a scheduled daemon job.** Add a periodic job (senseid, alongside pattern detection) that promotes recurring signals into `sensei.memories`: any `detected_patterns` with `instance_count >= 3` (14 exist today), any `corrections.count >= N`, and any recommendation title repeated >K times (`High rework: …/crates/` × 91). Write with the correct `memory_type` (`pattern`/`decision`, not always `convention`), set `spine_slot`, link `corrections.memory_id`/`detected_patterns.fix_pattern_id`, and flip `lifecycle` off `suggested`. Expected effect: memory count grows from 11 toward the dozens of genuinely-recurring lessons already in the tables; repeat-mistake recommendations stop re-emitting.

2. **(P0) Recall-before-"I can't" guard.** Before the agent emits a negative capability claim (regex over assistant output: `can't|cannot|unable to|no way to` within N tokens of `verify|e2e|test|drive|screenshot|tauri`), require a registry lookup (`get_commands` + a new capability query over `assistant_tools`/`project_commands`) and block the claim if a matching tool/command exists. Wire into the sensei MCP server + system prompt. Expected effect: the 9 "can't visually verify" claims become 0 given 24 Playwright tools and `test:e2e` are registered.

3. **(P0) Fix the corrections extractor.** The single `corrections` row is tool-notification noise. Filter `activity.turns` where `is_correction=TRUE` (38 exist, with `triage_signal` revert/correction/why) instead of scraping raw notification text, dedupe by semantic signature, and auto-`propose_memory` above a count threshold. Expected effect: real corrections (the 18 reverts, 17 corrections) become the seed of durable memory instead of a garbage aggregate.

4. **(P1) Capability/harness registry surfaced at plan time.** Add a `context_pack` section (or a `plan`-tool preamble) that lists available harnesses/tools/commands for the active project — "E2E: `make test-app-e2e` (Playwright+Tauri, 1,003 prior invocations)". Source from `assistant_tools` + `project_commands` + a curated skills index (`tauri-playwright-testing` already exists as a skill). Expected effect: the planner sees capabilities before writing caveats.

5. **(P1) Restore session linkage on `memory_loads`.** Populate `session_id`/`client_session_id` (both NULL on all 24 rows) so recall can be attributed to outcomes. This unlocks recall-hit-rate and knowledge-retention metrics below. Expected effect: the daemon can strengthen memories that precede good outcomes and prune those that don't.

6. **(P1) Close the recommendation loop with acted/measured feedback.** 1,477/1,478 recommendations are `pending`. Surface promoted memories back as recommendations with `acted_at`/`measured_at`/`baseline_ftr`/`current_ftr` populated (the columns already exist), so consolidation is measurable and the ignored-70% verdict rate can be driven down.

7. **(P2) Curate memory quality.** Enforce a promotion bar (min strength, generalised content, non-empty `impact`, assigned `spine_slot`) so recall injects high-signal principles rather than pasted chat lines. Deprioritize `recommended`/never-loaded memories in ranking. Expected effect: the 3-of-11 load rate rises because injected memories are worth loading.

---

### Proposed metrics & instrumentation

| Metric | Definition / formula | Source (table.column) | Cadence | Current gap |
|---|---|---|---|---|
| Memory promotion rate | promoted memories ÷ eligible recurring signals (`detected_patterns.instance_count≥3` + `corrections.count≥N`) | `sensei.memories.origin='learned'`, `inference.detected_patterns.instance_count`, `inference.corrections.count` | daily | No promotion job exists; rate ≈ 0 (0 promoted from 943 patterns) |
| Knowledge-retention rate | fraction of promoted memories loaded ≥1× within T days of creation | `sensei.memories.created_at` ⋈ `activity.memory_loads.loaded_at` | weekly | Only 3/11 ever loaded; `memory_loads.session_id` NULL blocks per-session attribution |
| Recall-hit rate | sessions calling `get_layered_context`/`context_pack` before first Edit ÷ all sessions | `activity.assistant_events.tool_name`, `event_type` | session | 19 layered_context calls across 68 sessions (~28% ceiling) |
| Repeat-mistake rate | distinct `correction-prone`/`High rework` signatures recurring in ≥2 sessions and NOT covered by a memory | `inference.detected_patterns.name/instance_count`, `inference.recommendations.title` | daily | 14 recurring patterns + 91-dup rework recs, 0 covered by memory |
| Confabulated-incapacity count | assistant turns asserting `cannot/can't … verify/e2e/test` where a matching tool/command is registered | `activity.transcript_turns.assistant_text` ⋈ `sensei.assistant_tools`/`project_commands` | session | 9 "can't visually verify" vs 1,003 Playwright invocations |
| Guidance-adoption rate | `used` ÷ (`used`+`ignored`+`partial`) tool-call verdicts | `sensei.tool_call_verdicts.verdict` | daily | 29.8% used / 70.0% ignored today |
| Consolidation ratio | `memories` ÷ (`detected_patterns`+`recommendations`+`corrections`) | row counts across `sensei.memories`, `inference.*` | weekly | 11 ÷ 2,422 ≈ 0.5% |
| Reasoning→action yield | `reasoning_traces` with non-null `action_proposed` ÷ total traces | `inference.reasoning_traces.action_proposed` | weekly | 0 / 88 = 0% |
| Memory durability | live memories ÷ distinct historized `memory_id` (1 − churn) | `sensei.memories`, `history.past_memories.memory_id` | monthly | 11 live / 21 historized ≈ 52% |
