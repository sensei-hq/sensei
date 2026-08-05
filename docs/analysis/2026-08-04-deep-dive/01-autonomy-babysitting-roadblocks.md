## Autonomy: Babysitting, Bounded Recovery, and the Resume-on-Limit Gap
_Why long autonomous runs halt every few minutes, and why they never come back after a usage-limit reset._

**User's observation.** From `docs/analysis/2026-08-04-metrics.md`: *Babysitting* — "long runs stop every few min asking 'should I continue?'"; *Roadblocks* — "execution doesn't resume when a usage limit resets." The premise: an agent left to run an epic unattended should make forward progress on its own, and should survive an interruption (rate limit, stall) without a human re-launching it. Today it does neither reliably.

**What the data shows.**

- **Autonomous runs mostly do not finish, and the ones that fail all die the same way.** Of 9 runs in `activity.runs`, 5 are `done`, 3 `crashed`, 1 `failed`. **All 4 non-`done` runs hit `recovery_attempts = 3`** — the exact recovery cap — and so did one `done` run (5 runs total pinned at the cap).
  ```sql
  SELECT status, count(*), round(avg(recovery_attempts),2) avg_recov, max(recovery_attempts) FROM activity.runs GROUP BY status;
  SELECT count(*) FILTER (WHERE recovery_attempts=3) hit_cap FROM activity.runs;  -- 5
  ```

- **A "crash" is not a code crash — it is the watchdog giving up.** Every terminal failure event carries `{"note":"exhausted bounded recovery","attempts":3}`. The preceding events are all `{"note":"no progress in a while — nudge to continue"}` (or `heartbeat stale; watchdog marked stalled`). The agent stops *making progress*; the daemon watchdog nudges it; after 3 nudges it marks the run dead.
  ```sql
  SELECT detail FROM activity.run_events WHERE kind='crashed';
  ```

- **The stall/recover cadence is ~20 minutes.** For the failed run `9e6880b5`, the watchdog fired on a fixed clock: `stalled 16:05 → recovered 16:26 → stalled 16:27 → recovered 16:47 → stalled 16:48 → recovered 17:08 → stalled 17:09 → crashed 17:29`. Roughly 20 min of no-progress trips a stall; the auto-recover nudge lands ~1 min later; 4 stalls (~80 min) exhausts the budget. Across all 43 stall/recover/crash transitions the mean gap is 11.3 min (the ~1-min recover flips pull the average down).
  ```sql
  WITH ev AS (SELECT run_id, created_at, lag(created_at) OVER (PARTITION BY run_id ORDER BY created_at) prev
              FROM activity.run_events WHERE kind IN ('stalled','recovered','crashed'))
  SELECT round(avg(EXTRACT(EPOCH FROM (created_at-prev))/60),1) FROM ev WHERE prev IS NOT NULL;  -- 11.3
  ```

  | run | status | stalled | recovered | recovery_attempts | terminal note |
  |-----|--------|--------:|----------:|------------------:|---------------|
  | 5a175bdb | crashed | 6 | 5 | 3 | exhausted bounded recovery |
  | 2bb03bd7 | crashed | 4 | 3 | 3 | exhausted bounded recovery |
  | 8b0321b6 | crashed | 4 | 3 | 3 | exhausted bounded recovery |
  | 9e6880b5 | failed  | 4 | 3 | 3 | exhausted bounded recovery → report_run_outcome |
  | 8a06a179 | done    | 4 | 3 | 3 | survived the cap |

  ```sql
  SELECT substring(run_id::text,1,8), count(*) FILTER (WHERE kind='stalled') stalled,
         count(*) FILTER (WHERE kind='recovered') recovered FROM activity.run_events GROUP BY 1;
  ```

- **"Recovery" never delivers a real resume — it re-labels and re-arms.** The `recovered` events are `{"auto":true,"attempt":N}` with no phase advance; the only recoveries that carried a phase change (`{"via":"update_phase","revived":true}`) came from a human calling `update_phase`, not the auto-loop. Auto-recovery is a heartbeat re-stamp, not work.
  ```sql
  SELECT detail FROM activity.run_events WHERE kind='recovered' ORDER BY created_at LIMIT 6;
  ```

