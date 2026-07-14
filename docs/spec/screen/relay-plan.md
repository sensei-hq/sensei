# 段 · Relay · Plan

**Segment:** 05 · Relay (mobile companion · planner)
**Route:** Relay iOS app · Plan (no web route)
**Source mockup:** [`lib/relay/relay-planner.jsx`](../../mockups/Sensei/lib/relay/relay-planner.jsx) → `RelayPlan`
**Data:** _greenfield_ — `GET /api/relay/projects/{name}/plan` (coordinator-published, filtered status only): the one active plan → ordered phases, each with a `state` (`done`/`doing`/`next`), a `count` of done-of-total items, and its items — features · checkpoints · gates — each an item with a `kind` (`done`/`doing`/`gate`/`next`) and an optional filtered `note`.
**App file:** _greenfield — mobile app not built_
**Daemon files:** _greenfield — coordinator + planner model not built_
**Status:** greenfield (Relay backend unbuilt) — see [architecture/relay](../../architecture/relay.md)

## Purpose

Tap a project on [Projects](relay-projects.md) and see **the plan itself** — the
modular structure R1 promises: phases stacked in order, each carrying its
features, checkpoints, and gates. The current phase is expanded so the user
sees what's `done`, what's `doing` right now, and what a **gate** is waiting
on; earlier and later phases collapse to a header + `count`. A **gate marker**
(the lock) is the one thing that pulls the eye — it's the step that will stop
for a human. This is read-only supervision: the user watches the plan run in
auto mode and drills into a gate only when one appears.

Kanji is 段 — *dan / step, stage* — the phased structure of the plan.

## Data invariants

The planner model this screen renders in full:

- **One active plan per project**, named in the hero
  (`OAuth & session hardening`); the mono sub-label reads `{project} · auto`
  to confirm auto mode (R2).
- **A plan is phases in order.** Each phase has `n` (`01`…), `name`, a
  `state` (`done` / `doing` / `next`), and a `count` string `done / total`
  (`1 / 3`). Exactly the current phase is `doing`.
- **A phase holds features · checkpoints · gates** as items. Each item has
  a `kind` — `done`, `doing`, `gate`, or `next` — and an optional `note`.
  The **item marker** encodes kind: a filled check (`done`), a pulsing dot
  (`doing`), a **square lock** (`gate`), an empty ring (`next`).
- **A gate is a step that stops for a human.** Plan authoring (desktop)
  marks which steps gate; here a gate item shows the lock marker and an
  accent `note` (`decision waiting — needs you`). It is the only item kind
  that blocks — everything else runs non-blocking.
- **Progress is plan-level.** The hero shows `Phase {n} of {x}` and a
  whole-plan `pct` bar, matching the project card that opened it.
- **Filtered status only.** An item `note` is a track/checkpoint label with
  a wall-clock time (`writing store.ts · 14:08`), never a diff or a
  transcript. Only the current phase expands; collapsed phases publish just
  their header + count.

## Signals shown

Hero:

| Element | Value | Meaning | Example |
|---|---|---|---|
| Back | ← | return to Projects | — |
| Auto label | `{project} · auto` (mono) | which project, auto mode on | `lumen-auth · auto` |
| Plan name | `plan` (display) | the one active plan | `OAuth & session hardening` |
| Phase count | `Phase {n} of {x}` | where the plan is | `Phase 2 of 4` |
| Progress | `pct`% + `PBar` | whole-plan progress | `47%` |

Per-phase header:

| Element | Value | Meaning | Example |
|---|---|---|---|
| Number | `n` (mono) | phase order | `02` |
| Name | `name` | phase title | `Session hardening` |
| Count | `count` | items done of total | `1 / 3` |
| State pill | `done` / `in progress` / `next` | phase state (the `StatePill` vocabulary) | `in progress` |

Item (`PlanItem`) — feature · checkpoint · gate:

| `kind` | Marker | Text weight | Note colour | Example |
|---|---|---|---|---|
| `done` | filled ink circle + check | muted | — | `Rotate signing keys` |
| `doing` | ring + pulsing success dot | semibold | success | `Refresh-token store` — `writing store.ts · 14:08` |
| `gate` | square accent lock | semibold | accent | `Session strategy` — `decision waiting — needs you` |
| `next` | empty ring | normal | muted | `Feature flag behind auth_v2` |

Worked example (the mockup plan):

- `01 Foundation` — **done** · `2 / 2` (collapsed)
- `02 Session hardening` — **doing** · `1 / 3` (expanded): Rotate signing
  keys `done` · Refresh-token store `doing` (writing store.ts · 14:08) ·
  Session strategy `gate` (decision waiting — needs you)
- `03 Rollout` — **next** · `0 / 2` (expanded): Feature flag behind
  auth_v2 · Gradual 5% → 100%
- `04 Cleanup` — **next** · `0 / 1` (collapsed)

## Done gate

- The hero name, `Phase {n} of {x}`, and `pct` match the project card that
  opened this plan (one plan, consistent numbers).
- Phases render in `n` order; exactly one phase is `doing` and it is the
  one expanded by default. `done` phases precede it, `next` phases follow.
- Each phase `count` equals its done-of-total items, and the state pill
  uses the exact labels `done` / `in progress` / `next`.
- A `gate` item shows the **square lock marker** and an accent note; it is
  the only item kind that visually signals it will stop for a human (R4).
- A `doing` item shows the pulsing dot and a timestamped filtered note
  (`writing store.ts · 14:08`) — a label, never a code line (R5).
- Collapsed phases show only header + count; expanding one never fetches
  or shows code.
- Dark mode: the lock/accent gate marker and the success `doing` dot stay
  distinct on `bg-paper-soft`.

## Wrong gate

- **No gate marker on a step that gates.** The step reads like an ordinary
  `next` item, so the human never learns a decision is coming (R1/R4
  broken — plan authoring's gate flag was dropped on the wire).
- **Two phases marked `doing`.** Auto mode runs one active phase; two
  `doing` phases means phase state isn't derived from a single cursor.
- **`count` disagrees with the visible items** (`1 / 3` but two checks
  shown) — the summary and the item list came from different reads.
- **A collapsed phase leaks item detail / code.** Only the current phase
  expands, and only filtered labels ever appear; a diff in a note breaks
  zero-knowledge (R5).
- **Plan-level `pct` shown as the current phase's percent** — a plan on
  phase 2 of 4 reading the phase's own completion misrepresents the run.
- **Gate item is silently auto-resolved** and shown as `done` without the
  human ever deciding — a gate must stop, not proceed (R2/R4).

## Related

- [[architecture/relay]] — the planner model (phases → features · checkpoints · gates)
- [[journeys/relay]] — author a plan · mark which steps gate
- [[screen/relay-projects]] — the card that opens this plan
- [[screen/relay-decisions]] — where a `gate` item's decision is answered
- [[screen/relay-nudge]] — a stalled phase surfaced separately
