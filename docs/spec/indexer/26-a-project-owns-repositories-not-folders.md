---
name: A project owns repositories, not folders
description: project_id lives on every folder, so a repository's folders can disagree about their project — and one already does
date: 2026-09-30
status: ready
---

# A project owns repositories, not folders

## The defect

The conceptual model is **project 1:n repository 1:n folder**. The implementation
maps a project straight to *folders*:

| | |
|---|---|
| `sensei.repositories` has a `project_id`? | **No** |
| project↔repository mapping table? | **None** |
| the only link | **`folders.project_id`**, on all 13,697 folders |

Nothing enforces that a repository's folders agree about their project, so
nothing stops them drifting.

## It has already drifted

```
/Users/Jerry/Work/pre-sales/swarco/documentation
  → project "swarco"        | git    |   1 folder    ← the repo ROOT
  → project "documentation" | folder | 362 folders   ← everything beneath it
```

One repository, two projects. 285 of 286 repos are consistent; this one is not,
and nothing prevents the next.

Five projects also span two repositories each (`fitness` 7 modules, `swarco` 6,
`website`, `database`, `documentation.wiki`). That is **legitimate** under
project→repository — it is only expressed folder-by-folder today.

## Why it must be fixed before more is built on it

Anything scoping by `folders.project_id` inherits the drift. In #209 the
resolver's `World` was scoped by project, which in those five multi-repo projects
would have made **repo B's packages first-party to repo A's files** — producing
false edges. For a diagram a false edge is worse than a missing one. That code
now scopes through `repo_anchor_for`, but every future consumer faces the same
trap.

## `repository_id` sparseness is NOT the problem

`folders.repository_id` is set on 185 of 13,697 rows, which looks like missing
propagation. It is deliberate — the column comment says *"Set ONLY on the
repo-root/checkout folder (I16); subfolders resolve via nearest ancestor
(`repo_anchor_for`)"*, and `sensei.repo_anchor_for(path)` exists as *"THE shared
repo-anchor mapper… the ONE implementation"*. **No propagation is needed, and
adding it would be a second source of truth.**

## What the write surface actually is

Four production sites, which is what makes this tractable:

| site | what it does |
|---|---|
| `folders.rs:194` `upsert_subfolder_kind` | INSERT … project_id |
| `folders.rs:498` `upsert_folder` | INSERT … project_id |
| `folders.rs:289` `set_folder_project` | UPDATE folders SET project_id |
| `projects.rs:474` merge | UPDATE folders SET project_id WHERE project_id = old |

1,233 Rust and 221 DDL mentions of `project_id` exist overall, but they are
overwhelmingly the `projects` table and metric rollups — **readers**, which this
change leaves working.

## Prerequisite, checked

100 repo-anchor folders have no `repositories` row (10 git, 90 standalone, 1
subtree). The 10 git anchors hold **1,704 nodes** and matter; the 90 standalones
hold **zero** and do not.

They are not unlinkable: existing `repositories` rows carry **empty**
`repo_key`/`remote_url`, and the 10 have no git remote — so a row does not
require a remote, those 10 simply never got one. A backfill, not a blocker.

## Design

**Source of truth moves to the repository. A folder's project becomes derived.**

1. **DDL** — `repositories.project_id` → `sensei.projects(id)`.
2. **Backfill**, in order:
   a. a `repositories` row for every repo-anchor folder that lacks one;
   b. `repositories.project_id` from the **anchor folder's** current
      `project_id` — the ROOT wins, which resolves swarco to `swarco`;
   c. re-derive `folders.project_id` from the anchor so subfolders agree.
3. **Writers** — folder upserts stop taking a project from the caller and derive
   it from the repo anchor. `set_folder_project` is replaced by
   `set_repository_project`.
4. **Guard** — a test asserting no repository's folders disagree, so the drift
   cannot return.

### `folders.project_id` is KEPT, as a derived column

Dropping it would mean auditing ~1,400 mentions. Keeping it derived — written in
one place, from the repository — gets the correctness guarantee now and leaves
the column free to be dropped later. **The guarantee is that it can no longer be
set independently**, not that it stopped existing.

## Done gate

1. `repositories.project_id` exists and is non-null for every repository that
   owns indexed nodes.
2. Every repo-anchor folder holding nodes has a `repositories` row.
3. **No repository's folders disagree about their project** — asserted by a test
   over the live shape, not by reading the code.
4. The swarco split is resolved: all 363 folders of that repo sit in one project.
5. A folder upsert cannot set a project that contradicts its repository —
   mutation-probed.
6. Full suite green; sensei re-indexes and `structure_edges` counts do not fall.

## Wrong gate

- **`folders.project_id` still settable independently.** The column surviving is
  fine; a second writer is not.
- **Project derived per folder from its own path** rather than from the
  repository row — that is the same drift with extra steps.
- **The 90 empty standalones treated as a blocker.** They hold zero nodes;
  requiring rows for them before proceeding is cost with no benefit.
- **Backfill taking the majority folder's project.** The swarco repo's 362
  subfolders outvote its root, which would move the repository into
  `documentation` — the root is the repository's identity and must win.

## Related

- #210 — this issue
- #209 — the world scoping that exposed it
- `docs/backlog.md` — the interrupted-scan and `IS NOT DISTINCT FROM` findings
  from the same investigation