- **The daemon's own limit-reset resume path has been exercised exactly once — as a smoke test.** `run_event_kind` includes `paused_on_limit`, and `runs` has `paused_until` / `pause_reason`. There is **one** `paused_on_limit` event in 800 run-events, and its payload is `{"via":"pause_run","until":"2026-08-02T20:00:00Z","reason":"_verify_ pause smoke test"}`. No real run has ever paused on a limit and resumed.
  ```sql
  SELECT detail FROM activity.run_events WHERE kind='paused_on_limit';  -- 1 row, "_verify_ pause smoke test"
  ```

- **The resume-on-limit behavior that *is* used lives outside the daemon entirely — a hand-rolled `/loop` + `ScheduleWakeup`.** `ScheduleWakeup` fired 256 times but is confined to **2 sessions**; the 128 delay-carrying calls average a **1186 s (~20 min)** delay (range 180–1700 s). **59 of 128** wake prompts explicitly encode limit-survival intent, e.g. `/loop continue executing docs/llm-spec/EXECUTION-PLAN.md — advance the next queued slot through its gated loop, persisting state so it survives usage limits` with `reason: "Fallback heartbeat while the C4 agent builds…"`. This is a user working *around* the missing daemon feature, not the feature.
  ```sql
  SELECT count(*) wakes, count(DISTINCT payload->>'session_id') sessions,
         round(avg((payload->'tool_input'->>'delaySeconds')::int)) avg_s
  FROM activity.assistant_events WHERE tool_name='ScheduleWakeup' AND payload->'tool_input'->>'delaySeconds' IS NOT NULL;
  -- 256 wakes | 2 sessions | 1186 s
  SELECT count(*) FILTER (WHERE payload->'tool_input'->>'prompt' ~* 'survive.*limit|usage limit|limit.*reset') FROM activity.assistant_events
  WHERE tool_name='ScheduleWakeup' AND event_type='PreToolUse';  -- 59 of 128
  ```

- **The interactive fleet babysits at a rate of ~1 halt per user turn.** Aggregate `Stop / UserPromptSubmit = 1701 / 1780 = 0.96`. Notifications confirm the human-in-the-loop tax: **1133 "Claude is waiting for your input"** and **276 "Claude needs your permission"** pings.
  ```sql
  SELECT sum(stop)::numeric/NULLIF(sum(ups),0) FROM
    (SELECT count(*) FILTER (WHERE event_type='Stop') stop, count(*) FILTER (WHERE event_type='UserPromptSubmit') ups
     FROM activity.assistant_events GROUP BY session_id) s;  -- 0.96
  SELECT COALESCE(payload->>'message','other'), count(*) FROM activity.assistant_events
  WHERE event_type='Notification' GROUP BY 1 ORDER BY 2 DESC;  -- 1133 waiting / 276 permission
  ```

- **Halts concentrate in a heavy tail.** 11 sessions carry 50+ Stops (1132 of 1701 total = 67%); 16 sessions have zero.

  | Stops / session | sessions | total Stops |
  |-----------------|---------:|------------:|
  | 0     | 16 | 0 |
  | 1–10  | 29 | 147 |
  | 11–50 | 20 | 423 |
  | 50+   | 11 | 1132 |

  ```sql
  SELECT CASE WHEN stops=0 THEN '0' WHEN stops<=10 THEN '1-10' WHEN stops<=50 THEN '11-50' ELSE '50+' END b,
         count(*), sum(stops) FROM (SELECT count(*) FILTER (WHERE event_type='Stop') stops
         FROM activity.assistant_events GROUP BY session_id) s GROUP BY 1;
  ```

- **Length tracks rework, not completion — long, heavily-babysat sessions are the ones that *don't* go first-time-right.** `corrected` sessions average **69.9 turns / 65.7 Stops**; `completed` sessions average **10.5 turns / 11 Stops**; `abandoned` sessions are short (0.8 turns, 0 Stops — killed early, not stalled late). The max-length session ran 255 turns and landed `corrected`.

  | outcome | n | avg turns | avg Stops | avg corrections |
  |---------|--:|----------:|----------:|----------------:|
  | corrected | 19 | 69.9 | 65.7 | 2.00 |
  | completed | 42 | 10.5 | 11.0 | 0.00 |
  | abandoned | 4 | 0.8 | 0.0 | 0.00 |

  ```sql
  SELECT s.outcome, count(*), round(avg(s.turns),1), round(avg(a.stops),1)
  FROM activity.sessions s JOIN (SELECT session_id, count(*) FILTER (WHERE event_type='Stop') stops
       FROM activity.assistant_events GROUP BY session_id) a ON a.session_id=s.client_session_id
  GROUP BY s.outcome;
  ```

