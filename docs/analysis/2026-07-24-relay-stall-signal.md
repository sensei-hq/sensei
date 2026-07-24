---
title: Relay stall signal — "no update in 5 min" needs progress-staleness, not heartbeat
date: 2026-07-24
status: design note (NOT yet implemented — needs Jerry's sign-off on the "what counts as progress" line)
relates: docs/plan/relay-engine.md · crates/senseid/src/run_watchdog.rs · tasks/handlers/advance_run.rs · tasks/watchdog_scheduler.rs
---

# The stall signal doesn't yet mean what Jerry asked for

**Jerry's spec:** *"no update in 5 minutes indicates a stall."* i.e. the run should flip
to `stalled` (→ surface + nudge) when the **agent stops making progress**, so Jerry can
click *nudge* instead of guessing whether it's stuck.

**What the watchdog actually does today.** Stall = `heartbeat_at` age > `stale_after`
(**20 min**, `run_watchdog.rs:58`). But with `drive` OFF, the `advance_run` handler runs
**every tick** and does two things (`advance_run.rs:14` — "heartbeat + housekeeping only"):

1. bumps `heartbeat_at = now()` (liveness), and
2. appends a `housekeeping` `run_event` (cadence).

So **both** `heartbeat_at` **and** `last_event_at` are refreshed every tick by the daemon
itself — regardless of whether the agent did anything. Consequences:

- The 20-min heartbeat stall only fires when the **daemon stops ticking** (process down /
  wedged) — a **crash-recovery** signal, not an **agent-stuck** signal.
- A run where the agent has done nothing for an hour still shows `running` with a fresh
  heartbeat. The "am I stuck?" question Jerry wanted answered is **not** answered.

## Two distinct concerns (conflated today)

| Concern | Question | Signal | Action |
|---|---|---|---|
| **Daemon liveness** | Is the daemon still ticking this run? | `heartbeat_at` age | crash-recover (existing 3-attempt ladder) |
| **Agent progress** *(Jerry's ask)* | Has the agent advanced in the last 5 min? | age of the last **meaningful** run event | mark `stalled` → surface + nudge |

They need **separate** timers. Keep the heartbeat crash-recovery as-is; **add** a
progress-staleness check.

## Proposed design (progress-staleness)

1. **Classify events.** Add `RunEventKind::is_progress()` — everything the *agent/run*
   does counts; the daemon's per-tick bookkeeping does not. Concretely, **`Housekeeping`
   is the only non-progress kind** today (it's literally the every-tick no-op marker);
   `Resumed`/`Throttled`/`PausedOnLimit`/`Stalled`/`Crashed`/`Recovered` are daemon
   lifecycle and should also be excluded. Progress = phase/feature/gate/commit/push/
   merge/bump/flag/done/failed. **This one line is Jerry's to confirm.**
2. **Query** the age of the newest *progress* event per active run (extend
   `list_recoverable_runs`, or a small `last_progress_at(run_id)`).
3. **Watchdog:** if a `running` run's last progress event is older than
   `progress_stale_after` (**5 min**, tunable/env-overridable), mark it `stalled` with a
   `Stalled` event noting "no progress in 5m" → the existing surface + push + nudge path
   fires. Recovery/crash ladder unchanged.
4. **Root-cause option worth weighing:** the per-tick `Housekeeping` event is *only* there
   as a cadence marker. If nothing consumes it, consider **not** logging it every tick (it
   also inflates `run_events`) — then `last_event_at` alone becomes the progress signal and
   step 1's classification is unnecessary. Check consumers first.

## Why this is feeding-ready now

The [workflow→run phase bridge](../../crates/senseid/src/api/handlers/sessions.rs)
(`update_phase` → `phase_started`/`phase_done` on the active run, shipped `37770d07`) is
exactly the progress signal this would read: while the agent calls `update_phase`, progress
events are fresh; when it goes quiet for 5 min, they go stale → stall → nudge. The input
exists; only the watchdog's *reference timestamp* needs to change.

## Not done because

Redefining the "am I stuck?" signal unattended risks false-stalls (nag) or never-stalls
(silent) — the exact failure Jerry wants to avoid. The **progress/noise line (step 1)** and
the **5-min default** are product calls. Everything else is mechanical + testable
(pure `is_progress()` unit tests + a watchdog tick test with a stubbed clock).
