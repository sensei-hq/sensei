# External Integrations & Gating — Requirements (T3)

> **Status:** Requirements gathering — feeds the T3 module spec, the T3 `.feature`
> set and (via M20) the planner/baseline work.
> **Sources:** `mockups/Sensei/lib/dojo2/dojo2-project.jsx` (Integrations · Targets ·
> Sprints tabs), `mockups/Sensei/lib/relay/relay-planner.jsx` (plan · gates ·
> decisions), `reorg.md` §9.1–9.5, `prd/modules/04-team/03-dojo-project-planning/`,
> `prd/modules/03-delivery/20-planner-and-baseline/`.
> **Companion:** [`project-management-detailed-requirements.md`](project-management-detailed-requirements.md)
> (the M16 window this table sits behind).

This document fleshes out the **tracker integration** and **gating** requirements
to the depth the mockup implies but the current T3 spec does not yet carry. It
also records three places where T3's spec **contradicts its own design source**
(§6) — those need a decision before the spec can be called complete.

---

## 0 · Resolution (user, 2026-10-21)

These settle the open questions below and override §5 where they differ.

- **External integrations are read-only; we pull.** Trackers (Jira, Azure Boards,
  GitHub Issues) and CI (GitHub Actions, Azure Pipelines) are read, never written —
  §2.6 and the §6.2 decision stand, and there is no export action.
- **The dōjō owns them.** A connection is made once per project **in the dōjō**,
  which holds the credential and mirrors the tickets, sprints and pipeline runs;
  sensei reads the mirror. Connecting from sensei writes the connection to the dōjō.
  The §5 tables and API therefore live in the **dōjō** (`dojo.*`, `/v1/t/{tenant}/…`),
  not in `sensei.*` / `/api/…` as first proposed.
- **Sensei ⇄ dōjō is two-way sync.** Projects, teams, roles and connections may be
  created or changed on either side and arrive on the other; the dōjō is the
  authority on conflict, and sensei says when a local change lost.
- **Releases and DORA** are specified in T5
  ([`prd/modules/04-team/05-releases-and-dora/`](../prd/modules/04-team/05-releases-and-dora/README.md)):
  deployments from CI with git tags as the fallback, production incidents, the four
  keys, and **release gates** — the §3 targets evaluated over a release's change set.

Plan: [`roadmap/09-dojo-expansion.md`](../roadmap/09-dojo-expansion.md) · epic #307.

---

## 1 · What the mockup actually specifies

`dojo2-project.jsx` is the source of record for the tracker + targets model. The
header comment states the rules plainly:

> * a project groups many repos (any provider, or a local path via the CLI)
> * tasks + sprints are synced **read-only** from the tracker that **owns sprints**
> * a session counts toward a task when its **branch, commit or prompt carries the
>   ticket key**
> * **targets gate on code quality only; AI-usage signals are insights, never gated**
> * targets inherit **Dōjō → Project → Sprint → Task type → Task**; most specific
>   wins
> * a sprint that misses a **blocking** target **can't close without a note**
>   (maintainer and up)

Those six lines are the load-bearing requirements. Everything below expands them.

---

## 2 · External tracker integrations

### 2.1 Supported trackers (mockup inventory)

| Tracker | Mock state | Role in the model |
|---|---|---|
| **Jira** | connected · `acme.atlassian.net · project AUTH · board "AUTH Scrum" · synced 2m ago · 142 issues` | Issues + the **sprint calendar** (owner by default) |
| **Azure DevOps Boards** | connectable · `dev.azure.com/acme-platform · project Auth Infra · area Auth\Infra · key AB#\d+` | Work items + **iterations** (alternative sprint owner) |
| **GitHub Issues** | connected · `acme/lumen-auth-sdk · issues labelled bug / feature map to task types · key #\d+ in the SDK repo only` | Issues only (no sprints) |
| **Slack** | connected · `acme.slack.com · posts to #auth-eng` | **Notifications out** (not a task source) |

GitLab and Bitbucket appear as **repo providers** (the Repos tab), not as trackers.
The mockup's model line is explicit: *"tasks and sprints sync in, read-only."*

