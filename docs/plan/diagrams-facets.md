# Diagrams as facets — plan (user review, 2026-11-02)

Each diagram is a different facet of the code. Today they share one "What to act on"
(cycles first, upward dependencies next, then a lens-specific item), one zoom, and four
of them (structure, cycles, hidden coupling, complexity) draw the same block map with a
different tint — so they do not show what they claim to. Database · ER is our own layout,
not dbd's.

Order is forward-only: no slice depends on a later one. Every slice is TDD (failing spec
first, committed with its implementation when green) and ends with a live check in the
installed app.

## S0 — rokkit 1.12.3 everywhere
app, dōjō (shares `packages/codemap`, which resolves rokkit from its host) and the
website repo move to 1.12.3. Gate: `bun run check` + unit + e2e green in each.

## S1 — what every diagram shares
1. **Zoom per diagram.** `CodeMapState.zoom` becomes a zoom per lens; switching lens
   restores that lens's own zoom (spec: zoom Zones, switch to Matrix → 1, back → kept).
2. **What to act on is the lens's, and only the lens's.** `takeaways(lens, …)` drops the
   shared cycles/upward lead; each lens owns its list (S2). Structure keeps upward
   dependencies; Cycles owns cycles.
3. **Selectable, copyable.** The takeaways render as text (title, why, the modules), not
   as List buttons, so a selection copies; plus a Copy action that puts the lens's
   takeaways on the clipboard as markdown. Each still opens its module.

## S2 — each lens shows its facet
| Lens | What it highlights | What to act on |
|---|---|---|
| Structure | the layering: upward bands red, skips amber, the rest faint | upward dependencies, heaviest first; layer skips |
| Cycles | only modules in a cycle, one cluster per cycle, each closing edge drawn, the cheapest edge to cut marked | each cycle with the edge to cut (lowest weight) |
| Hidden coupling | only coupled modules; dashed pair edges by commits together; no static edges | top pairs, with "make it explicit or extract" |
| Dependency matrix | issue cells: upward (below the diagonal) red, mutual pairs (i→j and j→i) marked as cycles, skips amber; healthy cells dimmed; counts in the legend | the issue cells, worst first |
| Zones | pain and uselessness in focus (default filter), points labelled, main sequence faded | modules in pain (by dependents), abstractions nothing uses (by distance) |
| Complexity | a **tree**: module → folder → file, each row with functions, over-10 share, max cyclomatic; an overall complexity figure and the maintainability grade at the top | hottest folders and files |
| Wheel | fan-in/fan-out | most depended-on modules; widest fan-out |

Complexity needs one daemon read: `GET …/diagrams/codemap/complexity` aggregating
`sensei.file_complexity` up the module/folder hierarchy (pure aggregation in Rust,
SQL in pg_store), plus the workspace maintainability grade from the quality metric.

## S3 — Database · ER is dbd's viewer
dbd's `/diagram` (dbd.sensei-hq.com) with its sidebar on the right:
- Content header: name, dialect badge, counts (tables · enums · refs).
- Design tabs: **Overview** (at-a-glance cards: schemas, tables, views, procedures,
  enums, references; schemas table) · **Diagram** (ER with Names | Keys | All, Curved,
  zoom, schema legend) · **Entities**. Changelog waits for dbd release 1.
- Entity tabs: **Details** (info, fields table with PK/FK/NN/ENUM settings, default,
  references, notes; references in/out; dependencies) · **Diagram**.
- Sidebar: find an entity, type chips with counts, schemas as a Tree with counts.

The views are dbd's (`site/src/lib/design/*`); `EntityView`/`EntitiesView` are already in
`@rokkit/graph/schema`. Port `OverviewView`, `ObjectView`, `EnumView`, the sidebar and the
diagram controls there too (one implementation for dbd and sensei), release rokkit, then
compose the sensei page from them. The schema payload gains what Overview counts need
(enums, procedures, views) where it lacks them.

## S4 — load time
Workspace window and Metrics page: measured endpoint timings, EXPLAIN evidence, then
indexes (via dbd reconcile) / query rewrites / parallel loads. Findings first, fixes
red-first, before/after timings recorded here.

## S5 — dōjō sync handoff to Muse
`docs/plan/dojo-sync-handoff.md`: repository, metrics and diagram sync divided into
independent packages for another agent.
