---
name: Folder-project resolution must be indexable
description: folder_projects resolves through a function, so a predicate cannot push down — every view that joins it collapses
date: 2026-09-30
status: ready
---

# Folder-project resolution must be indexable

## The defect, and it is mine

`sensei.folder_projects` (added in #210) resolves a folder's project through
`sensei.repo_anchor_for(abs_path)` — a set-returning FUNCTION. Postgres cannot
push a predicate into a function scan, so asking the view about **one** project
reads **every** folder first:

```
Aggregate
  -> Hash Join
       -> Nested Loop
            -> Seq Scan on folders f        (actual rows=13715)
            -> Function Scan on repo_anchor_for  (loops=13715)
```

That is the plan for `SELECT count(*) FROM folder_projects WHERE project='sparsh'`
— a question with 45 answers, costing a full walk of 13,715 folders and 13,715
function calls.

The view is correct. It is simply unusable as a join in anything hot, which is
every view that needs it.

## How it was found, and what it nearly cost

A migration of 8 views onto `folder_projects` was run and **adversarially
verified**. Four failed outright and three of the four "passing" ones carried
serious findings:

| view | measured |
|---|---|
| `structure_edges` | **~450× slower** — 81 ms → ~37 s for one project |
| `task_health` | **~70×** on the query its own doc comment advertises |
| `project_library_version_conflicts` | latency regression on a shipped API handler |
| `project_metrics` | LEFT JOIN made the view **non-auto-updatable**, breaking a production WRITE path silently |

All four trace to the same plan above. The edits were reverted; the foundation
is what is wrong.

**This was my error, not the migration's.** I built the resolver view in the
previous slice and made it the migration target without once reading its plan
under a predicate.

## The fix

Materialise the resolution into an **indexable relation**:

```sql
create materialized view folder_project_mv as
  select folder_id, repo_folder_id, repository_id, project_id, project
    from folder_projects;

create unique index on folder_project_mv (folder_id, project_id);  -- REFRESH CONCURRENTLY
create index on folder_project_mv (project_id);
create index on folder_project_mv (project);
```

Then a `project` predicate is an index scan, joins are hash/index joins, and all
four regressions disappear at once — the data is identical, only its access path
changes.

### Refresh belongs on index completion, not a timer

The mapping changes only when a repository is scanned or a project is edited.
`DetectCommunities` is already the sole writer of `folders.status='indexed'` and
is the natural hook, which is the same conclusion #206 reached for the diagram
views. Doing both under one mechanism is cheaper than two.

`REFRESH ... CONCURRENTLY` needs the unique index above, which is why it is
`(folder_id, project_id)` and not `(folder_id)` — a folder legitimately has two
rows when its repository serves two projects.

### Staleness must be visible

A matview can be out of date, and silence about that is how a wrong diagram gets
believed. Carry a `refreshed_at` the UI can show, per #206's done gate.

## Done gate

1. `SELECT ... WHERE project = $1` against the materialised relation uses an
   index scan — proved by `EXPLAIN`, not assumed.
2. `structure_edges` for one project returns to its pre-migration latency
   (81–91 ms measured on `sparsh`), not 37 s.
3. Row-for-row identical to `folder_projects` — same count, same pairs.
4. Refresh is wired to index completion; a stale matview is distinguishable from
   a fresh one.
5. Only then: re-run the view migration, with the before/after counts already
   captured (`/tmp/p2/baseline.txt`).

## Wrong gate

- **A plain view with a different shape.** Any formulation that still calls
  `repo_anchor_for` per row has the same plan. The point is materialisation, not
  rewording.
- **An index on the view.** Postgres cannot index a non-materialised view.
- **Refresh on a timer.** The mapping is event-driven; a timer is either too slow
  (wrong answers) or too fast (needless churn).
- **Shipping the migration against the unmaterialised view "carefully".** The
  regressions are in the access path, not in the SQL style, so care does not
  reach them.

## A rule this surfaced, worth keeping

One agent wrote an **unmeasured claim into a `comment on` block** — "ten
repositories serve two or three projects each", when four do. A wrong comment in
source is a bug; a wrong comment in a `comment on` is a false fact in a queryable
catalog surface that tooling and future readers treat as data. Any
agent-authored DDL must measure what it asserts there, or not assert it.

## Related

- #210 — the junction this resolves through
- #211 — the migration this blocks
- #206 — diagram-view materialisation, which should share the refresh hook
