---
name: Docs reorg — step 1 proposal (agreed)
description: Agreed structure for docs/prd + docs/roadmap, .feature scenarios, SVG plan, rename/move map, gap register
date: 2026-10-13
status: agreed — decisions folded 2026-10-13
---

# Docs reorg — step 1 proposal (agreed)

> **Agreed 2026-10-13.** The structure below is signed off and the open
> questions are answered (§16). No PRD or roadmap documents have been written
> yet — execution is phased (§13): Phase 1 scaffolds the tree, then module
> specs land in priority order.
>
> **The problem being solved:** today the "what are we building and how far
> along is it" question is answered by ~5 competing sources (`vision.md`,
> `objectives.md`, `features/coverage-map.md`, `plan/README.md`,
> `roadmap/phases.md`), plus 186 files in `spec/` that the repo itself calls
> *legacy/transitional*, plus ~30 dated snapshots. One authority per question is
> the goal.
>
> **The second problem:** we have a large built surface (daemon, MCP, app,
> Dōjō) and no document that says, per module, *here is what "done" means and
> here is which parts of it are actually done*. That is what the PRD + roadmap
> pair, with Gherkin scenarios, is for.

---

## 0 · How to read this

| § | What it answers |
|---|---|
| 1 | What is in `docs/` today, and why it is hard to use |
| 2 | The target set: `docs/prd` + `docs/roadmap` + what stays |
| 3 | `docs/prd` outline — the vision README and the module-spec template |
| 4 | The core module list (the spine both docs are organised by) |
| 5 | `docs/roadmap` outline — prioritisation + status + module sections |
| 6 | How Gherkin scenarios fit, and how they surface gaps |
| 7 | Which diagrams become SVGs (and why not all of them) |
| 8 | Rename / move plan (current → target → action) |
| 9 | The new Dōjō planning slice — integrations + target gate, and the metric gating map |
| 10 | Platform & distribution: static website, Homebrew (macOS), Linux/Windows |
| 11 | Performance register (present pain + future risks) |
| 12 | Gap register (consolidated, deduplicated) |
| 13 | Migration phasing |
| 14 | Open questions — answered (§16) |
| 15 | Approval checklist |
| 16 | Decisions (2026-10-13) — answers, folded in |

### 0.1 Decisions at a glance

Ten decisions were taken 2026-10-13 (§16 has the full text). The ones that
changed the structure:

