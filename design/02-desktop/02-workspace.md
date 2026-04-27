# 02 — Workspace

> Route: `/overview` — the home screen after setup is complete.

## Purpose

The developer's workspace manager. See all repos, create and manage projects, trigger scans, monitor indexing. This is the "all repos" view — the starting point for organizing work.

## What the User Sees

### Repo List

Every indexed repo, grouped by project membership:

```
Acme Platform (3 repos)
  [backend]  acme-api      342 fns · 45 types · indexed 2m ago
  [frontend] acme-ui       128 fns · 22 types · indexed 2m ago
  [library]  design-system   56 fns · 18 types · indexed 5m ago

Standalone
  blog-engine              89 fns · 12 types · indexed 1h ago
  experiment-ml            (not indexed)
```

Each repo card shows: name, path, role badge (if in a project), language breakdown, function/type counts, last indexed time.

### Search and Filter

Search bar filters repos by name. Future: filter by project, by language, by indexed status.

### Scan Action

"Scan folder" button to discover new repos. Uses same flow as setup step 3-4 but without the wizard wrapper.

### Project Actions

- **Create project** — name it, add repos, assign roles
- **Quick group** — select multiple repos, click "Group into project"
- **Unassigned indicator** — recently scanned repos not in any project are visually distinct

## Insights

- **Index health at a glance** — which repos are indexed, which have errors, which are stale
- **Unassigned repos** — repos that haven't been organized into projects may be forgotten side projects or new additions that need grouping

## Actions

| Action | Result |
|--------|--------|
| Click repo card | Navigate to repo detail (`/r/{id}` or `/p/{id}/r/{rid}`) |
| Click project header | Navigate to project dashboard (`/p/{id}`) |
| Scan folder | Daemon scans for new repos, adds to list |
| Create project | Opens project creation flow |
| Re-index repo | Triggers re-index for stale/errored repos |

## What's Built

The overview page is implemented with repo cards, search, scan button, and project grouping. Navigation to repos and projects works.

## What Needs Work

- **Project creation from overview** — currently only available via setup wizard
- **Index health indicators** — cards don't show error/stale state clearly
- **Bulk actions** — select multiple repos for grouping or batch re-index
