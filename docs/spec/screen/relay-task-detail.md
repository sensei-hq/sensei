# 経 · Relay · Task detail

**Segment:** 05 · Relay (mobile companion)
**Route:** Relay iOS app · task detail (no web route — phone app)
**Source mockup:** [`lib/relay/relay.jsx`](../../mockups/Sensei/lib/relay/relay.jsx) → `RelayTaskDetail`
**Data:** greenfield — the coordinator publishes a filtered per-task view. Proposed: `GET /api/relay/agents/{id}` → `{ title, machine, assistant, ago, state, plan: { of, x, steps: [{label, status}] }, activity: [{time, kind, summary, live?, block?}], gate?: {kind:'approve', label, command, machine} }`. Status + plan structure only — never code or transcripts.
**App file:** _greenfield — mobile app not built_
**Daemon files:** _greenfield — coordinator + relay transport not built_
**Status:** greenfield (Relay backend unbuilt) — see [architecture/relay](../../architecture/relay.md)

## Purpose

The user tapped one task on the dashboard and wants the full picture of a
long run in one scroll: *where is the plan, what has the agent done, and
what does it want from me?* This screen carries **both** mental models at
once — a stage checklist (**Plan · n of x**) that shows how far the modular
plan has walked, and an **activity timeline** that shows the beats of the
run underneath it. When the task is blocked, the pending **gate docks as a
sheet** at the bottom — the exact command it wants to run, with Approve/Deny
right there — so the user can read the whole story and act without leaving
the screen. It is the "I trust it because I can see the plan and the beats"
surface.

## Data invariants

- Everything on screen is the coordinator's **filtered per-task status**:
  plan structure (labels + per-step status), activity beats (time + a
  short composed summary + optional file *name* and +/- line *counts*),
  and the pending gate. **ZERO-KNOWLEDGE: no code, no diff bodies, no file
  contents, no transcript text.** A timeline row may say `Edited auth.ts
  +42 −8` — the file *name* and *counts* are status; the actual diff is
  not, and must never cross the relay.
- The header reads `machine · assistant · ago` (e.g. `macbook-pro · Claude · 4m`)
  — this is the only place the assistant family is named on this screen (R7).
- **Plan checklist** = the modular plan's steps for this task, each with a
  status: `done` (filled dot + check, muted label), `block` (accent ring +
  "needs approval", tinted row), or `todo` (faint ring). The eyebrow states
  progress as **`Plan · {n} of {x}`** where `n = count(done)`, `x = total steps`.
- Exactly the step in `block` state is the one the docked gate corresponds to;
  there is at most one blocking step per task at a time.
- **Activity timeline** rows carry `time` and a summary; a `live` row is
  jade-dotted (in progress), a `block` row is accent-dotted and time-labelled
  `HH:MM · WAITING`. The most recent block row restates the gate in prose
  ("Wants to apply the migration.").
- The **approval sheet** appears only when `state == 'block'` and a gate is
  pending. It shows the **exact command** the agent wants to run, preceded by
  a context label (the mockup: `RUN ON PRODUCTION DB`). Approve and Deny are
  the only two actions; Approve routing to the full-screen [Approve](relay-approve.md)
  gate (for destructive/dry-run detail) is acceptable, but the docked sheet
  must itself already show the exact command — never a vague "the agent wants
  to do something."
- A non-blocked task (`live`/`done`) shows the plan + timeline with **no**
  docked sheet.

## Signals shown

Header:

| Element | Value | Meaning | Example |
|---|---|---|---|
| Back button | → [Dashboard](relay-dashboard.md) | return to the 3-state list | — |
| Context line | `machine · assistant · ago` (mono) | which agent/machine, freshness (R7) | `macbook-pro · Claude · 4m` |
| Title | `task.title` (display) | the task | `Refactor auth module` |
| State line | dot + "Blocked — waiting for your approval" | headline state | accent dot + text |

Plan checklist (`Plan · n of x`):