- **The transcript corpus confirms the qualitative pattern.** In 1517 assistant turns: 14 contain a rhetorical continue-prompt (`should i continue|want me to continue|continue?`), 77 ask permission (`would you like me to|shall i proceed`), 23 mention a rate/usage limit, 46 mention pausing. A concrete roadblock example: *"Both research agents hit the session limit before producing their reports … No usable research came back, so I'll write the two tech…"* — work lost to a limit with no resume.
  ```sql
  SELECT count(*) FILTER (WHERE lower(assistant_text) ~ 'should i continue|want me to continue|continue\?') should_continue,
         count(*) FILTER (WHERE lower(assistant_text) ~ 'usage limit|rate limit|limit reset') limit_talk
  FROM activity.transcript_turns;  -- 14 / 23
  ```

- **The runs feature is barely load-bearing yet.** The 5 `plan_graph`s that exist are tiny (1–3 phases, 2–4 tasks, 210–1200 bytes) and 3 of 5 `done` goals literally begin with "verify"/"_verify_". Real epics (relay-engine, the EXECUTION-PLAN loop) run in the **session/`ScheduleWakeup`** layer, disconnected from `activity.runs`. The two autonomy systems do not share state.
  ```sql
  SELECT substring(id::text,1,8), status, jsonb_array_length(plan_graph->'phases') phases, length(plan_graph::text) bytes
  FROM activity.runs WHERE plan_graph IS NOT NULL;
  ```

**Root cause / interpretation.**

There are **two independent autonomy runtimes and neither is complete.** The daemon-native one (`activity.runs` + `run_events` + the watchdog) has a real pause/resume vocabulary — `paused_until`, `pause_reason`, the `paused_on_limit` event, the `pause_run` MCP tool — but that vocabulary has only ever been fired by a smoke test. What actually runs in that lane is a *watchdog with a nudge budget*: on a ~20-minute no-progress timer it emits `stalled`, auto-`recovered` (a bare heartbeat re-stamp, `{"auto":true}`), and after 3 rounds emits `crashed`/`exhausted bounded recovery`. Because "recovery" carries no plan advance and no limit-aware wait, a run that stalls for any durable reason (a usage limit, a blocking question, a genuinely hard step) burns its 3 attempts in ~80 minutes and is marked dead — indistinguishable from a real crash. The `recovery_attempts=3` signature on 100% of non-`done` runs is the tell: these are budget exhaustions, not exceptions.

The second runtime is the one the user actually trusts for long work: a hand-authored `/loop … persisting state so it survives usage limits` re-armed via `ScheduleWakeup` at ~20-minute delays. It works well enough that 59 of 128 wake calls state limit-survival as their explicit purpose — but it lives entirely in the assistant-hook layer, is confined to 2 sessions, and writes nothing to `runs`. So the daemon has no idea these long runs exist, can't show them in the Dōjō inbox, can't watchdog them, and can't resume them if the machine or the `ScheduleWakeup` chain drops. The feature that survives limits and the feature that tracks runs are different features.

The **babysitting** is partly a UX/system-prompt artifact and partly honest gating. Stop/UserPromptSubmit ≈ 0.96 means the agent returns control to the human about once per prompt, and 1133 "waiting for your input" notifications dwarf the 276 genuine "needs your permission" gates — most halts are *rhetorical* ("should I continue?", "would you like me to…?"), not blocked on a real decision or a real permission. The correlation is the important part: Stops scale linearly with turns, and both scale with the `corrected` outcome (65.7 Stops / 69.9 turns) versus `completed` (11 / 10.5). Halting more does not buy correctness — the sessions that halt most are the ones that end up reworked. That points at the halts being a symptom of drift (the agent loses the thread and checks in) rather than a safety mechanism that's earning its cost.

**Recommendations.**