- **29 modules across 5 planes; planes are first-class** (nav README per plane). *(The proposal's summary said "23" — a miscount; the approved list is M1–M22 · T1–T4 · P1–P3 = 29. See §4.)*
- **A root `README.md` in every folder** — GitHub renders the folder landing page.
- **Scenarios are `.feature` files**, referenced from the module markdown.
- **Roadmap = 5 plane files** (+ `06-gaps.md`).
- **`history/` tree retained** for dated snapshots.
- **Website = `P2 · Website`** (marketing site).
- **Distribution targets macOS, Linux and Windows** — design-ready, built in parallel.
- **Metrics need a master-detail model** with explicit gating eligibility, and graphs need a legible level-of-detail.
- **`backlog.md`'s hand-maintained table is dropped**; work is tracked by GitHub issue reference.
- **Personas are authored in this pass**, including management and client/engagement viewpoints.

---

## 1 · What `docs/` looks like today

### 1.1 Inventory

366 markdown files, 703 files total. The top level and the working folders:

| Folder | Files | What it is | Health |
|---|---:|---|---|
| `vision.md` + `objectives.md` | 2 | the why + measurable outcomes | **canonical, good** |
| `personas/` | 1 | a stub README (personas never authored) | **gap** |
| `journeys/` | 4 | distilled flows (Mermaid) over the mockups | good |
| `roadmap/` | 2 | 6-phase roadmap + index | **stale** (points at `plan/` + `spec/EXECUTION-PLAN.md`) |
| `features/` | 13 | the "feature dossier" spec + `coverage-map.md` | **split authority** (see 1.2) |
| `spec/` | 186 | per-screen/per-pipeline build specs | **declared legacy/transitional** |
| `design/` | 30 | cross-cutting design + design system | mixed; dated files mixed in |
| `architecture/` | 19 | the "how", per surface | **canonical, good** |
| `mockups/` | ~300 | the visual source of truth (HTML/JSX/dc) | **canonical, must not move** |
| `plan/` | 45 | living gap analysis (`README.md`) + dated build plans | good + transient mixed |
| `analysis/` | 37 | dated deep-dives | **dated snapshots** |
| `blueprints/`, `plans/`, `reports/`, `verification/`, `database/` | 13 | more design/dated material | scattered |
| `requirements/` | 3 | already absorbed (left as redirects) | **dead** |
| top-level loose files | 8 | `backlog.md`, `decisions.md`, `CHECKPOINT.md`, `production-readiness.md`, `svelte5-runes.md`, `transcript-paths.md`, `README.md`, `.gitignore` | mixed |

### 1.2 The three conflicts that make it unusable

1. **Three declared sources of truth.**
   - `docs/README.md`: *"a feature's truth lives in its `features/<name>/` dossier"*.
   - `features/README.md`: *"`coverage-map.md` is the authoritative built-vs-pending snapshot … treat it as the source of truth where this index disagrees."*
   - `spec/README.md`: *"`spec/` is transitional — folds into `features/*/design.md` or `architecture/`."*
   → Nobody can answer "is feature X built?" without reading three docs and guessing which wins.

2. **Duplicated content, five ways.** The same fact is written in `features/`, `spec/`, `design/`, `architecture/`, and `plan/`. Measured duplicates: front door ×5, relay ×8, governance ×5, metrics ×12, code-graph ×15. Each copy drifts independently.

3. **Dated snapshots mixed with canon.** `plan/README.md` says evidence date 2026-07-14; `design/phases.md` is 2026-07-20; `coverage-map.md` is 2026-07-25; `production-readiness.md` is 2026-09-25. A reader cannot tell which one is *now* without checking the frontmatter date — and some don't carry one.

### 1.3 What is already right (and must survive)

- `vision.md` / `objectives.md` — the front door, and genuinely good.
- `architecture/` — the "how", already well-structured.
- `journeys/` — the distilled paths, small and clear.
- `mockups/` — the visual source of truth. **We copy from these; we never restructure them.**
- `backlog.md` — the open-work issue index.
- `production-readiness.md` — measured, quote-the-query engineering truth.
- The **Gherkin house style** already present in `spec/` (`# pinned: <test>`, `NO TEST` markers) — see §6.

### 1.4 What is measurably built (the baseline the roadmap must reflect)

From `features/coverage-map.md` + the code surface: ~130 daemon routes · 37 MCP tools · 29 Dōjō `/v1` routes · 45 app screens · 7 dojo2 pages (~30 sections) · 123 DB tables across 8 schemas · ~3,317 senseid tests. Graph 40.3% resolved (1,627,350 resolved / 2,415,191 unresolved). Logs = 25 GB of a 37 GB DB.

This is the number that makes the current docs a liability: the product is *mostly built*, and the docs still read like a greenfield backlog.

---

## 2 · The target documentation set

Two new authorities, three retained sources, everything else folded or aged out.

```
docs/
  README.md                    # hub: "what are you looking for?" → the 4 doors below
  prd/                         # ← NEW. WHAT we are building. The source of truth.
    README.md                  #   vision + what is to be built (front door)
    01-vision.md               #   moved from docs/vision.md
    02-objectives.md           #   moved from docs/objectives.md
    03-personas.md             #   authored in this pass (decision 10)
    modules/
      README.md                #   module index, grouped by plane
      <plane>/README.md        #   plane nav (Foundation · Insight · Delivery · Team · Platform)
      <module>/README.md       #   per-module spec (see §3)
      <module>/scenarios/*.feature   #   Gherkin — referenced from README.md (decision 3)
    assets/diagrams/*.svg      #   authored, self-contained
  roadmap/                     # ← NEW. WHERE we are. Status + priority + sequencing.
    README.md                  #   how to read; status model; priority model
    <plane>.md                 #   module sections — 5 files, 0N- prefix (decisions 4, 11)
    06-gaps.md                 #   consolidated gap register (§12)
  architecture/                # retained: HOW. Cross-linked from every module spec.
    README.md                  #   root README per folder (decision 2)
  journeys/                    # retained: the paths. Distilled from mockups.
  design/                      # retained, trimmed: cross-cutting design + design system.
  mockups/                     # retained UNTOUCHED: visual source of truth.
  history/                     # ← NEW: dated snapshots (moved, not deleted)
    README.md                  #   the readable index of superseded material
  decisions.md                 # retained: the decision log
  backlog.md                   # retained: OPEN work only — live GitHub issue references (decision 9)
```

**Root `README.md` in every folder** (decision 2). GitHub renders a folder's
`README.md` as its landing page, so each of `prd/`, `prd/modules/`, each plane,
each module, `roadmap/`, `architecture/`, `journeys/`, `design/` and `history/`
carries one. This is the navigation mechanism, not decoration.

**Planes are first-class** (decision 1). A plane is not just a heading inside a
file — it is a folder with its own README, so the module list is browsable at two
levels: plane → module.

### 2.1 Precedence — stated once, in `prd/README.md`

This ends conflict 1.2:

| Question | Authority |
|---|---|
| What are we building, and why? | `prd/` |
| Is it built? How far? What's next? | `roadmap/` |
| How is it built? | `architecture/` |
| What does it look like? | `mockups/` |
| What is open right now? | `backlog.md` (GitHub issues) |

If two disagree, the higher row wins; the lower one is the bug.

### 2.2 What happens to each existing folder

| Current | Becomes |
|---|---|
| `vision.md`, `objectives.md` | moved into `prd/` (content preserved) |
| `features/<name>/` | folded into `prd/modules/<module>/` (content preserved) |
| `features/coverage-map.md` | folded into `roadmap/` status tables; file stamped `superseded` and moved to `history/` |
| `spec/**` (186 files) | folded into `prd/modules/<module>/{spec.md,scenarios}` **as each module is next touched**; stamped `superseded`; moved to `history/` |
| `plan/README.md` | rewritten as `roadmap/README.md` + `roadmap/06-gaps.md` |
| `plan/*` dated | moved to `history/` |
| `roadmap/phases.md` | superseded by `roadmap/` module sections; moved to `history/` |
| `requirements/` | deleted (already absorbed; content lives in `prd/`) |
| `analysis/`, `reports/`, `verification/`, `plans/`, `blueprints/` | moved to `history/` (a few promoted into `prd/`/`architecture/` first) |
| `design/` | retained; dated files moved to `history/` |
| `personas/` | content authored into `prd/03-personas.md`; folder removed |
| `database/` | retained (DB model docs); cross-linked from `architecture/data.md` |
| `production-readiness.md` | moves under `roadmap/` as the measured-quality annex |
| `CHECKPOINT.md`, `svelte5-runes.md`, `transcript-paths.md` | `history/` or `architecture/` per topic |

**Rule:** nothing is deleted. Content is preserved, then the superseded file is stamped and moved to `history/` (git history is the backstop; `history/` is the readable index of it).

---

## 3 · `docs/prd` outline

### 3.1 `prd/README.md` — the front door

One page, answering: *what is this, what does it do, what are we building.* Structure:

1. **One paragraph** — sensei is the observer/mentor of how a developer works with AI assistants (reuse `01-vision.md` §One paragraph, tightened).
2. **North star — FTR** (first-turn resolution), one paragraph.
3. **The core loop** — capture → graph → analyze → learn → deliver → measure. One SVG (§7).
4. **The four segments + Dōjō + Relay** — one SVG of the journey (§7).
5. **What is to be built** — the module table (from §4), each row linking to its module spec, with a one-line "what done means".
6. **What sensei is *not*** — carried from `01-vision.md`.
7. **Themes** — the six non-negotiables, kept as the design contract.
8. **Precedence + how this doc set works** (§2.1).

### 3.2 `prd/01-vision.md` and `prd/02-objectives.md`

Moved verbatim from `docs/vision.md` / `docs/objectives.md`. They are good. Only edits: cross-links point into the new tree; the "read next" links point at `prd/` and `roadmap/`.

### 3.3 `prd/03-personas.md` — authored in this pass (decision 10)

Authored now, not stubbed. Each persona carries: goals, the job-to-be-done, what
"good" feels like, and **which modules serve it** (the `Serves:` line on every
module spec points back here).

Core set:

| Persona | Scope | Notes |
|---|---|---|
| Solo developer | personal | the wedge; the free local loop |
| Team developer | a Dōjō membership | inherits org knowledge, contributes upstream |
| Maintainer | a Dōjō scope | triage → approve → distribute |
| Org admin | one org | identity, members, policies, audit |
| Engagement / client lead | client engagements | confidentiality, universal strip, audit |

**Newly required viewpoints (the multi-user, multi-team case):**

| Persona | Scope | Why it is needed |
|---|---|---|
| **Manager / delivery lead** | across pods, projects and teams | needs a roll-up (status, risk, throughput) without reading code; the customer for T3's sprint/scorecard and for the birds-eye graph view (§9.7) |
| **Client / project stakeholder** | one project or engagement | wants project-level progress + quality posture with confidentiality held; reads the same data as the lead through a narrower lens |

**Naming:** the grouping vocabulary (pods / swimlanes / teams) is deliberately
left open — the model is *a person can hold many scopes*, and the label is a
display concern, not a schema one.

**Not a persona:** an AI assistant is a *delivery channel* (M14), not a human
whose goals we design for — but its constraints (context budget, tool surface)
are captured as acceptance criteria, not as a persona.

### 3.4 `prd/modules/<module>/README.md` — the module spec template

Every module spec answers the same questions, in this order:

```markdown
# <Module name>

**Plane:** Foundation | Insight | Delivery | Team | Platform
**Status:** built | partial | mock | designed | gap   (mirrors roadmap; one line)
**Serves:** <personas>
**Depends on:** <modules>

## Purpose
One paragraph: the user-visible outcome this module produces. Not a feature list.

## In scope / out of scope
The boundary. "Out of scope" is mandatory — it is what stops scope creep.

## Surfaces
The screens, routes, tables, MCP tools, CLI commands this module owns.
(These are the *wiring*: `app/src/routes/...`, `crates/senseid/src/api/...`, `dojo/...`.)

## Data invariants
What must be true in the DB before this module can be correct. Named tables/joins.

## Signals shown
For any user-visible number: shape, meaning, worked example (reuse the `spec/` template).

## Done gate
The English claim we make when this module is finished.

## Wrong gate
What would make this embarrassing / silently wrong.

## Scenarios
`.feature` files live in `scenarios/` beside the spec and are linked here. See §6.

## Open gaps
Pointer to `roadmap/06-gaps.md` entries for this module.

## Related
Architecture · journeys · mockups · decisions
```

This is a merge of the best of three existing templates: `spec/README.md`'s five questions (Purpose / Data invariants / Signals / Done gate / Wrong gate), `features/README.md`'s dossier shape, and the explicit *surfaces* list that makes a gap machine-checkable.

### 3.5 Feature inclusion — which existing features land where

Every feature in `features/README.md` + `coverage-map.md` maps to exactly one module:

| Existing feature / surface | PRD module |
|---|---|
| Setup, bootstrap, health, provisioning | **M1 · Bootstrap & Provisioning** |
| Preferences, config, assistants, providers, roots, inference settings | **M2 · Setup & Preferences** |
| hooks, sessions, transcripts, turns, adapters (Claude/Cursor/Copilot/Codex/…) | **M3 · Capture & Sessions** |
| code graph, indexer, languages, watcher, scan/reconcile, atlas data | **M4 · Code Graph & Indexing** |
| gateway, inference chains, MoE/consensus, budget/circuit | **M5 · Inference Gateway** |
| DDL, migrations, stores, entity-access model | **M6 · Storage & Data Model** |
| analyzer, signals, FTR derivation, session_outcomes | **M7 · Analyzer & Signals** |
| metrics registry, ratings, project health | **M8 · Metrics & Health** |
| memories, promotion ladder, knowledge plane | **M9 · Memory & Knowledge** |
| patterns, anti-patterns, conventions | **M10 · Patterns & Conventions** |
| traceability, doc-drift, symbol_names | **M11 · Traceability & Doc Drift** |
| libraries detect/wrap/query/watch, lib-docs | **M12 · Libraries** |
| rules, constitution, rule packs, stance, injection, governance | **M13 · Governance & Rules** |
| MCP tools, context delivery, context_pack, layered context | **M14 · MCP & Context Delivery** |
| Today, projects, sessions, insights, memories, libraries, instruments, impact, upgrades, collective, dojo connections | **M15 · Observatory** |
| project window (overview, sessions, memories, traceability, libraries, instruments, patterns, impact, about) | **M16 · Project Window** |
| Atlas (code graph view) | **M17 · Atlas** |
| Instruments (playground, replay, health) | **M18 · Instruments** |
| insights, recommendations, impact verdicts, consolidation | **M19 · Insights & Impact** |
| planner, baseline/DoD, capabilities contract | **M20 · Planner & Baseline** |
| run engine, gates, remote status, relay PWA | **M21 · Relay** |
| agentic execution / coordinator (future) | **M22 · Agentic Execution** |
| Dōjō federation, identity, memberships, policies, audit, rule packs | **T1 · Dōjō Core** |
| Dōjō consoles (developer/maintainer/admin/lead), dojo2 | **T2 · Dōjō Consoles** |
| Dōjō project planning — integrations + target gate | **T3 · Dōjō Project Planning** *(new — §9)* |
| billing, seats, plans, metering | **T4 · Billing & Seats** |
| Homebrew, installers, service management | **P1 · Distribution** |
| marketing website | **P2 · Website** |
| CI, test harnesses, dev container | **P3 · Harness & CI** |

*(Full module definitions in §4.)*

---

## 4 · The core module list

Organised into five planes. This list is the spine: **one module = one PRD spec
= one roadmap section = one `.feature` set.**

**A plane is a folder, not a heading** (decision 1). `prd/modules/<plane>/README.md`
is the plane's landing page — what the plane owns, its modules, its reading
order; `roadmap/<plane>.md` is its status page.

### 4.0 Scope added by the 2026-10-13 decisions

These are existing modules whose *scope grew*; none adds a 24th module:

| Module | Added scope | Decision |
|---|---|---|
| M2 · Setup & Preferences | personal preference for **icon style (kanji ⇄ solar)** and the wider preference surface; the current app is kanji-only — rokkit's data-driven components already support the toggle | 10 |
| M3 · Capture & Sessions | **transcript scanning** review; **Claude "mods"** (supersedes the old plugin+hooks install); **adapter expansion — Claude and OpenCode are top-2, others later**, designed for extension | 10 |
| M5 · Inference Gateway | **inference-pipeline review** as part of the same transcript/capture pass | 10 |
| M8 · Metrics & Health | **master-detail metric model** + per-metric **gating eligibility** with design/config coverage (§9.6) | 8 |
| M17 · Atlas | **level-of-detail (LOD) rendering** — birds-eye → group/module → detail, each level readable without 1000% zoom (§9.7) | 8 |
| P1 · Distribution | **macOS + Linux + Windows**, design-ready, built in parallel | 7 |
| P2 · Website | confirmed marketing site | 6 |
| `backlog.md` | hand-maintained open-issue table dropped → live GH references | 9 |

### Foundation plane — the floor everything stands on
| ID | Module | One-line "done" |
|---|---|---|
| M1 | Bootstrap & Provisioning | Fresh install reaches all-green with no manual toolchain work |
| M2 | Setup & Preferences | Every default is adjustable in a searchable surface |
| M3 | Capture & Sessions | Every assistant session is captured with turns, tools and outcome |
| M4 | Code Graph & Indexing | A repo scans to a resolved, queryable graph, incrementally |
| M5 | Inference Gateway | Every LLM step routes to a local or paid model under a policy |
| M6 | Storage & Data Model | The schema is the single source of truth and drifts clean |

### Insight plane — turning observation into understanding
| ID | Module | One-line "done" |
|---|---|---|
| M7 | Analyzer & Signals | Every project has fresh derived signals on a schedule |
| M8 | Metrics & Health | Every advertised metric computes, is rated, and cannot lie by omission |
| M9 | Memory & Knowledge | A proven lesson becomes a durable, promoted memory |
| M10 | Patterns & Conventions | Recurring practice (good and bad) is detected and named |
| M11 | Traceability & Doc Drift | A doc claim that no longer matches code is flagged with a receipt |
| M12 | Libraries | An unmanaged dependency is detected, wrapped and queryable |
| M13 | Governance & Rules | Rules resolve by scope precedence and reach the assistant |

### Delivery plane — getting signal back to the human and the assistant
| ID | Module | One-line "done" |
|---|---|---|
| M14 | MCP & Context Delivery | The assistant gets the right context on the first turn |
| M15 | Observatory | The daily user sees *today's one thing*, with its receipt |
| M16 | Project Window | A user trusts what sensei learned about one project |
| M17 | Atlas | The code graph is legible at every level — birds-eye to symbol |
| M18 | Instruments | The user can see and replay what the assistant actually did |
| M19 | Insights & Impact | A recommendation is applied, measured, and reinforced |
| M20 | Planner & Baseline | A goal becomes phases → features → acceptance criteria + gates |
| M21 | Relay | A long run is supervised from anywhere, without leaking code |
| M22 | Agentic Execution | sensei drives a multi-model Planner→Builder→Judge run |

### Team plane — the Dōjō
| ID | Module | One-line "done" |
|---|---|---|
| T1 | Dōjō Core | Knowledge travels up, is governed, and comes back down |
| T2 | Dōjō Consoles | All four roles have a live console |
| T3 | Dōjō Project Planning | A project plans against its tracker and gates on code health |
| T4 | Billing & Seats | A private shared Dōjō can be subscribed and metered |

### Platform plane
| ID | Module | One-line "done" |
|---|---|---|
| P1 | Distribution | One command installs sensei on macOS, Linux and Windows |
| P2 | Website | The site says only what is true, and sells the built product |
| P3 | Harness & CI | No change reaches main without tests, lint, fmt and e2e |

**29 modules** — M1–M22 (Foundation · Insight · Delivery) + T1–T4 (Team) + P1–P3 (Platform). Deliberately more than the 7 feature docs today: the existing docs are too coarse to say "done" against, which is why the coverage map had to be invented.

> **Count correction.** The earlier draft labelled this "23 modules". It is **29**;
> the approved *list* is unchanged, only the label was wrong (§16).

### 4.1 Plane → module index

| Plane | Modules | Count |
|---|---|---:|
| Foundation | M1–M6 | 6 |
| Insight | M7–M13 | 7 |
| Delivery | M14–M22 | 9 |
| Team | T1–T4 | 4 |
| Platform | P1–P3 | 3 |
| **Total** | | **29** |

---

## 5 · `docs/roadmap` outline

### 5.1 `roadmap/README.md`

1. **How to read** — one paragraph, and the status legend.
2. **Priority model** (below).
3. **Status model** (below).
4. **The now / next / later board** — a single table of every module with status + rationale.
5. **Sequencing** — the dependency graph (which modules gate which).
6. **Pointer** to `06-gaps.md` and to `backlog.md` for issue-level work.

### 5.2 Status model

Six states, matching what the code can actually evidence:

| State | Meaning | Evidence required |
|---|---|---|
| ✅ **Built** | Shipped and verifiable | a pinned test or a live route |
| 🟡 **Partial** | One half present | names the missing half |
| 🖼️ **Mock-only** | Design exists, no surface | mockup path |
| 📄 **Designed** | Doc/design only | design path |
| 💤 **Dormant** | Built but flagged off / not on main | the flag name |
| ⛔ **Gap** | Intended, neither designed nor built | the gap register entry |

(These are the coverage-map legend plus the gap state, so the existing map migrates without translation.)

### 5.3 Priority model

Ranked by the product's own rule — *does it move FTR, or expose why it moved?* — plus three hard gates:

1. **Correctness before breadth** — anything silently wrong (production-readiness §A) outranks features.
2. **Loop-closing before surface** — the learn→deliver→measure arc beats net-new screens.
3. **Floor before polish** — CI/tests/distribution before anything a second person installs.

Concretely: `P0 (trust)` → `P1 (loop)` → `P2 (differentiator)` → `P3 (breadth)` → `P4 (team)` → `P5 (orchestrator)`. This preserves the existing `plan/README.md` sequencing (D → A+B → C → F/G) while making it module-scoped.

### 5.4 Module sections — `<plane>.md`

Five files, one per plane, each a table:

| Module | Status | Priority | Blocker / next step |
|---|---|---|---|

- `01-foundation.md` — M1–M6
- `02-insight.md` — M7–M13
- `03-delivery.md` — M14–M22
- `04-team.md` — T1–T4
- `05-platform.md` — P1–P3

Each row's "next step" is one actionable sentence, and points into `06-gaps.md` by ID.

### 5.5 `roadmap/06-gaps.md`

The consolidated register from §12. Every gap has: ID · module · one line · evidence · priority. This replaces `plan/README.md`'s G1–G10 (which are absorbed and re-IDed) and pulls in the production-readiness A–H.

### 5.6 What the roadmap answers that it cannot today

- "Which modules are done?" → one table, one legend, no cross-reading.
- "What is the next thing for module M8?" → one row.
- "Is this a known gap?" → one register.
- "What shipped, when?" → `history/` + git, not a status table that rots.

---

## 6 · Gherkin — how scenarios fit

### 6.1 Why Gherkin (and not prose acceptance)

The repo already has the house style, in `spec/dojo/dojo-auth-provisioning.md`
and `spec/indexer/16-scenarios.md`. It moves into `.feature` files unchanged:

```gherkin
# prd/modules/team/dojo-core/scenarios/auth-provisioning.feature
Feature: Dōjō auth provisioning

  # pinned: dojo::auth::cli_refresh_sets_membership_active
  Scenario: signing in from the CLI activates a pending membership
    Given a membership in state "authenticating"
    When the CLI completes the device-code refresh
    Then the membership is "active"
    And a heartbeat is recorded
```

The `# pinned:` comment names the test that proves the scenario. `NO TEST` marks a
scenario with no proof. That single convention is what turns the PRD into a
**gap-finder**: a scenario either has a pin (built), or it doesn't (gap).

### 6.2 Where scenarios live — `.feature` files (decision 3)

Each module owns a `scenarios/` folder of `.feature` files, **referenced from the
module `README.md`**:

```
prd/modules/delivery/mcp-context-delivery/
  README.md
  scenarios/
    context-delivery.feature
    search.feature
```

- **One feature per file**, named for the behaviour, not the screen.
- The module `README.md`'s `## Scenarios` section links every file and says which
  are pinned and which are gaps.
- `.feature` files are the machine-readable artifact; the markdown is the human
  narrative. A runner can execute the files directly; the markdown links rather
  than duplicates the steps.
- The existing house style carries over unchanged: `# pinned:`, `# pending:`,
  `NO TEST`, `# gap:` (below).
- Feature and scenario names are **stable identifiers** — renaming one is a
  breaking change, because tests, gaps and the roadmap reference them.

### 6.3 Status tags

Each scenario carries one of:

| Marker | Meaning |
|---|---|
| `# pinned: <test>` | a test proves it today |
| `# pending: <issue>` | no test yet; issue filed |
| `NO TEST` | accepted as unproven (rare — must be justified) |
| `# gap: <gap-id>` | points into `roadmap/06-gaps.md` |

### 6.4 How this validates the codebase and surfaces gaps

1. Author the scenario from the PRD (what *should* be true), not from the code.
2. Search the test suite for the assertion. Found → pin it.
3. Not found → either the feature is a gap, or the test is missing. Either way it lands in `06-gaps.md` with a category.
4. The roadmap status for the module is then **derived**: `built` when all its scenarios are pinned.

This is the mechanism the brief asked for: the spec is written first, the code is measured against it, and the delta is the gap register. It also catches the reverse — code with no scenario is unclaimed behaviour.

### 6.5 Acceptance for this reorg

We do not need all scenarios perfect on day 1. Rule of thumb: **modules that are ✅ built get scenarios pinned as they are next touched; modules that are ⛔ gaps get scenarios authored now** (that is what tells us what to build).

---

## 7 · Diagrams → SVG plan

### 7.1 What the audit found

Almost no mockup diagram is standalone-web-safe:

| Source | Why it can't be copied |
|---|---|
| `*.dc.html` (Schema & Call Graph, Metrics Directions, Deck) | D3/data-bound; needs the design-canvas runtime |
| `lib/**` `.jsx` (journey maps, diagrams) | React components taking props + CSS tokens |
| template-string SVGs | reference `var(--…)` tokens that don't travel |
| `Sensei End-to-End Journey.html`, `Sensei Flow Walkthrough.html` | **contain no SVG at all** (HTML/CSS only) |

Only genuinely standalone assets today: `mockups/Sensei/assets/glyphs.svg` and the brandmarks (`sensei.svg`, `dojo/dojo.svg`).

**Conclusion: author clean, self-contained SVGs into `prd/assets/diagrams/`. Do not copy runtime-bound markup.**

### 7.2 Diagram register

| # | Diagram | Source | Format | Notes |
|---|---|---|---|---|
| D1 | The core loop (capture→…→measure) | `01-vision.md` | **SVG** | hero of `prd/README.md` |
| D2 | The journey — 4 segments + Dōjō + Relay | `Sensei Journey Map.html` | **SVG** | distilled; big, one-off |
| D3 | Dōjō role journeys (dev/maintainer/admin/lead) | `Sensei Dōjō Journey Map.html` | **SVG** ×4 | or one 4-lane SVG |
| D4 | The whole loop: machine → Dōjō → beyond | `Sensei End-to-End Journey.html` | **SVG** | source has no SVG; author fresh |
| D5 | Dōjō lifecycle: contribute→distribute | `journeys/dojo.md` | **SVG** | small |
| D6 | Relay: away-from-keyboard through the Dōjō | `Sensei Dōjō Journey Map.html` | **SVG** | |
| D7 | Schema & call graph | `Sensei Schema and Call Graph.dc.html` | **SVG** | re-author static; the dc version is D3-bound |
| D8 | Metrics directions / families map | `Sensei Metrics Directions.dc.html` | **SVG** | re-author static |
| D9 | Module lifecycles — the 9 loops | `Sensei Journey Map.html` | **SVG** | feeds M8–M19 |
| D10 | Target inheritance chain (Dōjō→Project→Sprint→Task type→Task) | `dojo2-project.jsx` (new) | **SVG** | §9 |
| D11 | Target gate eligibility map (gated vs insight) | new | **SVG** | §9 |
| D12 | Metric → rating → health pipeline | `metrics/catalog.md` | **SVG** | |
| D13 | Data model (schemas + key tables) | `database/*.md` | **SVG** | from `design.dbml` |
| D14 | Graph level-of-detail (L0 birds-eye → L1 group → L2 detail) | new (§9.7) | **SVG** ×3 | the readability model |
| D15 | Metric gating funnel (all metrics → gate-eligible → sprint gates) | new (§9.6) | **SVG** | master-detail entry point |

### 7.3 What stays Mermaid

Everything that is a *flowchart of text*: the per-module reading-order chains, the
architecture layer diagram, roadmap sequencing. Rule:

> **Mermaid for things a developer edits in a PR. SVG for the handful of
> diagrams a human looks at and reads as a picture.**

Both render on GitHub; SVG wins on fidelity, Mermaid wins on maintenance. The
register above is the closed list of SVGs — nothing else gets hand-drawn.

---

## 8 · Rename / move plan

### 8.1 Directory-level

| Current | Target | Action |
|---|---|---|
| `docs/vision.md` | `docs/prd/01-vision.md` | move |
| `docs/objectives.md` | `docs/prd/02-objectives.md` | move |
| `docs/personas/` | `docs/prd/03-personas.md` | author + remove folder |
| `docs/features/<name>/` | `docs/prd/modules/<module>/` | fold |
| `docs/features/coverage-map.md` | — | stamp `superseded` → `history/` (status folded into roadmap) |
| `docs/spec/**` | `docs/prd/modules/**/` | fold as touched; stamp + `history/` |
| `docs/plan/README.md` | `docs/roadmap/README.md` + `06-gaps.md` | rewrite |
| `docs/plan/*` (dated) | `docs/history/plan/` | move |
| `docs/roadmap/phases.md` | — | stamp `superseded` → `history/` |
| `docs/roadmap/README.md` | `docs/roadmap/README.md` | rewrite in place |
| `docs/requirements/` | — | delete (absorbed) |
| `docs/analysis/` | `docs/history/analysis/` | move |
| `docs/reports/` | `docs/history/reports/` | move |
| `docs/verification/` | `docs/history/verification/` | move |
| `docs/plans/` (plural) | `docs/history/plans/` | move (merge with `plan/`) |
| `docs/blueprints/` | `docs/history/blueprints/` | move (promote any live ones first) |
| `docs/design/` dated files | `docs/history/design/` | move; keep the rest |
| `docs/database/` | unchanged | keep; link from architecture |
| `docs/architecture/`, `docs/journeys/`, `docs/mockups/` | unchanged | keep |
| `docs/production-readiness.md` | `docs/roadmap/07-quality.md` | move (measured annex) |
| `docs/CHECKPOINT.md`, `svelte5-runes.md`, `transcript-paths.md` | `history/` or `architecture/` | move per topic |

### 8.2 Naming conventions for the new tree

- Module folders: `prd/modules/<kebab-name>/` (e.g. `code-graph`, `governance`, `dojo-project-planning`).
- Roadmap files: one per plane (`01-foundation.md`, `02-insight.md`, `03-delivery.md`, `04-team.md`, `05-platform.md`).
- Diagrams: `prd/assets/diagrams/dNN-<slug>.svg`.
- History: `history/<origin>/<original-name>` — keep the original filename so `git log --follow` and external links survive.

### 8.3 The one hard rule

**Content is never lost.** A file is either:
- **moved** (same bytes, new path), or
- **folded** (content merged into a new doc; original stamped `status: superseded` and moved to `history/`), or
- **superseded** (stamped and moved; no merge).

No `rm`. The `history/` tree is the readable backstop; git is the durable one.

---

## 9 · The new Dōjō planning slice (T3)

### 9.1 What the mockup proposes

`mockups/Sensei/lib/dojo2/dojo2-project.jsx` adds a Project detail surface: **Repos · Team · Integrations · Targets · Sprints · Code health**. Key model lines:

- a project groups many repos (GitHub/GitLab/Azure/Bitbucket/local path);
- tasks + sprints are **synced read-only** from the tracker that owns sprints (Jira *or* ADO iterations — one owner);
- a session counts toward a task when its **branch, commit or prompt carries the ticket key**;
- **targets gate on code quality only; AI-usage signals are insights, never gated**;
- targets inherit **Dōjō → Project → Sprint → Task type → Task** (most specific wins);
- a sprint that misses a **blocking** target cannot close without a note.

Seven gating targets in the mockup: hard-rule violations, coverage on guarded paths, new layer violations, new cycles, modules in the zone of pain, changed functions over complexity 10, new high-severity anti-patterns.

### 9.2 Which of these are buildable today

This is the honest map. The `target` column in `sensei.metrics` is vestigial (a
normalisation bound), so this is a product decision, not a schema read.

| Mockup target | Metric / source | Available today? | Verdict |
|---|---|---|---|
| Hard-rule violations at merge | `rule_check_runs`, checker | **yes** (`checker.rs` shipped) | ✅ gate-eligible |
| Coverage on guarded paths | `coverage` (lcov ingest) | **thin** (3 repos); needs a path-scoped read | 🟡 gate-eligible once scoped |
| New layer violations | architecture metrics | **absent** (#156) | ⛔ blocked |
| New dependency cycles | architecture metrics | **absent** (#156) | ⛔ blocked |
| Modules in zone of pain | architecture metrics (instability × abstractness) | **absent** (#156) | ⛔ blocked |
| Changed functions over complexity 10 | qlty complexity | project-level only; **no per-function/per-changed-function read** | 🟡 needs a new read |
| New high-severity anti-patterns | anti-pattern detector | `detected_patterns` is behavioural churn; `patterns/` absent | ⛔ blocked |

**So 2 of 7 gates are buildable now, 2 need a scoped read, 3 are blocked on the architecture-metrics family (#156), which is itself blocked on edge resolution (#146/#151/#152).** That is the single most important thing to know before promising this feature.

### 9.3 The gate vs improvement split

The product rule — *"targets gate on the code; insights only describe how it was made"* — combined with the inherited-codebase problem.

**Gate-eligible** (code-derived **and delta-scoped**, so an inherited mess does not fail every sprint):

- hard-rule violations **introduced** in the sprint;
- coverage **on guarded/changed paths**;
- new layer/cycle/anti-pattern violations **introduced**;
- complexity of **changed** functions.

**Improvement-only** (trend, never a gate):

- everything behavioural: `ftr`, `rework_ratio`, `spec_depth`, `spec_deviation_rate`, `context_pressure_rate`, `throughput`, `time_to_useful_result`, `session_duration`, `tokens_*`, `cache_reuse`, `cost_per_result`, `interruption_rate`, `memory_promotion`, `incomplete_analysis_*`, `refuted_finding_rate`, `run_completion`;
- everything project-level and absolute: `duplication_ratio`, `module_quality`, `churn_rate`, `churn_concentration`;
- weight-0 / neutral metrics: `churn_concentration`, `session_duration`, `tokens_*_per_day`, `false_crash_rate`.

**Rule, stated for the spec:** *a gate must be scoped to the sprint's own delta or to a path the sprint chose to guard. Absolute project-level ratios are improvement-only.* An inherited low-quality codebase thus fails nothing on day 1 — it starts a ratchet.

### 9.4 The fairness mechanism (own-baseline ratchets)

Fixed absolute targets punish adoption. Two mechanisms to make gates fair:

1. **Delta gates** — count *new* violations, not total.
2. **Own-baseline ratchets** — for the project-level metrics that can't be delta-scoped, compare the repository against **its own history**. `graph_confidence` / `public_surface_ratio` (proposed, and computable today) already say *"compare against ITS OWN history"*. A ratchet says "no worse than the trailing baseline", which is achievable for a legacy repo and still catches regressions.

**Consequence for the PRD:** T3 must ship a *ratchet* concept, not just thresholds. That is a new design element the mockup implies but does not name.

### 9.5 T3 module spec — outline (to write later)

Purpose · In scope (repo grouping, tracker sync, task linkage, targets, sprint scorecard) · Out of scope (writing back to the tracker; per-author metrics as gates) · Surfaces (`dojo2-project.jsx` tabs; `/v1` routes for trackers/targets/sprints) · Data invariants (tracker sync read-only; one sprint owner; key pattern per repo) · Signals · Done gate · Wrong gate · Scenarios · Gaps.

**Known gaps for T3:** external tracker adapters (Jira/ADO) exist only in the mockup — nothing is built; the architecture-metric gates (#156); path-scoped coverage read; per-function complexity read.

### 9.6 Metrics — master-detail, and gating eligibility (decision 8)

Three problems, one model:

- **Not every metric can be a gate.** Gating requires the reading to be *about the
  code* and *scoped to a delta* (§9.3). Metrics that describe behaviour or the
  whole project can only trend.
- **Too many metrics confuse.** 29 registered metrics on one surface is a wall.
- **A reading is only useful with its design and configuration coverage.** A
  metric with 3 repos of data and a broken rating scale must *look* thin.

**The master-detail model.** One master table (all metrics, one row each) and one
detail view per metric:

| Layer | Shows | Answers |
|---|---|---|
| **Master** | every metric as a one-line row: name · family · direction · rating · gating-eligibility · coverage | "what do we measure, and can it gate?" |
| **Detail** | one metric expanded: definition, formula, source, live coverage, how to read, representation, rating scale (direction-aware), **gating config** | "what exactly goes into it, and how is it used?" |

**Every metric's detail record gains three fields the mockup implies but does not
name:**

| Field | Values | Purpose |
|---|---|---|
| `gating` | `none` · `delta` · `ratchet` | whether (and how) it may block a sprint/merge — §9.3 |
| `coverage` | rated metrics / total weight · repos with data · status (`live`/`thin`/`never`) | stops a thin metric reading as a strong one |
| `design` | the chart/representation + its intended level of detail (§9.7) | keeps the visual honest per metric |

**Coverage-gated presentation:** a metric lights up in the master only when its
coverage clears a bar. This is the fix for #155 (a project with 1 rated metric
scores 98) generalised to the whole registry.

**Deliverable:** a metrics master-detail spec (M8) whose per-metric detail is the
single source for both the registry and the UI. The registry seed
(`database/import/staging/metrics.jsonl`) gains the `gating` field.

### 9.7 Graphs — level of detail, not more pixels (decision 8)

**The problem, stated plainly:** the layered/graph view renders granular
information at a scale that is unreadable — you need 1000% zoom to read a node,
which means the *picture* is not usable as a picture.

**The model — three readable levels, each complete at its own scale:**

| Level | Shows | Interaction |
|---|---|---|
| **L0 · Birds-eye** | modules/communities as blocks, grouped by plane or package, edges as weighted bands; **no individual symbols** | click a group → L1 |
| **L1 · Group** | one module's files/types, its internal edges, and its external edges as stubs | click a node → L2 |
| **L2 · Detail** | a symbol and its neighbourhood: callers, callees, declarations | click through to the code |

**Rules for each level to stay readable:**

1. **Aggregate, don't shrink.** A level shows *fewer things*, not smaller
   versions of the same things. Node count per level is bounded (e.g. ≤ 60 blocks
   at L0).
2. **Labels are level-appropriate.** L0 labels groups ("auth", "storage"),
   never symbol names.
3. **Edges are summarised** at L0 (band thickness = edge count), individual at L2.
4. **The visual style is a display mode, not a schema.** Switching kanji ⇄ solar
   icons (decision 10) happens at the presentation layer — the graph data is
   identical, so the toggle is a view concern shared with M2's preferences.

**Why this matters beyond the diagram:** L0 is also the **code-organization
view** — it is where a manager sees module boundaries, coupling and hotspots
(§9.2, the manager persona). The same three levels serve the developer (L2) and
the manager (L0).

**Deliverable:** M17's spec defines the three levels, the aggregation query per
level, the node/edge budgets, and the icon-style toggle. It is a prerequisite for
reading the architecture metrics (§12 D-7) at any scale.

---

## 10 · Platform & distribution

### 10.1 Static website (`website/`)

Confirmed as the **`P2 · Website`** module (decision 6) — a marketing site.
Today it is a SvelteKit marketing site. The workframes:

- **Honesty:** the site must present built ≠ unbuilt correctly (the product already dropped one claim, C3, for this reason).
- **Module pages:** the module list (§4) gives the site a natural IA — one page per user-facing module (Observatory, Project Window, Dōjō, Relay), each linking to its PRD spec.
- **Gate:** a claim on the site must trace to a module whose status is ✅ or an explicit "coming" marker. That is a *doc-driven* website, and the roadmap is its data source.

### 10.2 Homebrew — macOS (`homebrew/`)

Already a subtree → `sensei-hq/homebrew-tap`. Present state: `brew services`, codesign, `/Applications`, launchd, `stat -f`, `sed -i ''`.

- **Keep as the primary macOS path.** It works.
- Gaps: the Formula releases linux arm64/x86_64 tarballs but ships **no Linux service/packaging story**, and no Windows story at all.

### 10.3 Linux and Windows — the required change

**Current defect: Linux is treated identically to macOS.** That is wrong on every
axis — no `brew services`, no launchd, different filesystem and process
semantics. `crates/bootstrap` already has `health/platforms/windows.rs`, so the
platform seam exists and is under-used.

Proposed model, for the PRD's **P1 · Distribution**:

| Concern | macOS | Linux | Windows |
|---|---|---|---|
| Install | Homebrew tap | tarball + `install.sh` / distro package | MSI / winget (decide) |
| Service manager | launchd | systemd user unit | Windows Service / Task Scheduler |
| Data dir | `~/Library/Application Support` | `~/.local/share` (XDG) | `%APPDATA%` |
| Shell/scripts | `stat -f`, `sed -i ''` | GNU | PowerShell / no POSIX |
| Codesign | required | n/a | Authenticode (decide) |
| Health probe | `platforms/*.rs` | needs its own | exists (`windows.rs`) |

**The rule to write into the PRD:** *every platform-specific behaviour lives
behind a platform adapter.* No `if macOS` reaching into Makefile/scripts. The
Makefile becomes a dev convenience, not the install path.

**Delivery model (decision 7): macOS is the current target; Linux and Windows
must be supported.** Some parts are already in place in theory — the Windows
health probe (`crates/bootstrap/health/platforms/windows.rs`) and the Linux
tarball matrix in the Formula — but the service layer, data directories and
shell assumptions are not. The approach: **finish the three-target design, then
distribute the implementation across the three**, so each platform adapter can be
picked up and verified independently.

### 10.4 P1 module spec — outline (to write later)

Purpose · Platform matrix (above) · In scope (packaging, service, data dirs, upgrade, uninstall) · Out of scope (distro-specific repos, day-1) · Surfaces (Makefile, Formula, bootstrap health) · Data invariants (data dir per platform; migrations run on upgrade) · Done gate (one command → green on all three) · Wrong gate (Linux silently following the macOS path) · Scenarios · Gaps.

---

## 11 · Performance register

Present pain (measured) and future risk (architectural). Fed into the roadmap as
`P0 · trust` work where it makes the product wrong or slow.

### 11.1 Present (measured)

| Area | Measurement | Impact |
|---|---|---|
| DB size | `public.logs` = 25 GB of 37 GB (68%) | `pg_dump` ~12 min, 5.3 GB dump; `log_prune` not keeping up |
| Installs | every install writes a 5.3 GB backup; `database/backup` = 24 GB / 6 dumps | ~24 min lost to dumps; needs `SKIP_DB_BACKUP` + exclude logs |
| Builds | `make install` deletes `target/` → always-cold rebuild | e2e pays cold build + a second dump |
| Graph | 40.3% resolved (1.63 M / 4.04 M edges) | architecture metrics blocked; no target published |
| Indexing | 380 folders `discovered`, 12 `indexing`; watchdog abandonment ~4 folders + ~8 files / 7d | silent incompletion, no class-level health |
| Tests | 3,317 senseid tests run in **no** CI workflow; DB-coupled tests fail under parallel load | nothing is trustworthy in CI |
| E2E | 23/130 red, 19.5 min | slow, and a first run produced nothing (now diagnosable) |
| Parse | unbounded parse stack — one file can abort the daemon | whole-process failure mode |

### 11.2 Future (architectural risk)

| Risk | Why it will bite |
|---|---|
| Metric computation over full history | `churn_rate` has 6,176 rows / 64 repos back to 2016; per-module attribution multiplies it |
| Per-module heatmaps | "module × metric" is the primary friction visual and implies a per-module row per metric per day |
| Embeddings at scale | 123k–157k embedded nodes; re-embedding on every edit will grow |
| Communities | 32,495 clusters, nothing consumes them yet |
| Graph resolution work | every edge-resolution fix moves the whole corpus; needs a ratchet so regressions are visible |
| Realtime Relay | polling-first today; realtime fan-out is a new load class |
| Logs | unless partitioned, the 25 GB problem returns after every prune |

### 11.3 Improvements to write into the PRD

1. Partition `public.logs` by day; prune = `DROP PARTITION`.
2. `SKIP_DB_BACKUP=1` + exclude logs from dumps.
3. Publish per-language resolved-rate **ratchets**.
4. Move embeddings/metrics recomputation to delta-scoped, not whole-corpus.
5. Add a watchdog-abandonment health metric.
6. Bound the parse stack (size + longest-line) with an explicit skip reason.

---

## 12 · Gap register

Consolidated, deduplicated, re-IDed. IDs are stable: `<plane>-<n>`.

### 12.1 Correctness / trust (from `production-readiness.md`)

| ID | Gap | Module |
|---|---|---|
| F-1 | `adopt_node_by_identity` fails a whole file when its row moved (44 files/day) | M4 |
| F-2 | cost is never captured; every cost metric is derived or absent | M8 |
| F-3 | `tokens_per_day` is ~all cache reads, cannot proxy cost | M8 |
| F-4 | graph is 40.3% resolved and no target is published | M4 |
| F-5 | unbounded parse stack — one file aborts the daemon | M4 |
| F-6 | watchdog abandonment routine, unreported | M4 |
| F-7 | pre-bind exit invisible (`write_startup_error` has 3 of its callers) | M1 |

### 12.2 Operations

| ID | Gap | Module |
|---|---|---|
| P-1 | `public.logs` 25 GB / 37 GB; prune not keeping up | M6 |
| P-2 | 5.3 GB backup per install; 24 GB accumulated | M6 |
| P-3 | `make install` deletes `target/` | P3 |

### 12.3 Test & CI

| ID | Gap | Module |
|---|---|---|
| T-1 | CI runs no Rust tests, no clippy, no fmt | P3 |
| T-2 | DB-coupled tests not isolated; fail under parallel load | P3 |
| T-3 | e2e 23/130 red; six assert a retired contract | P3 |
| T-4 | `app/e2e/**` outside every static gate (9 type errors) | P3 |

### 12.4 Dōjō

| ID | Gap | Module |
|---|---|---|
| D-1 | enrollment never completed; 0 outbox rows ever sent | T1 |
| D-2 | daemon dials IPv4, Vite binds IPv6 (worked around, not coded) | T1 |
| D-3 | no written local-Dōjō setup path | T1 |
| D-4 | 7 dojo2 sections fixture-only (rule packs, knowledge, scopes, org ladder, stance) | T2 |
| D-5 | dojo2 cutover human-gated; lib/dojo features not ported | T2 |
| D-6 | external tracker adapters (Jira/ADO) exist only in the mockup | T3 |
| D-7 | architecture-metric targets blocked (#156 ← #146/#151/#152) | T3 · M8 |

### 12.5 Metrics

| ID | Gap | Module |
|---|---|---|
| M-1 | #153 `churn_rate` unit/formula still describe a retired feed | M8 |
| M-2 | #154 `time_to_useful_result` rating in minutes vs value in seconds | M8 |
| M-3 | #155 health score not coverage-gated; inverts (1 metric → 98) | M8 |
| M-4 | no registered metric reads the code graph (architecture family absent) | M8 |
| M-5 | `false_crash_rate` never computed | M8 |
| M-6 | per-module attribution for duplication/maintainability deferred | M8 |

### 12.6 Product capability (from `plan/README.md` G1–G10)

G1 loop-closed ✅, G2 ✅, G3 ✅, G4 ✅, G5 ✅, G6 corpus starved (residual), G7 impact-writer + testability/benchmarks (residual), G8 net-new surfaces, G9 capture/index residuals, G10 Dōjō-gated command governance. These re-ID into `M19` (G1/G2/G7), `M7` (G3/G5), `M14` (G4), `M9` (G6), `M15/M16` (G8), `M3/M4` (G9), `M13` (G10).

### 12.7 Capability gaps (from `features/README.md` + `coverage-map.md`)

| ID | Gap | Module |
|---|---|---|
| C-1 | planner DB persistence + Tasks-tab wiring (v0 stateless) | M20 |
| C-2 | default governance bundle: `seed_default_governance()` 0 callers | M13 |
| C-3 | DORA delivery module — prereq deploy/release-signal detector | M8 |
| C-4 | chat guidance (roadmap only) | M15 |
| C-5 | benchmark runner cut | M18 |
| C-6 | agent/persona editors partial (replay-test not built) | M22 |
| C-7 | solution-track rollup mock-only | M16 |
| C-8 | impact-alert full-screen (fold into Impact?) | M19 |
| C-9 | extension governance: dojo-extensions model not ported | T2 |
| C-10 | personas never authored | prd |

### 12.8 Documentation / structure

| ID | Gap | Module |
|---|---|---|
| X-1 | three declared sources of truth | reorg (§2.1) |
| X-2 | duplicated content across `features/`/`spec/`/`design/`/`architecture/`/`plan/` | reorg |
| X-3 | `spec/` 186 files declared legacy; no fold completed | reorg |
| X-4 | `backlog.md` hand-maintained open-issue table (12 vs 67) — **decision 9: drop it, reference live GitHub issues instead** | docs |
| X-5 | `sensei.error_signature` referenced, does not exist | docs |

### 12.9 Platform

| ID | Gap | Module |
|---|---|---|
| D-8 | Linux treated identically to macOS — wrong on every axis | P1 |
| D-9 | no Linux service/packaging story (tarballs only) | P1 |
| D-10 | no Windows install/service story | P1 |
| D-11 | website presents unbuilt as shipped (one claim already dropped) | P2 |
| D-12 | dependency drift: `@rokkit/*`, `@kavach/*` held back by lockfiles | P3 |
| D-13 | 28 medium osv advisories (transitive; none code-fixable) | P3 |

### 12.10 New gaps from the 2026-10-13 decisions

| ID | Gap | Module |
|---|---|---|
| M-7 | metrics have no master-detail model and no `gating` field; gating eligibility is undefined per metric | M8 |
| M-8 | coverage-gated presentation absent (#155 generalised) — thin metrics read as strong | M8 |
| M-9 | no per-metric design/coverage record tying a metric to its chart and level of detail | M8 |
| A-1 | app is kanji-only; the kanji ⇄ solar icon toggle the mockups + rokkit support is not wired | M2 · M17 |
| A-2 | the personal-preference surface is narrower than the configuration the app already supports | M2 |
| G-1 | graph/layered view is unreadable at granular scale — no L0/L1/L2 level of detail (§9.7) | M17 |
| C-11 | transcript-scanning review not scoped as a task | M3 |
| C-12 | inference-pipeline review not scoped as a task | M5 |
| C-13 | Claude "mods" (supersedes plugin + hooks) — install path + migration not verified | M3 |
| C-14 | multi-assistant expansion: Claude + OpenCode priority; others later; extension seam not specified | M3 |
| C-15 | personas (incl. manager + client/stakeholder viewpoints) never authored — **decision 10: author now** | prd |

---

## 13 · Migration phasing

Content-preserving, module-by-module. **Nothing is deleted; nothing blocks on a big-bang rewrite.**

| Phase | Work | Exit |
|---|---|---|
| **0 · Seed** (this doc) | ✅ structure agreed 2026-10-13; decisions folded (§16) | signed off |
| **1 · Scaffold + move + personas** | create `prd/` (with plane folders + root READMEs), `roadmap/` (5 plane files), `history/`; move `vision`/`objectives`; move dated snapshots; stamp `spec/` + `coverage-map` superseded; **author `prd/03-personas.md` (decision 10)**; rewrite the hub `README.md`; drop `backlog.md`'s hand-maintained table (decision 9) | tree exists; persona + navigation in place |
| **2 · Module specs + scenarios** | author `prd/modules/**` by folding `features/` + `spec/` per module, in priority order (Foundation → Insight → Delivery → Team → Platform); one `.feature` set per module | each module has a spec + `.feature` files |
| **3 · Roadmap** | rewrite `roadmap/README.md` + 5 plane files from the module statuses; publish `06-gaps.md` | one authority for "is it built" |
| **4 · Diagrams** | author the 15 SVGs (D1–D15) | diagrams self-contained |
| **5 · Website + platform** | wire the site to the module statuses (P2); publish the three-target design (P1, decision 7) | claims traceable; platform design ready |
| **6 · Retire** | remove empty folders (`requirements/`, `personas/`, folded `spec/`) | tree matches §2 |

**Ordering rule:** a module is only migrated when it is *next touched* — except the Phase-1 structural moves, which are mechanical. This matches the existing `docs/README.md` migration policy and avoids a 186-file rewrite.

---

## 14 · Open questions — all resolved 2026-10-13

| # | Question | Decision | Folded into |
|---|---|---|---|
| 1 | Module count / planes | 29 modules (M1–M22 · T1–T4 · P1–P3), planes first-class as folders | §2, §4 |
| 2 | `spec/` naming | keep the word for the per-module detail file; drop the folder | §3.4, §8 |
| 3 | `.feature` vs fenced blocks | **`.feature` files**, referenced from markdown | §6 |
| 4 | `roadmap/` shape | 5 per-plane files + `06-gaps.md` | §2, §5.4 |
| 5 | `history/` retention | keep `docs/history/` | §2, §8, §13 |
| 6 | Website scope | **marketing site = `P2 · Website`** | §10.1 |
| 7 | Linux / Windows | macOS current; **Linux + Windows required**; design-ready, built in parallel | §10.3 |
| 8 | Architecture metrics | WIP in a separate backlog session; **master-detail model + explicit gating eligibility**; graphs need **LOD** | §9.6, §9.7 |
| 9 | `backlog.md` table | **drop it**; track by live GitHub issue reference | §8, §12 X-4 |
| 10 | Personas | **author now**, incl. manager + client/stakeholder viewpoints | §3.3, §12 C-15 |

**Follow-ups surfaced by the answers** (now scoped in the register, §12.10):
the kanji ⇄ solar icon toggle and the wider preference surface (A-1, A-2); the
graph level-of-detail (G-1); transcript-scanning and inference-pipeline reviews
(C-11, C-12); Claude "mods" superseding plugin + hooks (C-13); and multi-assistant
adapter expansion with Claude + OpenCode first (C-14).

---

## 15 · Approval checklist

**Agreed 2026-10-13.** All items signed off:

- [x] Target tree agreed (§2) — `prd/` + `roadmap/` + retained `architecture/`/`journeys/`/`design/`/`mockups/` + `history/`
- [x] Precedence table agreed (§2.1)
- [x] Module list agreed — 29 modules (M1–M22 · T1–T4 · P1–P3), 5 planes first-class (§4)
- [x] PRD module-spec template agreed (§3.4)
- [x] Roadmap status + priority models agreed; 5 plane files (§5)
- [x] `.feature` scenario convention agreed (§6)
- [x] SVG register agreed (§7.2, D1–D15)
- [x] Rename/move plan agreed (§8)
- [x] T3 planning slice + gate/improvement split agreed (§9)
- [x] Platform model agreed — macOS / Linux / Windows (§10)
- [x] Open questions 1–10 answered (§14)
- [x] Proceed to Phase 1

**Next:** Phase 1 (§13) — scaffold the tree (planes + root READMEs), move
`vision`/`objectives`, move dated snapshots to `history/`, stamp `spec/` +
`coverage-map` superseded, **author `prd/03-personas.md`**, rewrite the hub
`README.md`, and drop `backlog.md`'s hand-maintained table. Then module specs in
priority order.

---

## 16 · Decisions (2026-10-13) — verbatim

1. **Module granularity:** "23 modules" *[doc miscount — the list agreed is 29: M1–M22 · T1–T4 · P1–P3]*; planes included as first-class (folder + README each).
2. **Root `README.md` per folder** — agreed; enables easy GitHub navigation.
3. **Scenarios:** `.feature` files, referenced via markdown.
4. **`roadmap/`:** 5 plane files — agreed.
5. **`history/` retention:** agreed.
6. **Website:** a marketing site — `P2`.
7. **Platforms:** macOS is the current target; Linux and Windows support is required. Some parts are done in theory; other areas need changes. Have the design ready and distribute development across the three targets.
8. **Architecture metrics:** WIP in a separate backlog session. Break down each metric — what goes into it, how it is used, and its design/configuration coverage. Not every metric can gate, and too many metrics confuse → **master-detail** approach. Graphs/charts can become too granular: need a **birds-eye view** to identify groups/modules, then **zoom into specifics** — this also makes code organization/management easier to view. The layered view was one approach, but the rendered granular view is unreadable (needs ~1000% zoom); each level must be a readable view.
9. **`backlog.md`:** drop the hand-maintained table in favour of the live issue list; track issues by GitHub reference.
10. **Personas:** author in this pass. Missing viewpoints include **management** and **project/client** (needed for multi-user, multi-team projects). Pod/swimlane nomenclature is not important. Also: the current implementation uses **kanji**, but the mockups were modified to support switching between **kanji and solar-style icons** — supported by rokkit's data-driven components, but a **gap in the current app**; **personal preferences need extending** alongside the configurations already supported. **Transcript scanning and the inference pipeline** need review. The **Claude "mods"** feature supersedes the old plugin + hooks. **Enhance support for other systems (OpenCode, Codex, …)** — Claude and OpenCode are the top 2 priorities, others later; **design for extension and expansion of support**.
11. **Numbered prefixes:** non-README docs in `prd/` and `roadmap/` get `0N-` filename prefixes so GitHub's file browser renders them in reading order — `prd/01-vision.md` → `03-personas.md`; `roadmap/01-foundation.md` → `07-quality.md`. `README.md` names are kept as-is so GitHub keeps rendering the folder intro. Applied during Phase 1 (2026-10-13); `reorg.md`'s final-path mentions use the numbered names.
