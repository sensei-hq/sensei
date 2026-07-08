# 綜 · Solution · Dashboard

**Segment:** 04 · Project window — solution variant
**Route:** `/solution/[id]/dashboard`
**Source mockup:** _none yet — solution track has no dedicated mockup. Reuse the `ProjOverviewLite` primitive from `lib/project-lite-panes.jsx` for the card shape; the aggregate strip is greenfield._
**Source design:** [`docs/archive/ideas/24-desktop-observatory.md`](../../archive/ideas/24-desktop-observatory.md) (Solution scope model)
**App file:** `app/src/routes/solution/[id]/dashboard/+page.svelte`

## Purpose

A **solution** is a logical grouping of related repos that
together deliver one product (backend + frontend + docs +
mobile). The solution dashboard aggregates metrics **across all
member repos** so the user can see the whole thing at a glance
instead of clicking through each project.

Same primitive as [[screen/project-overview]] but scoped to
multiple projects. Kanji is 綜 — *comprehensive*.

## When to use solution vs project

Multi-repo projects can either live as **one project** (single
`sensei.projects` row with N folders — see [[pipeline/capture]]
multi-repo detection) or as a **solution** (N projects grouped).

- **Multi-repo project** — when the repos belong to the same
  product AND share the same primary identity (Acme's UI +
  Backend + Docs, all owned by the same team).
- **Solution** — when the repos are peers under a common effort
  but each has its own identity, its own team, or its own
  release cadence (a monorepo-adjacent world: platform +
  several product teams).

Both live under [[pipeline/capture]] with the same detection
heuristics; the user picks which grouping level fits.

## Data invariants

- `sensei.solutions` — one row per solution:
  - `id`, `name`, `client?`, `vision?`, `icon`,
  - `created_at`, `updated_at`.
- `sensei.solution_members` — join with `sensei.projects`:
  - `solution_id`, `project_id`, `role` (`primary` /
    `platform` / `product-team` / `docs` / `infra` / `other`),
    `added_at`.
- `GET /api/solutions/{id}/dashboard` returns aggregate metrics
  across all member projects.

## Signals shown

Same shape as [[screen/project-overview]] but aggregated:

| Element | Value |
|---|---|
| Header | solution kanji + name + `{n} projects` chip + FTR aggregate |
| Aggregate FTR | weighted-mean of member projects' `ftr_14d` by session count |
| Aggregate sessions · 7d | sum |
| Aggregate memories | sum of `memories.total` |
| Aggregate doc drift | sum of `open_drift_count` |
| Per-project row | one card per member with its own FTR + sessions + drift |
| Cross-project connections | derived edges — API contracts, doc references |
| Aggregated top recommendation | across all members |

## Done gate

- Aggregate FTR is a weighted mean (by session count), not a
  naive average.
- Per-project rows link to each project's own Overview.
- Cross-project connections surface where they exist (e.g. UI
  project imports API types from Backend project).
- Adding / removing a project from the solution updates
  aggregates on next tick.

## Wrong gate

- **Aggregate FTR shown as unweighted mean.** Skews to inactive
  projects.
- **Cross-project connections missed** because
  `sensei.project_dependencies` view isn't consulted.
- **Removing a project leaves stale metrics.** Aggregate not
  re-computed.
- **Solution FTR outperforms every member project.** Weighting
  wrong.

## Related

- [[pipeline/capture]] — multi-repo detection + solution
  grouping
- [[pipeline/governance]] — solution-scoped profile cascade
- [[screen/project-overview]] — per-project peer
- [[screen/solution-sessions]] · [[screen/solution-architecture]]
- (archive: ideas/24-desktop-observatory.md) — source design