| Element | Value | Meaning | Example |
|---|---|---|---|
| Eyebrow | `Plan · {done} of {total}` | modular-plan progress (R1) | `Plan · 4 of 6` |
| Done step | filled dot + check, muted label | completed phase step | `Analyze current auth flow` ✓ |
| Blocking step | accent ring, tinted row, sublabel | the step waiting on you | `Apply DB migration` · needs approval |
| Todo step | faint ring, normal label | not yet started | `Update tests & ship` |

Real mockup steps: `Analyze current auth flow` (done) · `Draft OAuth schema`
(done) · `Migrate user model` (done) · `Apply DB migration` (**block** · needs
approval) · `Update tests & ship` (todo).

Activity timeline:

| Element | Value | Meaning | Example |
|---|---|---|---|
| Beat time | `HH:MM` (mono) | when the beat happened | `14:02` |
| Beat text | composed summary (+ optional file name / line counts) | what happened, status-only | `Started task · read 12 files` |
| Edit beat | file *name* chip + `+N` / `−N` counts | edit occurred (no diff body) | `Edited auth.ts +42 −8` |
| Live beat | jade dot | in progress | `Ran test suite · 18 passed` |
| Waiting beat | accent dot, `HH:MM · WAITING` | the gate, restated | `14:10 · WAITING — Wants to apply the migration.` |

Docked approval sheet (when blocked):

| Element | Value | Meaning | Example |
|---|---|---|---|
| Context label | e.g. `RUN ON PRODUCTION DB` (mono eyebrow) | where/what class of action | `RUN ON PRODUCTION DB` |
| Exact command | monospace, verbatim | the command it wants to run (R4) | `psql < 003_add_oauth.sql` |
| Approve | accent button | authorize the exact command | `Approve` |
| Deny | muted button | refuse | `Deny` |

## Done gate

The screen shows the coordinator's filtered per-task view: a `Plan · n of x`
checklist whose `n` equals the count of done steps and whose single blocking
step matches the docked gate; an activity timeline of composed status beats
(file names + line counts allowed, diff bodies never); and — only when
blocked — a docked sheet that names the **exact command** with Approve/Deny.
No code, diff body, or transcript text appears anywhere.

## Wrong gate

- **A diff body, code snippet, or file contents render in a timeline beat.**
  Only file *names* and +/- *counts* are status; the diff itself must not
  cross the relay (R5).
- **The docked sheet says "the agent wants to do something" with no exact
  command.** An approve gate with no exact command is the cardinal Relay
  failure (R4) — Approve must never be a blind yes.
- **`Plan · n of x` disagrees with the checklist** (n ≠ count of done steps,
  or x ≠ total). The header count was computed separately from the list.
- **More than one blocking step, or a blocking step with no docked gate.**
  The plan and the gate fell out of sync.
- **The docked sheet appears on a `live`/`done` task.** The sheet is
  gate-only; a running or finished task has nothing to approve.
- **The assistant/machine context is missing**, so a multi-machine user
  can't tell which agent this is (R7).

## Related

- Objectives [R1](../../requirements/objectives.md#relay--supervising-long-runs-from-anywhere) (modular plan · phases → features · checkpoints · gates), [R4](../../requirements/objectives.md#relay--supervising-long-runs-from-anywhere) (minimal + exact human-in-the-loop · exact command), [R5](../../requirements/objectives.md#relay--supervising-long-runs-from-anywhere) (zero-knowledge)
- [architecture/relay](../../architecture/relay.md) — the planner model (project → plan → phase → feature·checkpoint·gate) + gate shapes
- [journeys/relay](../../journeys/relay.md) — run & supervise round-trip
- Sibling relay screens: [Dashboard](relay-dashboard.md) · [Approve](relay-approve.md) (full-screen gate) · [Respond](relay-respond.md) (decide gate) · [Security](relay-security.md) · [Pairing](relay-pairing.md)
