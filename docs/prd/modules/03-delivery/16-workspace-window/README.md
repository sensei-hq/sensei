# Workspace Window · M16

**Plane:** delivery · **Status:** [roadmap/03-delivery.md](../../../../roadmap/03-delivery.md) ·
**Serves:** team developer ([persona 2](../../../03-personas.md#2--team-developer--the-inheritor)),
client stakeholder ([persona 7](../../../03-personas.md#7--client--workspace-stakeholder--the-narrow-lens))

## Purpose

One workspace, deep: everything sensei knows about a single codebase, the work in
flight, and the controls for what binds to or leaves it. The workspace window is the
place a user goes to **trust** sensei's read on a workspace — Overview shows the FTR,
the top recommendation and its provenance, memory / doc-drift / session stats and
recent rows; each section (Overview · Sessions · Metrics · Memories · Traceability ·
Libraries · Instruments · Patterns · Impact · About) offers
exactly one action (Apply / Review / Dismiss, or send to the assistant). The user
feels this as *I can see why sensei believes this, and act on it here*. Below the
sections, the **Diagrams** list opens the Code map (M17) and Database · ER.

> **Status honesty.** Overview, Sessions, Memories, Traceability, Libraries,
> Instruments, Patterns, Impact and About are built. The Diagrams list is built
> (2026-10-29): the sidebar's Diagrams subsection carries the eight entries, each with
> its mark and hint (`diagrams-nav.ts`). The **workspace roll-up across
> its repositories** (formerly "solution track") is mock-only (C-7), and the **Tasks** section is unbuilt pending planner
> persistence (C-1 → M20). The routes live in the `(workspace)` route group
> (`app/src/routes/(workspace)/workspace/[id]/…`, D12/D30); older screen specs that cite
> `project/[id]` are drift to fix as they fold.

## In scope / out of scope

- In: the per-workspace panes and their shell (`WorkspaceSidebar`); the Overview
  assembler (`GET /api/workspaces/{id}/overview`) — hero top-recommendation + FTR chip +
  memories / doc-drift / sessions stats + recent sessions; per-section endpoints;
  multi-repo membership via `sensei.folder_workspaces` + `folder_role` labels; the intake
  front door (lifecycle × intent × risk → playbook recommend-and-confirm); the memories
  promotion / generalise lane; and Dōjō binding from About (opt-in).
- Out: the portfolio / multi-workspace view (M15 Observatory); the workspace roll-up
  across repositories (mock-only, C-7); planner persistence and the Tasks tab (C-1 → M20); the
  Code map itself (M17) and the instruments tabs (M18) — this window mounts them.

## Surfaces

- **API** — `/api/workspaces/{id}/overview` · `/drift` · `/drift/scan` · `/patterns` ·
  `/libraries` · `/instruments` · `/mcp-tool-stats` · `/memories` · `/impact` ·
  `/sessions` · `/ftr` · `/icon` · `/health`; `/api/patterns/{workspace}`.
  **No `/api/dojo-binding` endpoint** (gap `G-DOJO-AUDIT`).
- **App** — `app/src/routes/(workspace)/workspace/[id]/{intake,overview,metrics,sessions,
  memories,traceability,libraries,instruments,patterns,impact,tickets,delivery,integrations,about}/+page.svelte`
  and `diagrams/`; shell `WorkspaceSidebar.svelte` (sections in `workspace-sidebar-view.ts`,
  diagram entries in `diagrams/diagrams-nav.ts`).
- **Data** — `sensei.workspaces`, `sensei.project_ftr_metrics` (view `project_metrics`),
  `sensei.memories`, `sensei.workspace_drift` (view), `inference.drift_items`,
  `inference.recommendations`, `activity.sessions`, `sensei.project_libraries`,
  `sensei.libraries`, `sensei.project_dependencies`, **`workspace_patterns` (view —
  the table `sensei.promoted_patterns` does not exist)**, `sensei.folder_workspaces` (view).
  **No `sensei.rules` table** — rules are `memories` rows with `namespace_id` +
  `enforcement` + `origin`.

## Diagrams

One flat list under the sections, each entry styled like the other sections with its
own mark and a one-line hint. The first seven are lenses of one embedded Code map
([M17](../17-code-map/README.md)), which switches lens without reloading and supplies
the only header for its lenses. Database · ER is its own document
([spec/indexer/20-schema-entities](../../../../spec/indexer/20-schema-entities.md)):
tables, columns and keys are not the call graph.

| Entry | Mark | Hint |
|---|---|---|
| Structure | 構 | the lay of the land |
| Wheel | 輪 | which groups lean on which |
| Zones | 域 | hard to change, or unused |
| Dependency matrix | 格 | dense areas, cycles |
| Cycles | 環 | break before refactoring |
| Hidden coupling | 縁 | change together, no edge |
| Complexity | 繁 | where to refactor and test |
| Database · ER | 表 | tables, columns and keys |

- **Retired:** the separate **Atlas** section, and the Layers, World, Neighbourhood and
  Ownership entries. Layers is Structure's *Colour by direction*; World and
  Neighbourhood are the Code map's L0 and L2; Ownership shows per-person data (D6).
- **Glyphs.** No mark is shared between two sidebar entries. Libraries keeps 庫; the
  Matrix takes 格 and Database · ER takes 表.

## Data invariants

- **Lead.** FTR has a single source. The chip everywhere reads
  `sensei.project_ftr_metrics.ftr_14d` — the same view as the workspaces index; two
  derivations must never disagree.
- **Lead.** Memories exclude archived. The section reads
  `WHERE workspace_id = $id AND status != 'archived'`.
- **Lead.** Never merge silently. A multi-repo auto-suggestion shows a *"combine with
  {other-workspace}?"* banner; it never presents a merged view without consent.
- **Lead.** `goal`, not `vision`. `workspace.goal ← sensei.workspaces.goal`; the column
  already exists — no rename.
- **Lead.** The icon is read from jsonb. `icon->>'value'` where `icon->>'kind'='kanji'`,
  else `DEFAULT_KANJI = "場"`.
- **Lead.** The warn rule. A workspace warns only when `sessions_7d > 0` AND (open drift
  > 0 OR `ftr_7d < 0.6`); a dormant workspace never warns.
- **Lead.** The wire is camelCase. `ftr14d`, `sessions7d`, `sessions7dCorrected`.

## Signals shown

| Element | Meaning | Worked example | Decision it drives / next step |
|---|---|---|---|
| Kanji + eyebrow | workspace identity | `全 · Workspace · Acme · 3 repos` | none — candidate to cut (identity) |
| FTR · 14d | rolling first-try rate | `FTR · 14d 72%` | low → open Sessions and read the corrected ones |
| Hero headline/body | top recommendation via narration-cache, or all-quiet | `top_recommendation.title` / `"Sensei is observing…"` | Apply, Review or Dismiss the top recommendation, or send it to the assistant |
| Sessions · 7d | sessions + corrected sub-count | `12 · 3 corrected` | open the corrected sessions |
| Memories | active / ready-share / to-merge | `24 · 5 to share · 2 to merge` | share the ready memories; merge the duplicates |
| Doc drift | open of referenced docs (warn tone) | `3 of 41 referenced docs` | open Traceability and fix the drifted docs |
| Recent rows | per-session FTR + folder role | `s-2891 · frontend · 42m · first-try right` | open a session's replay |
| Impact trend | FTR annotated per apply event | markers at `applied_at` | open Impact to see which applied change moved FTR |

## Done gate

- `/workspace/{id}/overview` renders header, hero, stats and the recent list on live data,
  with no placeholders.
- The FTR chip equals the workspaces-index card FTR exactly.
- Hero copy comes from the narration-cache, with a static fallback when quiet.
- A multi-repo workspace (>1 folder) shows the `{n} repos` chip and folder-role rows;
  recent rows click through to Replay.
- Stat sub-lines carry backing counts (no orphan numbers).
- The Diagrams list shows the eight entries above, in order, each with its mark and
  hint; no mark repeats anywhere in the sidebar.

```bash
curl -s http://localhost:7744/api/workspaces/$ID/overview \
  | jq '{ftr: .ftr14d, hero: .top_recommendation.title, drift: .drift_open}'
```

## Wrong gate

- **The FTR chip differs from the index card** — two derivations.
- **Hero shows "getting started" instead of all-quiet** — the quiet copy regressed.
- **The memories count includes archived rows** — filter missing.
- **"of N referenced docs" with `N=0` while the count is > 0** — denominator wrong.
- **Recent rows include other workspaces' sessions**, or a row click lands in "no tool
  calls" — a scoping / id-resolution regression.
- **A multi-repo header hides the `{n} repos` chip** — membership not surfaced.
- **A separate Atlas section, or a Layers / World / Neighbourhood / Ownership entry**
  in the Diagrams list; a page header drawn above a Code map lens on top of the map's own.

## Scenarios

- [`scenarios/workspace-overview.feature`](scenarios/workspace-overview.feature) — the
  Overview assembler, the kanji/icon, the warn rule, generalise (pinned).
- [`scenarios/workspace-sections.feature`](scenarios/workspace-sections.feature) — the
  pane endpoints, multi-repo membership, patterns, libraries, merge (pinned + `NO TEST`).

## Open gaps

- [C-7](../../../../reorg.md) — workspace roll-up across repositories is mock-only → M16.
- [G8](../../../../reorg.md) — net-new surfaces; residual → M15/M16.
- [C-1](../../../../reorg.md) — planner DB persistence + Tasks-tab wiring (v0
  stateless) → M20 — the workspace-window Tasks section stays unbuilt.
- [C-8](../../../../reorg.md) — impact-alert full-screen (fold into Impact?) → M19;
  it touches this window's Impact pane.
- **Section kanji** — the app's `SECTIONS` use other kanji than this spec (Libraries
  is 蔵, not 庫).
- **Doc drift** — the `spec/screen/project-*` specs cite `app/src/routes/project/[id]/…`;
  the real routes are `app/src/routes/(workspace)/workspace/[id]/…` (D12). Fix as the
  legacy specs fold.

## Related

- [design/project-management-detailed-requirements.md](../../../../design/project-management-detailed-requirements.md) —
  the shell, the sidebar section list, the diagrams group, and the About document,
  grounded in the mockup symbols.
- [features/04-project.md](../../../../features/04-project.md) — the workspace-window
  dossier (legacy depth, folds here).
- [spec/screen/project-*](../../../../spec/screen/) — overview, sessions, memories,
  traceability, libraries, instruments, patterns, impact, about, atlas.
- [spec/2026-08-05-mockup-drift-audit.md](../../../../spec/2026-08-05-mockup-drift-audit.md) —
  workspace-overview / memories / about / sessions / shell drift rows.
- [architecture/app](../../../../architecture/app.md) · [design/mockup-brief](../../../../design/mockup-brief.md).
- [prd/02-objectives.md §04](../../../02-objectives.md) · [prd/01-vision.md](../../../01-vision.md#the-core-loop).
- Crates: `crates/senseid/src/{workspace_overview.rs, api/handlers/workspace_detail.rs,
  api/handlers/knowledge.rs, api/handlers/codebase.rs, db/pg_store/workspaces.rs,
  db/pg_store/graph.rs}`.
