# 信 · Relay · Plan authoring

**Segment:** 05 · Relay (Observatory desktop)
**Route:** `/relay/plan` (under the Relay Observatory rail item) — the plan-authoring view
**Source mockup:** [`lib/relay/relay-desktop.jsx`](../../mockups/Sensei/lib/relay/relay-desktop.jsx) → `RelayPlanAuthor`
**Data:** _greenfield — a project's one active plan (phases → features · checkpoints · gates); each step carries an auto|gate mode; shapes proposed below_
**App file:** _greenfield — not built_ (`app/src/routes/(observatory)/relay/plan/+page.svelte`)
**Daemon files:** _greenfield — planner model not built_ (grows `crates/senseid`; new `plan` module — see [architecture/relay](../../architecture/relay.md) "the planner model")
**Status:** greenfield (Relay backend unbuilt) — see [architecture/relay](../../architecture/relay.md), objective [R1](../../objectives.md#relay--supervising-long-runs-from-anywhere)

## Purpose

Plan authoring is where the user, together with the agent, drafts a
task's plan and **marks which steps must stop for a human**. Sensei
drafts the plan; the user decides which steps run freely (`auto`) and
which must **gate** to a human. The gated steps are *exactly* what the
phone will surface while the user is away — this screen is the source
of the phone's stage checklist and of the pending gates the
[[screen/relay-coordinator]] raises.

This is the desktop authoring surface for the planner model
([architecture/relay](../../architecture/relay.md)): each **project
carries one active plan**, modular as
`project → plan → phase (n of x) → { feature · checkpoint · gate }`.
This view edits the gate/auto disposition of a plan's steps.

It delivers **R1 (plan a long run modularly)** — the plan is the unit
that makes an away-from-keyboard run legible — and it is the origin of
**R2/R4**: everything not marked `gate` runs non-blocking in auto
mode; a `gate` step is where the minimal, exact human-in-the-loop
moment fires.

Kanji is 信 — *signal* (this screen decides which steps will signal the human).

## Data invariants

The plan authored here supervises a run on this machine; the gate
markings become the human-stop contract published through the relay.

- **One active plan per project.** This view edits the *current*
  plan. The plan is modular: phases sequence the work, each holding
  features (units of build), checkpoints (progress markers), and
  gates (steps that stop for a human). The mockup shows a flat step
  list for one phase (`Refactor auth module`); the authoring control
  operates per step.
- **Every step has a mode: `auto` or `gate`.** `auto` steps run
  non-blocking (R2); `gate` steps stop and surface to a human (R4).
  The mode is a per-step toggle, defaulting to whatever sensei
  drafted — the human overrides.
- **A gate carries a reason.** Each gated step records *why* it
  gates (`command touches production`, `opens a pull request`).
  The reason is shown under the step and travels with the gate to
  the phone so the away-from-keyboard user has context.
- **Gate shape is derived from the step**, per R4:
  an **approve** gate (surfaces the exact command first — e.g.
  `Apply DB migration`) vs a **decide** gate (a 3–4-option question
  + free reply). The mockup's two gated steps are both stop-points;
  the shape is a property of the step's action, resolved at run time
  by the coordinator.
- **The gate set is the phone contract.** The count "N of M steps
  will ask for you" is a live derivation (`count(mode == gate)`),
  and starting the run **pushes exactly the gated steps** to the
  paired phone. Nothing not marked `gate` will ever interrupt the
  user remotely.
- **Auto ≠ unsupervised.** Auto steps still publish filtered status
  (done · doing · next) to the coordinator's stream — they simply
  don't stop. Only `gate` steps block.

Proposed shapes (greenfield — name the contract, don't fabricate values):

```
GET /api/relay/plan?project=… →
{
  "project": "lumen-auth",
  "plan_title": "Refactor auth module",
  "steps": [
    { "n": 1, "label": "Analyze current auth flow", "mode": "auto", "why": null },
    { "n": 2, "label": "Draft OAuth schema",        "mode": "auto", "why": null },
    { "n": 3, "label": "Migrate user model",        "mode": "auto", "why": null },
    { "n": 4, "label": "Apply DB migration",        "mode": "gate",
      "why": "command touches production", "shape": "approve" },
    { "n": 5, "label": "Update tests & ship",       "mode": "gate",
      "why": "opens a pull request",       "shape": "approve" }
  ]
}

PATCH /api/relay/plan/steps/{n}  { "mode": "auto" | "gate" }   # toggle a step
POST  /api/relay/plan/start      { "project": "…", "push_to": ["iPhone 16 Pro"] }
   → begins the run in auto mode; pushes the gated steps to the paired phone
```

## Signals shown

Header — eyebrow `信 · relay · plan`, title = the plan name
(**"Refactor auth module"**), sub-copy: *"Sensei drafts the plan with
the agent; you decide which steps run freely and which must stop for
you. Gated steps are exactly what your phone will surface while you're
away."*

Step list (one row per step, in a `bg-paper-soft` bordered card):

| Element | Value | Meaning | Example (mockup) |
|---|---|---|---|
| Step number | mono, zero-padded, `ink-faint` | Ordinal in the plan | `01 … 05` |
| Step label | `text-base`; semibold when gated | The unit of work | `Analyze current auth flow` · `Apply DB migration` |
| Gate reason | `text-sm text-accent`, only when gated | Why this step stops for a human | `gate · command touches production` · `gate · opens a pull request` |
| Auto \| Gate toggle | 2-segment control in a `paper-3` track | Set the step's mode | `Auto` (ink when on) · `Gate` (accent when on) |

Footer / launch:

| Element | Value | Meaning | Example (mockup) |
|---|---|---|---|
| Start button | primary | Begin the run in auto mode with these gates | `Start with these gates` |
| Gate summary | `text-sm text-ink-mute` | Live count of gated steps + push target | `2 of 5 steps will ask for you · pushed to iPhone 16 Pro` |

Interaction notes:

- Toggling a step to `Gate` reveals its reason line and bolds the
  label; toggling to `Auto` hides the reason. The summary count
  recomputes live.
- A step lacking a reason cannot silently gate — a `gate` step must
  carry a `why` (drafted by sensei, editable). The mockup's auto
  steps carry `why: null`; only gated steps show a reason.
- The push target names the paired phone(s) the gates go to (from
  the coordinator's paired-devices list), not a hardcoded label.

## Done gate

- The step list renders one row per plan step from
  `GET /api/relay/plan`, numbered `01…N`, each with its label and a
  working Auto|Gate segmented toggle reflecting `step.mode`.
- Gated steps are visually distinct (bold label + `gate · {why}`
  reason line in `accent`); auto steps show no reason.
- Toggling a step's mode PATCHes the step and **immediately**
  updates the gate summary count (`N of M steps will ask for you`) —
  the count is derived, never hardcoded.
- The gate summary names the real push target phone(s) from the
  coordinator's paired devices.
- "Start with these gates" begins the run in auto mode and pushes
  exactly the gated steps to the paired phone — these become the
  phone's stage checklist and the coordinator's pending gates.
- Every gate step carries a `why`; a gate with no reason is not
  permitted to start.
- Marking every step `auto` produces a valid non-blocking run with
  `0 of M steps will ask for you` (fully unattended) — and marking
  every step `gate` is equally valid (fully supervised).
- Dark mode: the segmented toggle's on/off states and the accent
  reason line stay legible.

## Wrong gate

- **A step gates with no reason** — the phone would surface a
  stop-point with no context for the away-from-keyboard user (breaks
  R4's "minimal + exact").
- **The gate summary count is static / hardcoded** rather than
  derived from the current `gate` markings — toggling a step doesn't
  move the count.
- **Starting the run pushes auto steps to the phone**, or fails to
  push a gated step — the phone contract must be *exactly* the gated
  set (nothing more, nothing less).
- **Auto steps are shown as blocking**, or gated steps as
  non-blocking — the mode's whole meaning is stop vs. don't-stop.
- **An approve gate is authored without its command surfacing at run
  time**, or a decide gate without its 3–4 options — the shape (R4)
  must resolve so the coordinator can present it correctly.
- **The plan is treated as many active plans per project.** A
  project carries exactly one active plan; this view edits that one.
- **Push target is a hardcoded phone name** rather than the paired
  device from the coordinator.

## Related

- [[screen/relay-coordinator]] — where the gates authored here surface as pending gates
- [[screen/relay-dojo-gates]] — the team fan-out of a plan's gates to on-call
- [[architecture/relay]] — the planner model (project → plan → phase → feature · checkpoint · gate)
- [[journeys/relay]] — "author a plan — make the run modular"
- [[screen/observatory-instruments-health]] — sibling Observatory format reference
- [[pipeline/narration-cache]] — mentor-voice text for drafted gate reasons
