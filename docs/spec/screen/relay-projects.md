# 場 · Relay · Projects

**Segment:** 05 · Relay (mobile companion · planner)
**Route:** Relay iOS app · Projects (no web route)
**Source mockup:** [`lib/relay/relay-planner.jsx`](../../mockups/Sensei/lib/relay/relay-planner.jsx) → `RelayProjects`
**Data:** _greenfield_ — `GET /api/relay/projects` (coordinator-published, filtered status only): one row per active project, each with its single active plan, phase `n of x`, `pct`, the track running `now`, and a `flag` telling the human whether it needs them.
**App file:** _greenfield — mobile app not built_
**Daemon files:** _greenfield — coordinator + planner model not built_
**Status:** greenfield (Relay backend unbuilt) — see [architecture/relay](../../architecture/relay.md)

## Purpose

The away-from-keyboard home screen. The user opens Relay to answer one
question: **is everything moving, and does anything need me?** Each active
project shows its one active plan, how far through the phases it is, what's
running right now, and a single verdict word (`on track`, `1 decision
waiting`, `quiet for 22m`). Development runs non-blocking in **auto mode**
(the `AwayPill` at the top says so); this screen is a glance, not a control
panel. Attention-needing projects (a **gate** waiting, a **stall**) tint
their card so they read first without sorting.

Kanji is 場 — *ba / the place*, the project as a place where work happens
(reused from the per-project mark in the mockup).

## Data invariants

The planner model this screen reads from:

- **One active plan per project.** A project row carries exactly one
  `plan` name; there is no plan picker here. If a project has no active
  plan it does not appear on this screen.
- **Phases sequence the plan.** `phase` is the current phase index and
  `of` is the phase count — rendered `Phase {phase} of {of}`. `pct` is
  whole-plan progress (0–100), not phase progress.
- **A phase holds features · checkpoints · gates.** This screen never
  shows those; it summarises them into `now` (the track running) and
  `flag`. A **gate** is a step that stops for a human.
- **`flag` is derived, never authored.** One of `gate` (a decision is
  waiting), `doing` (running, nothing needed), `stall` (a track has gone
  quiet). Only `gate` and `stall` are attention states and tint the card.
- **Filtered status only.** `now` is a track label
  (`refresh-token store`), never code, a diff, or a transcript line. The
  relay is zero-knowledge — nothing here would leak the codebase.

## Signals shown

Top of screen:

| Element | Value | Meaning | Example |
|---|---|---|---|
| Title | `Projects` | screen name | `Projects` |
| Away pill | live dot + label | confirms auto mode is running | `Auto mode · working while you're away` |

Per-project card:

| Element | Value | Meaning | Example |
|---|---|---|---|
| Project mark | 場 + `name` (mono) | which project | `場 lumen-auth` |
| Machines | `machines` (mono) | how many machines this run spans (R7) | `2 machines` |
| Plan | `plan` (display) | the one active plan's name | `OAuth & session hardening` |
| Phase count | `Phase {phase} of {of}` | where the plan is in its phases | `Phase 2 of 4` |
| Progress | `pct`% + `PBar` | whole-plan progress bar | `47%` |
| Now / paused | dot + `now` | the track running right now; label is `Now · ` normally, `Paused · ` when stalled | `Now · refresh-token store` |
| Verdict | `note` | one-line human-facing verdict, colour-coded by flag | `1 decision waiting` / `on track` / `quiet for 22m` |

Flag → colour + treatment (from the mockup):

| `flag` | Meaning | Card | Bar / dot colour | Verdict example |
|---|---|---|---|---|
| `gate` | a decision is waiting — **needs you** | tinted `bg-accent-soft`, accent hairline | accent (ink bar) | `1 decision waiting` |
| `doing` | running fine, nothing needed | plain `bg-paper-soft` | success (pulsing dot) | `on track` |
| `stall` | a track has gone quiet — nudge available | tinted `bg-warning-soft`, warning hairline | warning | `quiet for 22m` |

Worked example (the three mockup cards):

- `lumen-auth` · 2 machines · *OAuth & session hardening* · Phase 2 of 4 ·
  47% · Now · refresh-token store · **gate** · `1 decision waiting`
- `billing-svc` · 1 machine · *Invoicing v2* · Phase 3 of 5 · 61% · Now ·
  webhook retry tests · **doing** · `on track`
- `telemetry` · 1 machine · *Event pipeline rewrite* · Phase 1 of 3 · 18% ·
  Paused · ingest schema · **stall** · `quiet for 22m`

## Done gate

- One card per active project from `GET /api/relay/projects`, each with
  exactly one plan name (never a plan list).
- `Phase {phase} of {of}` and `{pct}%` agree with the plan the card links
  to; the progress bar width equals `pct`.
- `gate` and `stall` cards are visually tinted (accent / warning) and read
  before plain `doing` cards without any re-sort.
- A `stall` card reads `Paused · {now}`; `doing`/`gate` cards read
  `Now · {now}` with the live dot pulsing only on `doing`.
- The away pill shows auto mode is running (R2) whenever any track is live.
- `now` is a track label only — never a code fragment, diff, or path (R5).
- Dark mode: both tint fills (accent-soft, warning-soft) keep the verdict
  text readable and the two attention states remain distinguishable.

## Wrong gate

- **A project shows two plans / a plan picker.** The invariant is one
  active plan per project; a picker means the wrong data model leaked in.
- **A `gate` project is not tinted / sorts below `doing`.** A decision is
  waiting (R4) but the human can't see it at a glance — the whole point of
  the screen is lost.
- **`stall` never surfaces.** A track has been quiet (here 22m) but the
  card still reads `Now · …` with a live dot and `on track` — the stall
  detector didn't fire, so no nudge (R3).
- **`pct` is phase progress, not plan progress** — a Phase 1-of-3 card
  reading 90% because phase 1 is nearly done misreads the whole run.
- **`now` contains code or a file diff.** Zero-knowledge is broken (R5) —
  only filtered status may cross the relay.
- **Auto-mode pill absent while tracks run.** The user can't tell whether
  work is actually proceeding without them (R2).

## Related

- [[architecture/relay]] — the planner model + the three planes
- [[journeys/relay]] — run & supervise round-trip
- [[screen/relay-plan]] — tapping a card opens its plan
- [[screen/relay-decisions]] — where a `gate` verdict leads
- [[screen/relay-nudge]] — where a `stall` verdict leads