1. **(P0) Wire limit-reset resume into the daemon, and route the `/loop` pattern through it.** When a run hits a usage/rate limit, set `runs.status='paused'`, `paused_until = reset_time`, `pause_reason='usage_limit'`, emit `paused_on_limit`, and register a daemon-side wake at `paused_until` (the same intent `ScheduleWakeup.delaySeconds` already encodes). On wake, resume from `current_phase`/`current_feature` in `plan_graph` rather than restarting. This collapses the two runtimes into one and makes "survives usage limits" a daemon guarantee, not a user script. *Effect:* the 4/9 limit/stall deaths become pauses that self-resume; the 2-session `ScheduleWakeup` workaround becomes unnecessary.

2. **(P0) Distinguish "stalled-on-limit" from "stalled-on-nothing" before spending the recovery budget.** The watchdog currently treats every stale heartbeat identically. Before incrementing `recovery_attempts`, classify the stall: if a limit/permission/question is outstanding, `pause` (unbounded, resume on the trigger) instead of counting it against the 3-nudge cap. *Where:* the watchdog that writes `stalled`/`recovered` in the run engine. *Effect:* removes the "exhausted bounded recovery" false-crash — no run should die because it waited out a 5-hour limit reset.

3. **(P1) Add an autonomy budget with visible ledger instead of a silent 3-nudge cap.** Replace `recovery_attempts=3 → crashed` with an explicit per-run budget (turns, tokens, wall-clock, nudges) surfaced in the Dōjō run view and in `run_events`. When the budget is nearly spent, emit a single decision event rather than dying. *Where:* `activity.runs` (add budget columns) + run detail screen. *Effect:* the operator sees *why* a run is about to stop and can extend it, instead of finding a `crashed` corpse.

4. **(P1) Suppress rhetorical continue-prompts in autonomous mode.** Gate the "should I continue? / would you like me to…?" family behind an autonomy flag: in unattended runs, proceed and log the decision to `run_events` instead of halting. Reserve `Stop`/`Notification` for genuine permission gates (the 276 "needs your permission", not the 1133 "waiting for input"). *Where:* system prompt + the hook that emits `Stop`. *Effect:* drives Stop/UserPromptSubmit down from 0.96 toward the interruption target; recovers the ~1132 Stops in the 50+ tail.

5. **(P2) Run watchdog → operator handoff, not silent death.** When a run genuinely exhausts its budget or the agent asks a real question, transition to `blocked` (the enum already exists in `runs_active_idx`) and raise it in the inbox, rather than `crashed`. Pair with a "resume" affordance that re-enters at `current_phase`. *Effect:* closes the "execution doesn't resume" roadblock for the non-limit cases too.

**Proposed metrics & instrumentation.**

| Metric | Definition / formula | Source (table.column) | Cadence | Current gap |
|--------|----------------------|-----------------------|---------|-------------|
| Run completion rate | `done / all` runs, split real-vs-verify by goal prefix | `activity.runs.status` | per-run, weekly | verify runs inflate `done`; no `is_smoke` flag |
| Bounded-recovery death rate | share of terminal runs with `recovery_attempts = cap` | `activity.runs.recovery_attempts`, `.status` | per-run | 100% today — cap can't distinguish limit-wait from true crash |
| Mean-turns-to-stall | turns between run start and first `stalled` event | `activity.run_events.kind='stalled'`, `.created_at` | per-run | derivable but not surfaced; runs lack a turn counter |
| Interruption rate (Stops/turn) | `Stop / UserPromptSubmit` per session | `activity.assistant_events.event_type` | per-session, daily | 0.96 today; no autonomous-mode split to compare against |
| Rhetorical-halt ratio | `Notification("waiting for input") / Notification(total)` | `activity.assistant_events.payload->>'message'` | daily | 1133/1424 = 80% flagged as non-permission halts, unmeasured |
| Unattended-autonomy ratio | wall-clock under an active run/loop ÷ total agent wall-clock | `activity.runs.started_at..completed_at` + `ScheduleWakeup` chains | daily/project | `ScheduleWakeup` work invisible to `runs`; can't join today |
| Resume success rate | resumed-after-pause runs that reach `done` ÷ runs that paused | `activity.runs` (`paused_until`) + `run_events` `paused_on_limit`/`done` | per-run | denominator = 1 (smoke test); path not exercised |
| Limit-loss incidents | transcript turns reporting work lost to a limit with no resume | `activity.transcript_turns` (regex on `assistant_text`) | weekly | 23 limit mentions today; not counted or alerted |
