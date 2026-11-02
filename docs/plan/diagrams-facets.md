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
Measured 2026-11-02 on the `sensei` workspace (host load average ~100: a VM at ~1300% CPU,
so buffers, not wall time, are the steady measure). Fixes red-first; before → after.

| # | Finding | Fix | Before → after |
|---|---|---|---|
| 1 | `/hotspots` (workspace window waits on it): `to_timestamp(ts)` filter, seq scan of 543k events + payload detoast | partial index `assistant_events_file_edits_idx`, filter on `ts` itself | 77.6 s · 290k buffers → 54 ms · 5.3k; endpoint 0.15 s ✅ |
| 2 | every per-workspace metrics read aggregates all workspaces (correlated `workspace_id` subquery in `workspace_metrics`, 7k loops) | `workspace_metric_daily` joins `repository_workspace`, the one owner of the pick | 42,304 → 1,166 buffers, 1.2 s → 20 ms; same 6,391 rows, same pick for all 21,861 ✅ |
| 3 | `/metrics` narrative: 28 sequential `UPDATE narration_cache … RETURNING` per GET | one batched statement | median 1.37 s each in the slow log |
| 4 | Metrics page: 27-request second wave, one series per metric | page state (17771862a): draws at once, sections fill as they land, sparklines 4 at a time, reads cached across landing/details ✅; next: the dated daily series inside `/metrics` to drop the 27 reads | — |
| 5 | drill-down awaits `/metrics` before the series, and re-reads everything per metric/grain | the same page state: values once, a series once per metric+grain ✅ | — |
| 2b | the daily roll-up's representative row (props, point-in-time values) was plan-dependent where a repository has a row per commit on a day (70 groups) | latest-written row wins | test-pinned ✅ |
| 2c | OPEN: ratio/pct pooling sums numerator/denominator over every per-commit row of one repository on a day (`duplication_ratio`: 5 rows on 2026-11-02) — likely double counting | decide: one row per repo/day (latest commit) before pooling | not changed |
| 6 | workspace window loads all 133 workspaces + 133 folder queries to show one | `GET /api/workspaces/{id}` | 4–11 s → one row |

## S5 — dōjō sync handoff to Muse
`docs/plan/dojo-sync-handoff.md`: repository, metrics and diagram sync divided into
independent packages for another agent.

## S6 — diagrams over time (proposal, user 2026-11-02)

**Today nothing is kept.** The diagrams are derived live from the current graph
(`structure_graph`, `structure_edges`, `component_zones`), which the indexer overwrites;
`file_complexity` keeps one HEAD per repository (`replace_file_complexity` deletes the last).
Only the metrics keep history (`repository_metrics`, per day and commit).

**Proposal.** On each new HEAD the daemon already notices, snapshot the small derived payloads —
Zones points, the L0 module/band summary (cycles, upward, skips), the Complexity totals and per
module — into `sensei.diagram_snapshots (repository_id, commit_sha, committed_at, kind, payload
jsonb)`, keyed by commit. Days and sprints are views over commits: the last commit of each day, or
of each sprint (from the tracker, D23/D40). The app then plays a lens across the last n commits /
days / sprints: Zones points move from snapshot to snapshot (a point is a module, so it is matched
by id; one that appears or leaves fades), Cycles and Structure show counts appearing and going,
Complexity its totals as a line.

**Costs to decide.** Forward-only costs one small row per kind per commit. Backfilling history
means re-indexing past trees (a worktree per commit) — hours on a large repository, so opt-in and
bounded (last n commits). Workspace = union of its repositories' snapshots at each point in time.
