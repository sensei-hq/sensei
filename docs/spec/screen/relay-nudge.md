# 静 · Relay · Nudge

**Segment:** 05 · Relay (mobile companion · planner)
**Route:** Relay iOS app · Nudge (no web route)
**Source mockup:** [`lib/relay/relay-planner.jsx`](../../mockups/Sensei/lib/relay/relay-planner.jsx) → `RelayNudge`
**Data:** _greenfield_ — `GET /api/relay/projects/{name}/stall` (coordinator-published): the stalled track — `quiet_for` duration, `reason` (filtered, e.g. `waiting on API rate limit`), `auto_retry_in`, the phase (`n of x` + `pct`), and a mini `done · doing · next` of where it stands. `POST /api/relay/projects/{name}/nudge` continues the track.
**App file:** _greenfield — mobile app not built_
**Daemon files:** _greenfield — coordinator + planner model not built_
**Status:** greenfield (Relay backend unbuilt) — see [architecture/relay](../../architecture/relay.md)

## Purpose

A track has gone quiet, and this screen tells the user **why, where it
stands, and what they can do** — without them having to dig. Auto mode is
non-blocking, so a stall is the second reason (after a gate) the human gets
pulled in (R3). The banner names the stall (`No activity for 22m · waiting on
API rate limit`) and — crucially — says sensei will **retry on its own**
(`~8m`), so the user knows this is optional. Below it, a compact
`done · doing · next` shows exactly which step paused mid-work. Two actions:
**Nudge to continue** (resume now, don't wait for the auto-retry) or **View
logs**.

Kanji is 静 — *sei / quiet, stillness* — the quiet track (reused from the
mockup's stall banner).

## Data invariants

The stall model this screen renders:

- **A stall is a track quiet past a threshold, not a failure.** The banner
  shows `quiet_for` (`22m`) and a filtered `reason`
  (`waiting on API rate limit`). The track is paused, not errored.
- **Auto mode still owns it.** The coordinator will `auto_retry_in`
  (`~8m`) on its own — the nudge is an *option to resume early*, never the
  only path forward (R2). The copy must say the retry is automatic.
- **Where-it-stands is a mini plan slice.** Three `MiniStep`s —
  `done` / `doing` / `next` — for the stalled phase (R3). The `doing` step
  is the one paused mid-work, marked with a warning note
  (`paused mid-write · migrations/ingest.sql`).
- **Phase context matches the plan.** The sub-hero shows the phase name,
  `Phase {n} of {x}`, and `pct` — the same numbers as the project card and
  the plan screen.
- **Filtered status only.** `reason`, and the paused-step note (a file
  *path*, not its contents), are filtered labels — no code, no diff, no
  transcript crosses the relay (R5).
- **Nudge resumes, logs stay local.** Nudge tells the coordinator to
  continue now; `View logs` opens local logs on the machine (logs are not
  relayed — they never leave the hardware).

## Signals shown

Hero:

| Element | Value | Meaning | Example |
|---|---|---|---|
| Back | ← | return to Projects | — |
| Auto label | `{project} · auto` (mono) | which project, auto mode on | `telemetry · auto` |

Stall banner (warning-tinted):

| Element | Value | Meaning | Example |
|---|---|---|---|
| Mark | 静 (kanji, warning) | the quiet track | 静 |
| Title | `This track has gone quiet` | the stall headline | `This track has gone quiet` |
| Detail | `No activity for {quiet_for} · {reason}` | how long + filtered cause | `No activity for 22m · waiting on API rate limit` |
| Reassurance | auto-retry line | sensei retries on its own; nudge is optional (R2) | `sensei will retry on its own in ~8m. You can nudge it now.` |

Where-it-stands:

| Element | Value | Meaning | Example |
|---|---|---|---|
| Plan name | `plan` (display) | the stalled plan | `Event pipeline rewrite` |
| Phase line | `Phase {n} of {x} · {pct}%` (mono) | phase context | `Phase 1 of 3 · 18%` |
| Section label | `Where it stands` (eyebrow) | the mini done·doing·next | — |
| `done` step | filled check, muted | already finished | `Map current event schema` |
| `doing` step | ring, warning note | the paused step | `Draft ingest schema` — `paused mid-write · migrations/ingest.sql` |
| `next` step | empty ring | not yet started | `Backfill last 30 days` |

Actions:

| Element | Value | Meaning | Example |
|---|---|---|---|
| Primary | `Nudge to continue` (accent) | resume the track now, don't wait for auto-retry | — |
| Secondary | `View logs` (paper) | open local logs (not relayed) | — |

Worked example (the mockup stall): `telemetry · auto` — banner: *This track
has gone quiet · No activity for 22m · waiting on API rate limit* · retry
~8m — *Event pipeline rewrite* · Phase 1 of 3 · 18% — done: Map current
event schema · doing: Draft ingest schema (paused mid-write ·
migrations/ingest.sql) · next: Backfill last 30 days — [Nudge to continue]
[View logs].

## Done gate

- The banner names the stall with `quiet_for` and a filtered `reason`, and
  states the auto-retry (`~8m`) so the user knows nudging is optional (R2).
- Where-it-stands shows a `done · doing · next` slice with the paused step
  as `doing` and a warning note pointing at the paused work (R3).
- The phase name, `Phase {n} of {x}`, and `pct` match the project card and
  the plan screen for the same project.
- `Nudge to continue` resumes the track; `View logs` opens local logs and
  those logs never cross the relay.
- The stall detector fired only after the quiet threshold — a briefly-idle
  track that is about to auto-continue does not raise a nudge.
- `reason` and the paused note are filtered labels (a path, not contents);
  no code crosses the relay (R5).
- Dark mode: the warning-tinted banner and the `doing` warning note stay
  readable; the two actions stay distinguishable.

## Wrong gate

- **A stalled track never nudges.** The track is quiet well past threshold
  but no nudge surfaces — the user only finds out by opening the app and
  noticing the project card's `Paused` state didn't escalate (R3 broken).
- **Stall shown as a hard failure.** The banner reads like an error and
  omits the auto-retry line, so the user thinks they *must* act — but auto
  mode would have recovered on its own (R2).
- **`View logs` relays the log contents.** Logs must stay on the machine;
  surfacing them through the relay breaks zero-knowledge (R5).
- **The paused-step note contains code / a diff** rather than a path label
  — filtered status only (R5).
- **Nudge fires while the coordinator is already auto-retrying**, causing a
  double-run of the same step — the two paths aren't coordinated.
- **A too-eager threshold** flags a normally-slow step (a long build) as a
  stall, training the user to ignore nudges.

## Related

- [[architecture/relay]] — auto mode · the human is pulled in at a gate or a stall
- [[journeys/relay]] — run & supervise · `track stalled → Nudge`
- [[screen/relay-projects]] — a `stall` verdict card leads here
- [[screen/relay-plan]] — the phase whose step paused
- [[screen/relay-decisions]] — the other reason a track pulls the human in
