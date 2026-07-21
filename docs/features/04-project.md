---
name: Project
type: feature
kind: functional
---

# Project

The project window is a focused workspace for one project. The observatory is the
portfolio view — every project, the whole day; the project window is one project,
deep. Open a project and it gets its own window: everything sensei knows about
that codebase, the work in flight, and the controls for what binds or leaves.

It's where project-specific **actions and tasks** live. The user starts a chunk of
work here (Intake), tracks the tasks it becomes, sees what sensei has learned about
this project, acts on its findings, and — by choice — binds it to a dōjō. The
journey names the arc: work inside one project → trust what it learned → before any
of it travels.

## Flows

1. **Enter a project.** From the observatory's Projects, open one — it lands in a
   window of its own, on Overview: the one thing this project needs.
2. **Start a chunk of work.** Intake: describe what you're about to do; sensei
   recommends a way of working (a playbook) and confirms before it runs.
3. **Work it.** Move through the sections — each shows what sensei found and offers
   one action (Apply · Review · Dismiss). Track the tasks as they progress.
4. **Bind & share (opt-in).** From About, bind the project to a dōjō; findings can
   be shared upstream — always previewed.

## Working style — the operation manual

How work gets done inside a project. Every chunk enters through the same **front
door**: the user describes what they're about to do, and sensei recommends a *way
of working* — a playbook — sized to the risk. This is the project's operation
manual: the rigor follows the chunk, instead of every task getting the same
ceremony.

**The front door (intake).** sensei reads the chunk on three axes and shows its
read back for a sanity check (recommend-and-confirm), never a black box:

| Axis | Values | Read from |
|---|---|---|
| lifecycle | greenfield · stable | existing code + docs → stable; empty / new → greenfield |
| intent | explore · ux · feature · enhancement · bug | the goal of the chunk |
| risk | low · high | blast-radius from the code graph (callers · community reach) |

**Depth follows risk.** A high-risk chunk always keeps the human in the loop; a
low-risk chunk may auto-select a playbook once the recommendation has earned trust
(a strict bar: low-risk, enough runs, high FTR).

**The playbook catalog.** The three axes select one of six ways of working:

| Playbook | When |
|---|---|
| vibe / spike | greenfield, fuzzy objective — explore, keep only what you can justify |
| mockup-first | greenfield, UX-heavy — design the surface before the spec |
| spec-driven | clear objective + high blast-radius — design + edge cases before code |
| gsd (get stuff done) | known feature, low risk — lean plan, then build |
| change-flow | stable enhancement — map impact, design the smallest change |
| debug-flow | stable bug — reproduce, fix, lock with a regression test |

**MVP: run it by hand.** In the MVP the playbook is driven **manually via `sensei:`
commands** — `/sensei:intake` classifies and recommends; then the phase commands
(`/sensei:analyze`, `/sensei:blueprint`, `/sensei:build`, `/sensei:validate`, …)
carry out the chosen way of working. Auto-select and the outcome-learning loop
(playbooks tune from real FTR, governed like any rule) come later.

Deep design — the axes→playbook rule matrix, the learning loop, thresholds, and the
data contract — is the [playbook design module](../design/playbook.md).

## Mockups

- [Project window — sections + sidebar](../mockups/Sensei/lib/project/project-pages.jsx)
- [Section previews (overview · memories · about · …)](../mockups/Sensei/lib/project/project-lite-panes.jsx)
- [Atlas — the project's code graph](../mockups/Sensei/lib/project/project-atlas.jsx)
- [Logs](../mockups/Sensei/lib/project/project-logs.jsx)
- [Intake — the front door](../mockups/Sensei/lib/observatory/intake.jsx) (deep design: [playbook module](../design/playbook.md))

## What's involved

> What the user sees and does. `- [x]` done · `- [~]` partial · `- [ ]` not started.

### Start & track the work

- [~] **Intake** (門) — the front door; classify → recommend a playbook → confirm. See [Working style](#working-style--the-operation-manual) above. (Engine ships today as the observatory front door; the project window is its intended home.)
- [ ] **Tasks / plan** — the chunk becomes a plan: phases, backlog, progress, what's next. (Needs the planner.)
- [~] **Actions** — each section offers one action: Apply, Review, or Dismiss a finding, send it to an assistant, or run a prompt.

### What sensei knows about this project

- [x] **Overview** (全) — the project at a glance, and the one thing to act on now
- [~] **Atlas** (図) — this project's code + architecture graph (structure · calls · communities)
- [x] **Sessions** (録) — assistant sessions in this project
- [x] **Memories** (覚) — what sensei has learned about this project
- [~] **Traceability** (巻) — this project's requirement / doc ↔ code linkage
- [x] **Libraries** (庫) — the libraries this project uses, indexed
- [~] **Instruments** (具) — the MCP tools relevant to this project
- [x] **Patterns** (紋) — patterns and anti-patterns in this project
- [x] **Impact** (果) — verdicts on changes made in this project
- [~] **About** (識) — identity, stack, repos, links, guidelines, backlog; and binding the project to a dōjō (opt-in), which pulls the org's shared standards

### The window itself

- [~] **Its own window** — a project opens as a separate window, so one project's work stays focused and self-contained.

## Status

| Area | Status | Notes |
|---|---|---|
| Project window (separate window) | Partial | dedicated route group; per-project sections scaffolded; epic in progress |
| Intake (front door) | Partial | recommend-and-confirm engine shipped; project-scoped placement is the intended home |
| Tasks / plan | Not started | needs the planner |
| Overview | Done | at-a-glance + one action |
| Atlas / graph | Partial | code + architecture graph |
| Sessions | Done | per-project session digest |
| Memories | Done | per-project learnings |
| Traceability | Partial | requirement / doc ↔ code linkage |
| Libraries | Done | per-project library docs |
| Instruments | Partial | per-project MCP tools |
| Patterns | Done | patterns + anti-patterns |
| Impact | Done | verdicts on applied changes |
| About + dōjō bind | Partial | identity/stack/repos/links/guidelines/backlog; bind is opt-in, partially wired |
| Actions (per-section) | Partial | Apply / Review / Dismiss + send / run |
</content>
