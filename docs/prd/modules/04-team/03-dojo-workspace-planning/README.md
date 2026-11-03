# Dōjō Workspace Planning · T3

**Plane:** team · **Status:** [roadmap/04-team.md](../../../../roadmap/04-team.md) ·
**Serves:** org admin ([persona 4](../../../03-personas.md#4--org-admin--the-plane-owner)) and
maintainer ([persona 3](../../../03-personas.md#3--maintainer--the-curator)), who set targets (D54);
lead ([persona 8](../../../03-personas.md#8--lead--the-team-lead)), who reads results per person and
accepts a deviation (D38, D61); manager / delivery lead
([persona 6](../../../03-personas.md#6--manager--delivery-lead--the-roll-up)), as a maintainer or a lead
when a member; client / workspace stakeholder
([persona 7](../../../03-personas.md#7--client--workspace-stakeholder--the-narrow-lens)), a viewer
reading aggregates (D8, D68); and the solo and team developer
([1](../../../03-personas.md#1--solo-developer--the-wedge),
[2](../../../03-personas.md#2--team-developer--the-inheritor)), whose assistant receives the ticket,
its targets and any misses (D62, D63, D61).

## Purpose

Dōjō Workspace Planning ties a workspace's work to its tracker and holds that work to **targets**. A
workspace groups many repositories and connects to an external tracker (Jira today; Azure DevOps Boards
and GitHub Issues designed). A tracker **ticket is the unit of work handed to the assistant** (D63). A
**target** is a metric threshold on the code (D47). A target set to `block` makes its miss
**blocking**: the UI says "target missed (blocking)", Sensei tells the assistant which targets are
missed and what to fix, and in the dōjō a lead or admin accepts any deviation that remains (D61). The
planner (M20) emits a `PlanGraph`; the run engine (M21) drives execution; T3 syncs the tracker so the
plan and the ticket↔session links reflect the tracker's state.

The rule the whole module turns on:

> **Targets read the code; insights only describe how it was made.**

A target reads what was *merged* (hard-rule violations, coverage on guarded paths, new layer, cycle and
anti-pattern violations, changed-function complexity). An insight reads how the *sessions went*
(first-try rate, corrections, AI-assisted share, relay approval wait) and is never a target. The user
feels this as *the assistant knows the ticket and its targets before it starts, a miss comes back as a
list of what to fix, and a workspace that inherited a mess is measured against its own history.*

> **Built vs not built.** **Built on develop:** the daemon-side Jira adapter (PR #274:
> `crates/senseid/src/tracker/`, with `sensei.trackers`, `sprints`, `tracker_tasks` and
> `session_task_links`); session↔ticket linking by a key in the **branch** or the **prompt**
> (`link_source` enum; `commit` is reserved and never written); `GET /api/workspaces/{id}/tracker-tasks/{key}`;
> the checker substrate target 1 reads (`checker.rs`); module zone placement
> (`sensei.component_zones`, #223). **Not built:** target configuration and evaluation (no table, no
> API, no screen), the ladder, the missed-target handoff, the deviation acceptance, the dōjō-owned
> tracker path (D49, needs #310), the MCP ticket fetch and start-of-work link (D63, designed in
> [M14](../../03-delivery/14-mcp-context-delivery/README.md)), the list of tickets assigned to the user,
> the Azure Boards and GitHub Issues adapters (T-1), the dōjō Home "needs you" queue (D69) and the
> viewer's target view (D68). Ticket writeback (ac7822d2b) exists only on the branch
> `feature/tracker_metrics_writeback`, not on develop. Of the seven targets, 1 has its reading today
> (hard-rule violations), 2 need a scoped read (coverage, changed-function complexity), 1 has its
> placement computed but no per-sprint read (zone of pain), and 3 are blocked on the
> architecture-metrics family (#156). The workspace window Tasks tab (M16) stays unbuilt until planner
> persistence lands (C-1 → M20).

## In scope / out of scope

- In: **repository grouping** (a workspace groups many repositories, from any provider or a local path
  via the CLI); **tracker sync** (Jira / ADO / GitHub Issues) → `PlanGraph` fields, with **one tracker
  owning sprints**; **tracker ownership** — dōjō-owned for org workspaces, the daemon path for solo and
  personal use (D49); **ticket writeback, opt-in per workspace** (D50); the **ticket as the unit of
  work** — the assigned-tickets list, the assistant's ticket fetch that marks start of work, key
  matching as the fallback (D63); the **seven targets**, all code-derived and fair (delta-scoped or
  ratcheted); the **ladder** (Dōjō → Workspace → Repository → Sprint → Task type → Task, most specific
  wins — D48); **who sets and who accepts** (D54, D61); the **modes** (`block` / `warn` / `off`); the
  **own-baseline ratchet**; the **missed-target handoff and deviation acceptance** (D61); the **sprint
  scorecard**; **where results surface** (assistant, desktop, dōjō Home, viewer, lead — D62, D67, D68,
  D69, D38); **notifications** (Slack hooks); the **pricing line** for targets (D52).
- Out: **targets owned by teams** — teams report results only (D13, D48); **an Epic level** in the
  ladder (not in v1, D48); **a hard stop** — a blocking miss never locks the tracker, a merge or a
  sprint (D61; trackers are read-only unless writeback is on, D3/D50); **client separation** — the
  dōjō is the client, and a session is governed by its repository's dōjō alone (D53); the planner
  persistence (C-1 → M20), the run engine (M21), the Instruments verdict classifier (M18), the
  architecture-metrics family (#156); **per-author metrics as targets** (never).

## Surfaces

- **Assistant (MCP)** — at the start of work on a ticket the assistant receives the ticket, its rules
  and its resolved targets (D62); after a change it receives the missed targets and what to fix (D61).
  Tools designed in [M14](../../03-delivery/14-mcp-context-delivery/README.md); not built.
- **Desktop app** — the Dōjō screen's "What applies here" lists the resolved rules and targets for the
  current workspace, each with its source rung (D67); the workspace window shows each target's result
  for the current change. Not built.
- **Dōjō web** — the workspace page `/org/[slug]/workspaces/[id]` (built: per-repository metrics,
  WP5) gains the mockup's tabs (`dojo2-project.jsx`): **Repos · Team · Integrations · Targets ·
  Sprints · Code health**. **Team** lists repository roles mirrored from the forge with the admin
  override (D5) and the workspace's viewers (T6). **Targets** edits each rung the signed-in person may
  set (D54). **Sprints** has two modes of one tab: the sprint scorecard, and the targets × sprints
  matrix. **Code health** embeds the [Code map](../../03-delivery/17-code-map/README.md); each
  target's reading (layers, cycles, zone of pain, complexity) opens the lens that explains it. The
  **Home** is the "needs you" queue (D69). The viewer shell shows metrics first and target results in
  aggregate (D68). The tabs, the queue and the viewer's target view are not built.
- **Daemon** — `planner.rs` (v0 generate), `plan_graph.rs` (model), `runs.rs` (register plan, update
  task status, report outcome); `tracker/` (the Jira adapter: connect, discover, sync, key linkage —
  PR #274), which is the solo/personal path (D49). Azure Boards and GitHub Issues adapters: not
  present (T-1).
- **Data** — `sensei.runs`, `sensei.run_events`; `sensei.metrics` (its `target` column is a
  normalisation bound, not target configuration); `sensei.trackers`, `sensei.sprints`,
  `sensei.tracker_tasks`, `sensei.session_task_links` (PR #274); `dojo.repository_roles` and
  `dojo.memberships` (who may set which rung, D54). No target table exists yet.

## Data invariants

- **Lead.** **Targets read the code; insights never become targets.** Every target's reading is
  code-derived; behavioural and AI-usage readings, **including FTR**, are insights only.
- **Lead.** **Fair by construction** (persona 6, "what must be fair"). A target is **delta-scoped** —
  what this sprint or this ticket introduced: *new* violations, *changed* functions, *guarded* paths —
  **or a ratchet against the workspace's own trailing baseline**, which only ever asks for
  improvement. Never a borrowed benchmark, and never an absolute workspace-level ratio (§9.3). A
  reading over thin coverage shows as thin, not strong (§9.6).
- **Lead.** The **ratchet advances when the trailing 3-sprint median beats the baseline** — a single
  spike never advances it (V3). Targets 2 (coverage on guarded paths), 5 (zone of pain) and 6 (changed
  functions over complexity 10) ratchet against the workspace's own baseline; targets 1, 3, 4 and 7
  stay delta-scoped at 0 (V4).
- **Lead.** **A reading from an incomplete index is not evaluated** — it reads *not measured* and is
  neither met nor missed (V12).
- **Lead.** **Sprint targets read the development environment's branch** (falling back to
  production's — D18); release gates read development → testing → production in order (T5, M4).
- **Lead.** **The ladder is Dōjō → Workspace → Repository → Sprint → Task type → Task, most specific
  wins** (D48); a rung with no value inherits the nearest broader rung's. Teams own no targets (D13).
- **Lead.** **Who sets a rung** (D54): Dōjō and Workspace — admin; Repository — maintainer and up on
  that repository; Sprint, Task type and Task — maintainer and up on any repository in the workspace.
  A lead sets targets only where also a maintainer (D58).
- **Lead.** **Modes are `block` / `warn` / `off`.** A `block` miss is "target missed (blocking)": it
  is handed to the assistant to clear and, while it stands, waits in the dōjō for a lead or admin to
  accept the deviation (D61). A `warn` miss is shown and needs no acceptance. `off` is not evaluated.
- **Lead.** **A blocking miss is information plus a handoff, not a hard stop** (D61). Nothing locks a
  merge, a sprint or the tracker. **A lead or admin accepts the deviation with a note**, and may open a
  follow-up ticket for what the miss identified.
- **Lead.** **The tracker is read-only unless the workspace turns writeback on** (D3, D50; off by
  default). Opening a follow-up ticket is a tracker write: with writeback off, Sensei drafts the
  ticket for the lead to file.
- **Lead.** **One tracker owns sprints.** Issues and task types may come from several trackers; the
  sprint calendar has exactly one owner.
- **Lead.** **The assistant's ticket fetch marks the start of work** and links the session to the
  ticket (D63). Key matching on the branch or prompt (and the commit, once built) links a session
  whose agent fetched the ticket elsewhere. A session with neither is **unlinked** and counts toward
  no ticket.
- **Lead.** **A session in a repository is governed by that repository's dōjō alone** — its rules and
  its targets (D53). The person's own user-rung rules still apply locally.
- **Lead.** **Org private repositories need a subscription for targets; everything else is free**
  (D52). An unentitled org-private repository shows the Targets option, says targets on private
  repositories are on paid plans, names the plan, and links to Settings › Plan & billing (D46). The option is never hidden.
- **Lead.** **Viewers read aggregates; leads read per person.** A viewer sees target results for the
  granted workspace in aggregate, never per person, and an aggregate over fewer than 2 contributors is
  withheld (D8, D68, F4). A lead reads results per person for the teams they are on (D38).

## Quality headline

A display summary, **never a target**: the share of targets met per sprint, against a goal line
**the workspace sets** (no dōjō-wide default) — `86% met · goal 80%` (V2). The viewer (T6) shows it.

## The seven targets

All seven are code-derived. Values are the mockup's Dōjō default and the workspace's override; the
ladder resolves which one applies.

| # | Target | Compare | Dōjō | Workspace | Mode | Source | Reading today? |
|---|---|---|---|---|---|---|---|
| 1 | Hard-rule violations at merge | `= 0` | 0 | inherits | block | constitution checks (`checker.rs`) | ✅ substrate built; not wired to a target |
| 2 | Coverage on guarded paths | `≥` | 75 % | 80 % | block | test runs (`auth/**`, `payments/**`) | 🟡 needs a path-scoped read |
| 3 | New layer violations | `= 0` | 0 | inherits | block | code health · layer conformance | ⛔ #156 |
| 4 | New dependency cycles | `= 0` | 0 | inherits | block | code health · cycles | ⛔ #156 |
| 5 | Modules in the zone of pain | `≤` | 2 | 1 | warn | code health · `component_zones` (#223) | 🟡 placement computed; needs a per-sprint read |
| 6 | Changed functions over complexity 10 | `≤` | 3 | 2 | block | code health · cyclomatic, per function | 🟡 needs a per-function read |
| 7 | New high-severity anti-patterns | `= 0` | 0 | inherits | warn | patterns · anti-pattern detector | ⛔ #156 |

**Overrides** narrow a target at a narrower rung — e.g. target 2 for one repository `≥ 85 %`, for
`migration` tasks `≥ 90 %`, or target 6 for one legacy task `≤ 4` with a note. Most specific wins
(D48).

### Who sets, who accepts

| Rung | Sets the target (D54) | Accepts a blocking miss (D61) |
|---|---|---|
| Dōjō | admin | lead or admin |
| Workspace | admin | lead or admin |
| Repository | maintainer and up on that repository | lead or admin |
| Sprint · Task type · Task | maintainer and up on any repository in the workspace | lead or admin |

A team reports results and sets nothing (D13). A viewer sets and accepts nothing (D8).

### Pricing

| Dōjō / repository | Targets, blocking included |
|---|---|
| Personal dōjō (solo developer), public or private | free (D52, D46) |
| Org dōjō, public repository | free (D52) |
| Org dōjō, private repository | paid; the subscription is per org, priced by the seats on its private repositories (D52, D59) |

Local Sensei works in full on any plan (D46).

## A missed blocking target

What happens, in order (D61, D62, D63):

1. **Start of work.** The user hands the assistant a ticket key. The assistant fetches the ticket
   through the daemon's MCP tool, which marks the start of work and links the session (D63). The same
   call delivers the ticket's rules and the targets resolved for it (D62).
2. **After a change.** Sensei evaluates the targets over what the change introduced. For each missed
   target it hands the assistant the target, the reading, and what to fix (the files, functions or
   edges behind the miss).
3. **The assistant clears it** where it can. A cleared miss leaves no deviation.
4. **What stands goes to the dōjō.** A blocking miss still standing at review joins the dōjō Home
   "needs you" queue (D69). A lead or admin **accepts the deviation with a note**, and may **open a
   follow-up ticket** for the issues the miss identified — written to the tracker when the workspace
   has writeback on (D50), otherwise drafted by Sensei for the lead to file.
5. **The record.** The scorecard shows the miss, the acceptance, its note and who accepted it. Two
   misses on the same cause propose a memory (D66, M9).

## Insights — tracked every sprint, never targets

| Insight | Reads |
|---|---|
| First-try rate | how often a session lands without correction |
| Corrections per session | how much steering a session needed |
| AI-assisted share of commits | how much of the change came through sessions |
| Relay approval wait | how long sessions waited on a gate (a relay human approval, M21) |

They describe how sessions went, not what was merged, so they are never targets. They show beside the
scorecard for context.

## Tracker sync & task linkage

- **Ownership (D49).** An **org workspace's** tracker is connected once in the dōjō, which holds the
  credential (Wave C; needs the #310 secret store; not built). The **daemon path** — each person's
  daemon pulls at that person's cadence (D22, PR #274, built for Jira) — stays for solo and personal
  use. New adapters (GitHub Issues, ADO) go on the dōjō path.
- **Writeback (D50).** Off by default; a workspace opts in. It is the only way Sensei writes to a
  tracker (usage comments, ac7822d2b; a follow-up ticket, D61). Not on develop.
- **Providers** — repositories from GitHub, GitLab, Azure Repos, Bitbucket, or a local path.
- **What is pulled** — issues/work items, sprints/iterations, task types and **milestones** (Jira
  versions / epics, ADO milestones — V11) into the `PlanGraph`. The Jira mirror today holds key,
  summary, issue type, status and current sprint; it holds no assignee, description or acceptance
  criteria (`tracker_tasks`).
- **One sprint owner.** Jira or ADO owns the sprint calendar; another tracker may still supply issues
  and task types. A workspace has one tracker today (`unique (project_id)`); several, at different
  levels, are designed (D23, D40).
- **Task types** map to Sensei's kinds — e.g. `Story → feature`, `Bug → bug`, `Task → chore`,
  `label: migration → migration` — and are the Task type rung of the ladder.
- **The ticket as the unit of work (D63).** The sync lists the tickets assigned to the user (not
  built). The user hands the assistant a key; the assistant's fetch links the session and marks start
  of work (not built).
- **Key matching, the fallback** — a key pattern per repository (e.g. `AUTH-\d+`, `AB#\d+`). A session
  links when the key appears in the **branch** or the **prompt** (built); the **commit message** is
  reserved, not built. It covers an agent that fetched the ticket elsewhere (e.g. `gh issue view`).
- **Unlinked sessions** — no fetch and no key — are reported ("*41 of 47 sessions carry a key · 6
  unlinked don't count toward any ticket*") and count toward no ticket.

## Notifications (Slack)

Three hooks, each independently toggled, posted to a channel. Not built, and not this week's surface:
the dōjō Home queue is (D69).

| Hook | Fires when |
|---|---|
| `miss` | a blocking target is missed and not cleared by the assistant (once per target, with the tickets behind it) |
| `close` | a sprint closes (its misses, acceptances and their notes) |
| `gate` | a session waits on a relay approval for over 30 m (and who it is waiting on) |

## Signals shown

| Signal | Value | Decision it drives / next step |
|---|---|---|
| Tracker sync | `acme.atlassian.net · project AUTH · board "AUTH Scrum" · synced 2m ago · 142 issues` | stale or failing → reconnect or fix the credential (a stopped sync joins "needs you", D69) |
| Sprint ownership | `Owned by Jira — 6 imported, AUTH Sprint 23 active` | two claimants → pick the one tracker that owns sprints |
| My tickets | `AUTH-412 · AUTH-418 assigned to you` | hand a key to the assistant to start work (D63) |
| Start of work | `AUTH-412 fetched by the assistant 10:42 · session linked` | none — candidate to cut (a record; the link is what matters) |
| Ticket linkage | `AUTH-\d+ found in branch · prompt` | a key never matches → fix the key pattern for the repository |
| Sprint linkage | `41 of 47 sessions carry a key · 6 unlinked` | open the unlinked sessions; start work by fetching the ticket |
| What applies here | `Cov. ≥ 80% · block · from Repository auth-svc` | open the source rung to change it (if you may set it, D54) |
| Target result | `Cov. 77% on auth/** — target missed (blocking)` | hand the fix list to the assistant; else accept the deviation (lead/admin) |
| Missed target handoff | `2 missed: coverage on auth/**, 3 changed fns > 10 — fix: add tests for login.ts; split parseToken` | the assistant clears the listed items |
| Needs you | `AUTH Sprint 23 · 1 blocking miss awaiting acceptance` | accept with a note, and open (or file the drafted) follow-up ticket |
| Accepted deviation | `accepted by M. Rao · "legacy parser, AUTH-430 opened"` | follow the linked follow-up ticket to done |
| Override | `migration ≥ 90%` · `AUTH-420 ≤ 4` | review at sprint end; remove when the reason is gone |
| Ratchet | `coverage baseline +0.03 sustained at sprint 20` | none — candidate to cut (a result; the new baseline applies on its own) |
| Thin reading | `Cov. 82% · 3 files measured` | widen what is measured before trusting the target |
| Per person (lead) | `R. Iyer · 2 misses this sprint, both coverage` | offer help where one person's misses cluster (D38) |
| Aggregate (viewer) | `AUTH workspace · 86% of targets met · goal 80%` | ask the team about a missed goal; viewers act through the members (D68) |
| Unentitled private | `Targets on private repositories are on paid plans — Settings › Plan & billing` | subscribe, or keep the repository's targets off (D52) |
| Insight (never a target) | `First-try rate 81%` | none here — read it in M19; it never blocks |

## Done gate

- A workspace connects to a tracker; with writeback off (the default) **nothing is written back**
  (D50).
- The assistant fetches a ticket by key through the daemon's MCP tool; the session is linked and the
  start of work is recorded; the ticket's rules and resolved targets come with it (D62, D63).
- The targets with a reading (hard rules today; coverage, complexity and zone of pain once their
  scoped reads land) are evaluated on each change, and each miss reaches the assistant with what to
  fix (D61).
- The ladder resolves most-specific-wins across Dōjō → Workspace → Repository → Sprint → Task type →
  Task; a `null` rung inherits (D48). A person cannot set a rung their role does not allow (D54).
- A blocking miss that stands appears in the dōjō Home queue; a lead or admin accepts it with a note;
  the follow-up ticket is written (writeback on) or drafted (writeback off) (D61, D69).
- A viewer sees aggregate target results only; a lead sees them per person (D68, D38).
- An org-private repository without a subscription shows the Targets option with the plan named
  (D52).
- A session with no fetch and no key is **unlinked** and counts toward no ticket; the count is shown.

```bash
# Not implemented — placeholder
curl -s "localhost:7744/api/workspaces/$WS/tracker-tasks/AUTH-3" | jq   # built: the mirrored ticket
curl -s -H "Authorization: Bearer $TOKEN" \
  "https://dojo.sensei-hq.com/v1/t/org/sensei-hq/workspaces/$WS/targets" | jq   # not built
```

## Wrong gate

- **A target uses a borrowed benchmark or an absolute workspace-level ratio** instead of a delta or
  the workspace's own baseline — fairness regressed.
- **An insight — FTR, corrections, AI-assisted share, approval wait — is treated as a target** — the
  target/insight split regressed.
- **The baseline advances on a single spike** — ratchet regressed.
- **A `warn` or `off` target lands in "needs you"** — the mode model regressed.
- **A blocking miss hard-stops a merge, a sprint or the tracker** — D61 regressed.
- **A narrower rung does not override a broader one**, or a team owns a target — the ladder regressed.
- **A maintainer sets a Workspace target, or a viewer accepts a deviation** — D54/D61 roles regressed.
- **A viewer sees a per-person result**, or an aggregate over one contributor — D8/F4 regressed.
- **Two trackers claim sprint ownership**, or an adapter **writes with writeback off** — ownership /
  D50 regressed.
- **An unlinked session counts toward a ticket.**
- **The Targets option is hidden on an unentitled private repository** instead of naming the plan —
  D52 regressed.
- **The UI calls a target a "gate"** — D47 vocabulary regressed.

## Scenarios

- [`scenarios/dojo-workspace-planning.feature`](scenarios/dojo-workspace-planning.feature) — the
  target/insight split, the own-baseline ratchet, the seven targets, the ladder, the roles, the modes,
  the missed-target handoff and acceptance, writeback off by default, pricing (`NO TEST` /
  `# gap: T-1` for adapters).
- [`scenarios/dojo-workspace-planning-gaps.feature`](scenarios/dojo-workspace-planning-gaps.feature) —
  the unbuilt tracker adapters, the ticket fetch, and the scoped reads (`NO TEST` / `# gap: T-1`,
  `# gap: architecture-metrics`).

## Open gaps

- [T-1](../../../../roadmap/06-gaps.md) — **tracker adapters**: the daemon-side Jira adapter is built
  (PR #274); Azure Boards and GitHub Issues are not, and a workspace has one tracker, not several
  (D23).
- [G-SEC-2 / #310](../../../../roadmap/06-gaps.md) — the dōjō has no secret store, so the dōjō-owned
  tracker path for org workspaces (D49) cannot start.
- [T-2](../../../../roadmap/06-gaps.md) — Dōjō join / membership — **resolved**; the org context and
  its role floor are built.
- [C-1](../../../../reorg.md) — planner DB persistence + baseline + ratchet (M20). Blocks the
  workspace window Tasks tab (M16).
- Architecture-metrics family (#156, open) — blocks 3 of 7 targets (layers, cycles, anti-patterns);
  targets 2, 5 and 6 need scoped reads.
- Target configuration, evaluation and acceptance — no table, API or screen yet.
- The MCP ticket fetch, start-of-work link and missed-target handoff — designed in
  [M14](../../03-delivery/14-mcp-context-delivery/README.md), not built.
- The assigned-tickets list — the Jira mirror stores no assignee by design (`tracker_tasks`).

## Related

- [reorg.md §9](../../../../reorg.md) — the target/insight split, own-baseline ratchet, and the seven
  targets.
- [plan/2026-11-03-parallel-lanes.md §0](../../../../plan/2026-11-03-parallel-lanes.md) — D47–D70.
- [roadmap/09-dojo-expansion.md](../../../../roadmap/09-dojo-expansion.md) — D2, D3, D13, D18, D22,
  D23, D40, D46.
- [design/integrations-and-gating-requirements.md](../../../../design/integrations-and-gating-requirements.md) —
  the tracker and targets requirements this spec distils.
- [M14 MCP & Context Delivery](../../03-delivery/14-mcp-context-delivery/README.md) — the ticket fetch,
  target delivery and missed-target handoff tools.
- [T5 Releases & DORA](../05-releases-and-dora/README.md) — the same targets and ladder over a
  release's change set.
- [spec/dojo/governance](../../../../spec/dojo/governance/) — the constitution checks target 1 reads.
- [prd/02-objectives.md §06](../../../02-objectives.md) · prd/03-personas.md
  [§3](../../../03-personas.md#3--maintainer--the-curator),
  [§4](../../../03-personas.md#4--org-admin--the-plane-owner),
  [§6](../../../03-personas.md#6--manager--delivery-lead--the-roll-up),
  [§7](../../../03-personas.md#7--client--workspace-stakeholder--the-narrow-lens),
  [§8](../../../03-personas.md#8--lead--the-team-lead).
- Crates: `crates/senseid/src/{planner.rs, plan_graph.rs, api/handlers/runs.rs, api/handlers/trackers.rs,
  tracker/}`.
- Mockup: `docs/mockups/Sensei/lib/dojo2/dojo2-project.jsx`.
