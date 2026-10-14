# Project Window · M16

**Plane:** delivery · **Status:** [roadmap/03-delivery.md](../../../../roadmap/03-delivery.md) ·
**Serves:** team developer ([persona 2](../../../03-personas.md#2--team-developer--the-inheritor)),
engagement / client lead ([persona 5](../../../03-personas.md#5--engagement--client-lead--the-confidentiality-guard)),
client stakeholder ([persona 7](../../../03-personas.md#7--client--project-stakeholder--the-narrow-lens))

## Purpose

One project, deep: everything sensei knows about a single codebase, the work in
flight, and the controls for what binds to or leaves it. The project window is the
place a user goes to **trust** sensei's read on a project — Overview shows the FTR,
the top recommendation and its provenance, memory / doc-drift / session stats and
recent rows; each section (Overview · Sessions · Metrics · Memories · Traceability ·
Libraries · Instruments · Patterns · Impact · About) plus the Diagrams group offers
exactly one action (Apply / Review / Dismiss, or send to the assistant). The user
feels this as *I can see why sensei believes this, and act on it here*.

> **Status honesty.** Overview, Sessions, Memories, Traceability, Libraries,
> Instruments, Patterns, Impact, About and Atlas are built. The **solution-track
> roll-up** is mock-only (C-7), and the **Tasks** section is unbuilt pending planner
> persistence (C-1 → M20). The screen specs cite `app/src/routes/project/[id]/…`,
> but the real routes live in the `(project)` route group
> (`app/src/routes/(project)/project/[id]/…`) — a doc drift to fix as legacy specs fold.

## In scope / out of scope

- In: the per-project panes and their shell (`ProjectSidebar`); the Overview
  assembler (`GET /api/projects/{id}/overview`) — hero top-recommendation + FTR chip +
  memories / doc-drift / sessions stats + recent sessions; per-section endpoints;
  multi-repo membership via `sensei.folder_projects` + `folder_role` labels; the intake
  front door (lifecycle × intent × risk → playbook recommend-and-confirm); the memories
  promotion / generalise lane; and Dōjō binding from About (opt-in).
- Out: the portfolio / multi-project view (M15 Observatory); the solution-track
  roll-up (mock-only, C-7); planner persistence and the Tasks tab (C-1 → M20); the
  graph canvas itself (M17) and the instruments tabs (M18) — this window mounts them.

## Surfaces

- **API** — `/api/projects/{id}/overview` · `/drift` · `/drift/scan` · `/patterns` ·
  `/libraries` · `/instruments` · `/mcp-tool-stats` · `/memories` · `/impact` ·
  `/sessions` · `/ftr` · `/icon` · `/health`; `/api/patterns/{project}`.
  **No `/api/dojo-binding` endpoint** (gap `G-DOJO-AUDIT`).
- **App** — `app/src/routes/(project)/project/[id]/{overview,sessions,memories,
  traceability,libraries,instruments,patterns,impact,about,atlas}/+page.svelte`;
  shell `ProjectSidebar.svelte`.
- **Data** — `sensei.projects`, `sensei.project_ftr_metrics` (view `project_metrics`),
  `sensei.memories`, `sensei.project_drift` (view), `inference.drift_items`,
  `inference.recommendations`, `activity.sessions`, `sensei.project_libraries`,
  `sensei.libraries`, `sensei.project_dependencies`, **`project_patterns` (view —
  the table `sensei.promoted_patterns` does not exist)**, `sensei.folder_projects`.
  **No `sensei.rules` table** — rules are `memories` rows with `namespace_id` +
  `enforcement` + `origin`.

## Data invariants

- **Lead.** FTR has a single source. The chip everywhere reads
  `sensei.project_ftr_metrics.ftr_14d` — the same view as the projects index; two
  derivations must never disagree.
- **Lead.** Memories exclude archived. The section reads
  `WHERE project_id = $id AND status != 'archived'`.
- **Lead.** Never merge silently. A multi-repo auto-suggestion shows a *"combine with
  {other-project}?"* banner; it never presents a merged view without consent.
- **Lead.** `goal`, not `vision`. `project.goal ← sensei.projects.goal`; the column
  already exists — no rename.
- **Lead.** The icon is read from jsonb. `icon->>'value'` where `icon->>'kind'='kanji'`,
  else `DEFAULT_KANJI = "場"`.
- **Lead.** The warn rule. A project warns only when `sessions_7d > 0` AND (open drift
  > 0 OR `ftr_7d < 0.6`); a dormant project never warns.
- **Lead.** The wire is camelCase. `ftr14d`, `sessions7d`, `sessions7dCorrected`.

## Signals shown

| Element | Meaning | Worked example |
|---|---|---|
| Kanji + eyebrow | project identity | `全 · Project · Acme · 3 repos` |
| FTR · 14d | rolling first-try rate | `FTR · 14d 72%` |
| Hero headline/body | top recommendation via narration-cache, or all-quiet | `top_recommendation.title` / `"Sensei is observing…"` |
| Sessions · 7d | sessions + corrected sub-count | `12 · 3 corrected` |
| Memories | active / ready-share / to-merge | `24 · 5 to share · 2 to merge` |
| Doc drift | open of referenced docs (warn tone) | `3 of 41 referenced docs` |
| Recent rows | per-session FTR + folder role | `s-2891 · frontend · 42m · first-try right` |
| Impact trend | FTR annotated per apply event | markers at `applied_at` |

## Done gate

- `/project/{id}/overview` renders header, hero, stats and the recent list on live data,
  with no placeholders.
- The FTR chip equals the projects-index card FTR exactly.
- Hero copy comes from the narration-cache, with a static fallback when quiet.
- A multi-repo project (>1 folder) shows the `{n} repos` chip and folder-role rows;
  recent rows click through to Replay.
- Stat sub-lines carry backing counts (no orphan numbers).

```bash
curl -s http://localhost:7744/api/projects/$ID/overview \
  | jq '{ftr: .ftr14d, hero: .top_recommendation.title, drift: .drift_open}'
```

## Wrong gate

- **The FTR chip differs from the index card** — two derivations.
- **Hero shows "getting started" instead of all-quiet** — the quiet copy regressed.
- **The memories count includes archived rows** — filter missing.
- **"of N referenced docs" with `N=0` while the count is > 0** — denominator wrong.
- **Recent rows include other projects' sessions**, or a row click lands in "no tool
  calls" — a scoping / id-resolution regression.
- **A multi-repo header hides the `{n} repos` chip** — membership not surfaced.

## Scenarios

- [`scenarios/project-overview.feature`](scenarios/project-overview.feature) — the
  Overview assembler, the kanji/icon, the warn rule, generalise (pinned).
- [`scenarios/project-sections.feature`](scenarios/project-sections.feature) — the
  pane endpoints, multi-repo membership, patterns, libraries, merge (pinned + `NO TEST`).

## Open gaps

- [C-7](../../../../reorg.md) — solution-track roll-up is mock-only → M16.
- [G8](../../../../reorg.md) — net-new surfaces; residual → M15/M16.
- [C-1](../../../../reorg.md) — planner DB persistence + Tasks-tab wiring (v0
  stateless) → M20 — the project-window Tasks section stays unbuilt.
- [C-8](../../../../reorg.md) — impact-alert full-screen (fold into Impact?) → M19;
  it touches this window's Impact pane.
- **Doc drift** — every screen spec cites `app/src/routes/project/[id]/…` without the
  `(project)` route group; fix as the legacy specs fold.

## Related

- [design/project-management-detailed-requirements.md](../../../../design/project-management-detailed-requirements.md) —
  the shell, the sidebar section list, the diagrams group, and the About document,
  grounded in the mockup symbols.
- [features/04-project.md](../../../../features/04-project.md) — the project-window
  dossier (legacy depth, folds here).
- [spec/screen/project-*](../../../../spec/screen/) — overview, sessions, memories,
  traceability, libraries, instruments, patterns, impact, about, atlas.
- [spec/2026-08-05-mockup-drift-audit.md](../../../../spec/2026-08-05-mockup-drift-audit.md) —
  project-overview / memories / about / sessions / shell drift rows.
- [architecture/app](../../../../architecture/app.md) · [design/mockup-brief](../../../../design/mockup-brief.md).
- [prd/02-objectives.md §04](../../../02-objectives.md) · [prd/01-vision.md](../../../01-vision.md#the-core-loop).
- Crates: `crates/senseid/src/{project_overview.rs, api/handlers/project_detail.rs,
  api/handlers/knowledge.rs, api/handlers/codebase.rs, db/pg_store/project.rs,
  db/pg_store/graph.rs}`.
