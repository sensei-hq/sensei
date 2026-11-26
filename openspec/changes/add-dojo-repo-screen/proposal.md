# Change: Dōjō per-repository screen — metrics plus diagrams (#532)

## Why
A repository in the dōjō has no home: its metrics surface only inside
workspace rollups and its diagrams only at a deep route nobody links to.
Teams need one place per repository — what it is, how active it is, its
numbers, and its diagrams with the branch picker.

## What Changes
- **Route.** `GET /v1/t/{origin}/{org}/repositories/[id]/metrics[?days=N]`
  (member floor, `tenantRoute`): the repo's daily series via the existing
  `repository_metric_series` (invoker — the table's own policy decides row
  by row), 404 when the repo is not this tenant's.
- **Page.** `org/[slug]/repositories/[id]/`: header (name, key,
  visibility, activity, branches), metric cards shaped by the shared
  `toWorkspaceMetricsView` (one repo in, same cards out), and a link into
  the diagrams route (#268) preserving the branch.
- **Shared loader.** The org+repo+branch resolution the diagrams loader
  owns moves to one tested helper both pages call — the workspace screen
  (#533) will be its third caller.

## Non-goals
- New metric kinds or computations; the Teams-view metric set as-is.
- The per-workspace screen (#533); this builds the pieces it reuses.
- Editing anything (elections, branches) — the diagrams page already
  manages branches; elections stay on Sharing.