### 2.2 One sprint owner (lead invariant)

> *"Which tracker owns sprints? One tracker drives the sprint calendar. The other
> still links tasks."*

- Exactly **one** connected tracker owns sprints per project (`jira` or `ado` in
  the mockup's segmented control).
- Every other connected tracker still contributes issues/tasks — they are simply
  not the sprint calendar.
- The owner is a project-scoped setting; changing it re-labels sprints without
  dropping task links.

### 2.3 Connection model

Per tracker, the connect modal carries:

| Field | Example | Notes |
|---|---|---|
| Organization / base URL | `https://dev.azure.com/acme-platform` | OAuth host or PAT target |
| Project | `Auth Infra` | tracker project key/name |
| Area / scope | `Auth\Infra` (ADO) · board (Jira) | optional narrowing |
| Ticket-key pattern | `AB#\d+` (ADO) · `AUTH-\d+` (Jira) | per-tracker, per-repo-capable |
| Sprout owner | Keep Jira / Use ADO iterations | §2.2 |

- **Auth:** OAuth where the tracker offers it, PAT as the documented fallback.
- **Scope:** a tracker connection is **project-scoped**, not org-global — a client
  engagement uses the client's tracker and gates (T3 "client work" invariant).
- **Repo-scoped key pattern:** GitHub Issues is explicitly *"#\d+ in the SDK repo
  only"* — the key regex is per-repo, because two repos can number issues the same.

### 2.4 Task-type mapping

Tracker issue types map onto sensei's task types, shown as chips:

```
Story → feature     Bug → bug     Task → chore     label: migration → migration
```

- Mapping is **declared, not inferred** — the user sees each arrow and can change it.
- Unmapped types fall back to `chore` (safe default; never silently dropped).
- The task type feeds **target overrides** (§3.4) — e.g. `migration` may require
  90% coverage where the project requires 80%.

### 2.5 Session ↔ task linkage

> *"a session counts toward a task when its branch, commit or prompt carries the
> ticket key"* — three switchable sources, all on by default in the mockup:

| Source | Chip | Meaning |
|---|---|---|
| `branch` | branch name | `fix/AUTH-420-session-store` links to AUTH-420 |
| `commit` | commit message | `AUTH-420: migrate store` links |
| `prompt` | session prompt | the user typed the key while working |

- **Unlinked sessions don't count toward any task.** The sprint table renders the
  remainder: *"6 sessions this sprint carried no ticket key, so they count toward
  no task"* with a **Review unlinked** action.
- Linkage is a **read** of existing session metadata (M3 capture) — no new capture.
- One session may carry several keys (multi-task session) → it counts toward each.

### 2.6 Sync semantics (read-only)

- **Sync in is automatic and read-only.** Status shows `connected · synced 2m ago`,
  with a manual **Sync now**.
- **Sprints/tasks are never written back by sync.** Closing a sprint
  *"Closes in Sensei; Jira's sprint is untouched."*
- Exporting status to the tracker, if ever offered, is a **separate, explicit
  action** — never a side effect of sync (this is the T3 "read-only" wrong gate;
  see §6.2 for the spec's current contradiction).
- Staleness is a first-class state: `connected` / `stale` / `error`, with the
  last-sync age always visible.

### 2.7 Notification hooks (Slack)

Three toggleable events, each with a stated payload:

| Event | Fires when | Payload |
|---|---|---|
| Target missed mid-sprint | a **blocking** target is missed | posted **once per target**, with the tasks behind it |
| Sprint closed with a note | a sprint closes carrying a note | the note and the misses it explains |
| Session waiting on a gate > 30m | a session blocks on an approval | **who** it is waiting on |

These are delivery-plane notifications that reuse the relay/approve channel; they
are not a new messaging system.

---

## 3 · Gating: the target model

### 3.1 Anatomy of a target (mockup fields)

```js
{ id, label, short, cmp, unit, dojo, project, mode, source, hist, overrides }
```

| Field | Values | Meaning |
|---|---|---|
| `cmp` | `≥` · `≤` · `=` | comparator (`at least` / `at most` / `exactly`) |
| `dojo` | number / null | the **Dōjō-wide** default value |
| `project` | number / null | the project's tightened value; `null` = *inherits* |
| `unit` | `%` / `""` | display unit |
| `mode` | `block` · `warn` · `off` | **what a miss does** |
| `source` | string | where the reading comes from (`Code health · cycles`, …) |
| `hist` | number[] | per-sprint history for the trend spark |
| `overrides` | `[{scope, on, value, note?}]` | narrower-scope exceptions |

Effective value: `project ?? dojo` (`pjEff`). A project that shows `null` renders
**"inherits"** — inheritance is visible, never silently copied.

### 3.2 The inheritance ladder (a named requirement)

```
Dōjō → Project → Sprint → Task type → Task          (most specific wins)
```

- Set once at the Dōjō; **tighten** at the project; **override** for one sprint,
  a task type, or a single ticket.
- An override is a first-class row (`{scope, on, value, note}`), rendered as a chip
  and removable.
- Example from the mockup: `cx > 10` project target `≤ 2`, with
  `[Task type migration → no, that's cov]` and `[Task AUTH-420 → 4 · "legacy session
  store"]` overrides.
- **Most specific wins** — mirrors the rule/memory scope tiebreak already shipped in
  M13/M9; the resolver is the same idea on a different axis.

### 3.3 The seven code-health targets (mockup, mapped to today)

These are the mockup's seven and the union of what `reorg.md` §9.2 already
assessed. **All are code-derived** — that is what makes them gate-eligible.

| # | Target | Source | Buildable today? | Blocker |
|---|---|---|---|---|
| 1 | Hard-rule violations at merge | constitution checks (`rule_check_runs`, D-CHECKER) | ✅ | — |
| 2 | Coverage on guarded paths | `coverage` (lcov), path-scoped | ◐ | needs a path-scoped read |
| 3 | New layer violations | architecture metrics | ✗ | #156 |
| 4 | New dependency cycles | architecture metrics | ✗ | #156 |
| 5 | Modules in the zone of pain | architecture metrics (instability × abstractness) | ✗ | #156 |
| 6 | Changed functions over complexity 10 | qlty complexity, per-changed-function | ◐ | needs a per-function read |
| 7 | New high-severity anti-patterns | anti-pattern detector | ✗ | `patterns/` absent |

**2 of 7 buildable now, 2 need a scoped read, 3 are blocked on the
architecture-metrics family (#156), itself blocked on edge resolution
(#146/#151/#152).** This is the single most important honesty constraint in T3 —
it must not be promised otherwise.

### 3.4 Three modes per target (mockup)

`block` (default for the strict ones) · `warn` · `off`, chosen per target as
*"When a sprint misses it"*. Renderers:

- `block` → *"blocks close"* with a lock icon.
- `warn` → *"warns"* with a warning triangle.

The mockup's default split: 5 block, 2 warn (`pain`, `anti`). A miss on any
`block` target means the sprint **cannot close without a note**.

### 3.5 Gate vs improvement split (lead, from reorg §9.3)

**Gate-eligible** = code-derived **and** delta-scoped:

- hard-rule violations **introduced** this sprint;
- coverage on **guarded/changed** paths;
- new layer / cycle / anti-pattern violations **introduced**;
- complexity of **changed** functions.

**Improvement-only** (trend, never a gate):

- everything behavioural — `ftr`, `rework_ratio`, `spec_depth`, `tokens_*`,
  `cost_per_result`, … ;
- everything project-level and absolute — `duplication_ratio`, `module_quality`,
  `churn_rate`, …;
- weight-0 / neutral metrics.

**Rule for the spec:** *a gate must be scoped to the sprint's own delta or to a
path the sprint chose to guard. Absolute project-level ratios are
improvement-only.* An inherited low-quality codebase therefore fails **nothing** on
day 1 — it starts a ratchet.

**Insights are never gates.** The mockup shows four tracked-but-never-gated
signals: first-try rate, corrections/session, AI-assisted share of commits, gate
wait time — *"These describe how sessions went, not what was merged, so they can't
be a merge or sprint gate."* This is the model line *"targets gate on the code;
insights only describe how it was made"* made concrete.

### 3.6 Own-baseline ratchet (from reorg §9.4)

Fixed absolute targets punish adoption, so two fairness mechanisms:

1. **Delta gates** — count *new* violations, not totals (§3.5).
2. **Own-baseline ratchets** — for project-level readings that cannot be
   delta-scoped, compare the repo against **its own history**: *"no worse than the
   trailing baseline."*

A ratchet **advances only when the reading improves *and holds*** — a single spike
never moves it. The mockup's per-sprint `hist` arrays are exactly the series a
ratchet reads.

> **Ratchet scope is the open question.** `reorg.md` §9.4 names the ratchet for
> *project-level* metrics such as `graph_confidence` / `public_surface_ratio`.
> The T3 spec currently applies it to **FTR** — see §6.1; that is the one place
> the requirements need a decision, not more prose.

---

## 4 · The sprint scorecard

### 4.1 Surfaces

- **Sprint bar** — chips per sprint (`S18…S23`), the active one dotted; header shows
  `dates · day 5 of 10 · from Jira` and `N of 7 targets missed` (or *every target
  met*).
- **Scorecard** — one card per target: value, `target ≥ 80`, `on target` / `missed ·
  blocks` / `missed · warn`, plus the trend spark with the target band shaded and
  off-target points accented.
- **Task matrix** — rows are tickets; columns are the seven targets plus
  `Ticket · Task · Type · Sess. · Status`. A cell is accented when that task pushes
  the sprint toward a miss. Footer states unlinked sessions and *"Pain is read per
  module, not per task."*
- **Archived sprints** keep the scorecard but drop the per-task matrix.

### 4.2 Close-with-note (blocking miss)

- Closing a sprint with a blocking miss requires a note (≥ 12 chars in the mockup)
  and offers **Post to Slack**.
- The note is kept with the sprint and shown on its scorecard next to the misses.
- **Maintainer and up** may close. *"Closes in Sensei; Jira's sprint is untouched."*
- A note carries what happened **and what carries over** (e.g. *"AUTH-415 and
  AUTH-420 each added an upward call…; both carried to Sprint 24 with a refactor
  ticket"*).

### 4.3 Relay projection (plan view)

`relay-planner.jsx` projects the same plan into the away-from-keyboard surface:
phases hold **features / checkpoints / gates**; a `gate` item renders the lock and
*"decision waiting — needs you"*. Decisions are 3–4 options + free reply, and
**other tracks keep running** while one waits. This is the read side of the same
gating model — no separate gate semantics.

---

## 5 · Data model & API shape (proposed — moves to the dōjō, see §0)

> **Nothing here is built.** `sensei.runs` / `sensei.run_events` exist; plan,
> baseline, target and tracker tables do not (C-1, T-1).

### 5.1 Tables (proposed)

| Table | Holds | Notes |
|---|---|---|
| `sensei.trackers` | connection per project: kind, host, project/area, PAT/OAuth ref | secrets in the vault, not the row |
| `sensei.tracker_links` | repo → ticket-key regex, sync cursor, last-sync status | per-repo key pattern (§2.3) |
| `sensei.tracker_tasks` | tickets synced in: key, type, title, status, sprint, assignee | **read-only mirror** |
| `sensei.sprints` | name, dates, state, owner-tracker, closing note | one owner (§2.2) |
| `sensei.plan_targets` | target rows incl. `dojo`, `project`, `mode`, `source` | the ladder base |
| `sensei.plan_target_overrides` | `{target_id, scope, on, value, note}` | §3.2 |
| `sensei.plan_baselines` | `baseline_value` + ratchet history per target | §3.6, C-1 |
| `sensei.session_task_links` | session → task, via branch/commit/prompt | §2.5 |

### 5.2 API sketch

```
GET   /api/projects/{id}/trackers                      # connection + status
POST  /api/projects/{id}/trackers/{kind}/connect
PATCH /api/projects/{id}/trackers/{kind}               # mapping, key regex, owner
POST  /api/projects/{id}/trackers/{kind}/sync          # explicit, read-only

GET   /api/projects/{id}/targets                       # resolved ladder + effective
PATCH /api/projects/{id}/targets/{targetId}            # project value / mode
POST  /api/projects/{id}/targets/{targetId}/overrides

GET   /api/projects/{id}/sprints
GET   /api/projects/{id}/sprints/{sid}/scorecard
POST  /api/projects/{id}/sprints/{sid}/close           # {note, postToSlack}
GET   /api/projects/{id}/sessions/unlinked
```

---

## 6 · Reconciliation — where T3's spec contradicted its source

> **Resolved 2026-10-14** — decided in [`decisions.md`](../decisions.md)
> (“T3 tracker & gating reconciliation”) and folded into
> [`prd/modules/04-team/03-dojo-project-planning/`](../prd/modules/04-team/03-dojo-project-planning/README.md).
> The findings are kept below as the record of what was wrong.

### 6.1 The seven targets don't match, and FTR is gated

T3's spec lists a different seven from `reorg.md` §9.2 (the mockup's seven):

| # | T3 spec (`README.md` §"The 7 gating targets") | reorg §9.2 (mockup) |
|---|---|---|
| 1 | FTR ≥ baseline + δ — **gate** | Hard-rule violations at merge |
| 2 | No new hard blocks | Coverage on guarded paths |
| 3 | No new semantic review flags | New layer violations |
| 4 | Coupling ≤ baseline | New dependency cycles |
| 5 | Cohesion ≥ baseline | Modules in the zone of pain |
| 6 | Cycles = 0 | Changed functions over complexity 10 |
| 7 | Doc drift ≤ baseline (improvement) | New high-severity anti-patterns |

Two problems:

1. **FTR as a gate contradicts §9.3**, which lists `ftr` explicitly under
   **improvement-only** — *"everything behavioural … ftr … never a gate."* T3's
   target #1 and its Done gate (*"The 2 buildable gates (FTR, hard blocks)"*) are
   therefore **wrong against the design source**.
2. **Coupling / cohesion are not the mockup's targets.** The mockup gates on
   layer violations, cycles, zone-of-pain and cyclomatic complexity; coupling and
   cohesion are broader *architecture metrics* (`#156`) that may be
   ratchet-only, not sprint gates.

**Decision (2026-10-14):** T3 gates on the **mockup's code-health set**
(delta-scoped, §3.5). **FTR is an insight, never a gate**, and coupling/cohesion
are not sprint gates (absolute project-level ratios are improvement-only). The
T3 spec's target table and Done gate were rewritten accordingly.

### 6.2 "Read-only" vs "exports run status back"

T3's gaps feature contains **both**:

- *"tracker adapters are read-only — they never write back without explicit
  export"* and T3's Wrong gate *"A tracker adapter claims to write back"*;
- yet also *"the Jira adapter **exports run status back to issues**"* and *"work
  item statuses are updated in ADO."*

The mockup is unambiguous (*"synced read-only"*, *"Closes in Sensei; Jira's sprint
is untouched"*). **Decision (2026-10-14): read-only means read-only** — the export
scenarios were removed and replaced by a single “adapters never write back”
scenario; there is no export action in scope.

### 6.3 The inheritance ladder, modes and Slack are missing

T3's spec has **no** mention of:

- the `Dōjō → Project → Sprint → Task type → Task` ladder and overrides (§3.2);
- the `block` / `warn` / `off` per-target modes (§3.4);
- Slack notifications on miss / close / gate-wait (§2.7);
- unlinked-session handling (§2.5);
- one-sprint-owner (§2.2);
- the task matrix / scorecard / close-with-note shape (§4).

These are all first-class in the mockup and in `reorg.md` §9.1. **Decision
(2026-10-14): all added** — the ladder, the three modes, the Slack hooks, unlinked
sessions, one-sprint-owner and the close-with-note rule are now sections in the T3
spec.

---

## 7 · Gaps & roadmap routing

| Gap | What it blocks | Priority | Route |
|---|---|---|---|
| **T-1** | tracker adapters (Jira/ADO) exist only in the mockup | P1 | [`roadmap/06-gaps.md`](../roadmap/06-gaps.md) |
| **D-6** | tracker adapters named for M20's out-of-scope list → T3 | — | [`reorg.md`](../reorg.md) §12 |
| **D-7** / `#156` | architecture-metric targets (coupling/cohesion/layers/cycles/pain) | — | [`reorg.md`](../reorg.md) §12 |
| **C-1** | planner persistence + baseline + ratchet (M20) | — | [`reorg.md`](../reorg.md) §12 |
| **M-7/M-8/M-9** | metric `gating` field, coverage-gated presentation, per-metric design record | P1 | [`roadmap/06-gaps.md`](../roadmap/06-gaps.md) |
| G-2 | indexer ignore skews the code-health readings the gates consume | P2 | [`roadmap/06-gaps.md`](../roadmap/06-gaps.md) |

**Roadmap resolution tasks to add** (per the ACP-style gap resolution the roadmap
carries):

1. **T-1a** — Jira read-only adapter (projects, issues, sprints, key mapping).
2. **T-1b** — ADO read-only adapter (projects, work items, iterations, area path).
3. **T-1c** — one-sprint-owner + task-type mapping + per-repo key pattern.
4. **T-1d** — session↔task linkage (branch/commit/prompt) + unlinked review.
5. **T-1e** — target ladder resolver (Dōjō → … → Task) + `block`/`warn`/`off`.
6. **T-1f** — sprint scorecard + close-with-note + Slack hooks.
7. **M-7** — metric `gating` field (`none`/`delta`/`ratchet`) + master-detail (M8).
8. **C-1** — planner persistence → budgets → ratchet (M20).
9. **#156** — architecture metrics (layers · cycles · pain · coupling/cohesion).
10. **Scoped reads** — path-scoped coverage, per-changed-function complexity.

---

## 8 · Acceptance criteria

| Scenario | Pass condition |
|---|---|
| Connect Jira | OAuth/PAT completes; status shows `connected · synced …`; 142 issues import read-only |
| Connect ADO | Org/project/area + `AB#\d+` accepted; iterations import |
| Sprint owner switch | Switching to ADO iterations re-labels sprints; task links survive |
| Sync never writes | After a sync, the tracker is unchanged; close-sprint leaves Jira's sprint untouched |
| Session links by key | A session whose branch/commit/prompt carries `AUTH-420` counts toward AUTH-420 |
| Unlinked sessions | The count is shown and *Review unlinked* lists them |
| Inherit a target | A project target shows `inherits` when unset; setting a value overrides the Dōjō default |
| Override a task type | `migration` override applies only to migration tasks; other tasks keep the project value |
| Miss a blocking target | Sprint cannot close without a note; closing offers Post to Slack |
| Miss a warn target | Surfaces as a warning; sprint closes normally |
| Insight is never a gate | FTR / corrections / AI-share / gate-wait cannot be set to `block` |
| Ratchet holds | A one-sprint FTR spike does not advance the baseline; a sustained rise does |
| Client engagement | The client's tracker and gates are used; the employer's mandatory rules remain a floor |

---

## 9 · Related

- [`project-management-detailed-requirements.md`](project-management-detailed-requirements.md) — the M16 window; its Integrations tab is this doc's §2.
- [`diagrams-zoom-pan-detail-requirements.md`](diagrams-zoom-pan-detail-requirements.md) — the diagram surfaces the scorecard's trends sit beside.
- [`../prd/modules/04-team/03-dojo-project-planning/README.md`](../prd/modules/04-team/03-dojo-project-planning/README.md) — the T3 spec this feeds.
- [`../prd/modules/03-delivery/20-planner-and-baseline/README.md`](../prd/modules/03-delivery/20-planner-and-baseline/README.md) — `PlanGraph`, baseline, ratchet (C-1).
- [`../prd/modules/03-delivery/16-project-window/README.md`](../prd/modules/03-delivery/16-project-window/README.md) — the Tasks tab (unbuilt).
- [`../reorg.md`](../reorg.md) §9 — the canonical gate/improvement + ratchet design.
- [`../mockups/Sensei/lib/dojo2/dojo2-project.jsx`](../mockups/Sensei/lib/dojo2/dojo2-project.jsx) · [`relay-planner.jsx`](../mockups/Sensei/lib/relay/relay-planner.jsx) — visual source (never edited).
