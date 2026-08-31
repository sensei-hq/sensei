# The project / namespace model — measured state and options

**Status:** analysis, no code moved. Written 2026-08-31 from live measurements
against the local `sensei` database (146 projects, 16,254 metric rows).

The question that prompted it: *do we need one project per repository, or should a
project be optional and only exist when several repositories genuinely group?* And
the follow-on: *if insights/memories/metrics are repository-level, can we add
project and organization rungs and aggregate for tiered sharing?*

The measurements answer the first and **invert the second**.

---

## 1. Measured state

### Projects have at most one repository, and most have none

```
projects with 2+ repositories:   0
projects with 1 repository:     67
projects with 0 repositories:   79
repositories with no project:    0
```

Two things follow.

**The multi-repo grouping is unused.** `sensei.folders` carries both
`project_id` and `repository_id`, so a project spanning N repositories is
already representable. It has never happened. Every capability built on it —
`repository_roots_for_project`, the per-repo loop in `churn`/`quality`, the
pooling in `project_metric_daily` — has only ever run with N = 1. The
multi-repo path is therefore *implemented and untested by use*, which is worth
knowing before relying on it.

**79 projects are scan residue, not decisions.** They have folders but no
repository:

| project | folders | repositories |
|---|---:|---:|
| `find-me-board` | 1230 | 0 |
| `pljava-1_6_7` | 109 | 0 |
| `value-pricing` | 77 | 0 |
| `stories` | 34 | 0 |

These are directories the scanner walked that never resolved to a git remote.
Nobody chose them. They inflate the project count by 54%, and every "per
project" loop pays for them.

### The namespace ladder is project-centric, and has no repository rung

```
sensei.namespaces.scope_key prefix:
  project       311
  technology     16
  organization    1
  general         1

sensei.namespaces.level:  NULL for all 329 rows
```

### How things are ACTUALLY keyed

| Plane | Keyed by | Evidence |
|---|---|---|
| metrics | `project_metrics.repository_id` — a **column** | 16,254 of 16,254 rows carry it |
| memories | `memories.scope` = `project` (14) / `global` (2) | no `repository` scope exists |
| memories → namespace | `namespace_id` set on **5 of 16** rows | the ladder is barely wired to memories |

---

## 2. The inversion

The premise behind "add `namespace:project` and `namespace:organization` and
aggregate" is that today's data sits at `namespace:repository`. **It does not.**

- Metrics are repository-scoped, but through a **foreign key**, not a namespace.
  Nothing joins a metric to `sensei.namespaces`.
- Memories are **project**-scoped — the rung the proposal would add is the one
  that already dominates (311 of 329 namespaces).
- There is **no `repository:` namespace**, so the rung the metrics actually live
  at is the one missing from the ladder.
- `level` is NULL everywhere, so the ladder has no machine-readable ordering. Any
  "aggregate up one rung" logic has nothing to walk.

So the work is not "add two rungs above a working repository level". It is:

1. introduce the `repository:` rung,
2. populate `level` so the ladder is ordered and walkable,
3. connect metrics (a `repository_id` column) and memories (a `scope` enum) to it,
4. *then* aggregation at project and organization becomes a traversal rather than
   a special case per plane.

Step 3 is the expensive one and the reason this needs a decision rather than a
patch: two planes currently express scope in two different vocabularies, neither
of which is the namespace.

---

## 3. Options for the project model

### A — a project is what a repository gets by default

Keep one project per repository as the *rule*, stop pretending otherwise, and fix
the residue: a scanned directory with no git remote does not become a project.

- **For:** matches reality; no migration of the 67 real projects; kills 79 shells;
  every existing per-project loop keeps working unchanged.
- **Against:** the multi-repo case stays theoretical, so grouping two
  repositories remains a manual act with no UI.
- **Cost:** deciding what happens to the 79 (delete, or mark
  `maturity='archived'`, or leave and filter). Their folders carry scan state, so
  deletion is not free — `find-me-board` alone is 1,230 rows.

### B — project becomes optional, with a view that presents repo-less repositories as projects

Let a repository exist without a project; a view synthesises a project-shaped row
so the existing project UI keeps working.

- **For:** no per-repo project rows at all; the "create a project" act becomes
  meaningful because it only happens when someone groups things.
- **Against:** a *synthesised* project has no id to hang a memory, a metric or a
  namespace on — and memories are already `scope='project'`. Either the view gets
  a stable synthetic id (which is a fabricated identity, and the no-fabrication
  rule says no) or every writer learns to handle a null project.
- **Cost:** high. Touches every `project_id` foreign key.

### C — optional project plus a real namespace ladder

Option B's data model, but the thing memories and metrics hang off is the
**namespace**, not the project. `repository:` and `project:` become rungs; a
repository always has the former, and the latter appears only when someone groups.

- **For:** this is the shape tiered sharing wants — aggregate by walking rungs,
  one mechanism for repository / project / organization / general.
- **Against:** the largest change. Requires §2 steps 1–4 first.
- **Cost:** highest, and it is the only option that makes the sharing tiers fall
  out naturally rather than being built per plane.

---

## 4. What breaks, concretely

Anything touching the project model has to account for:

- **311 `project:` namespaces** against 146 projects — already more namespaces
  than projects, so some are stale. Nobody has audited which.
- **`project_metrics.repository_id`** on 16,254 rows, and the five views that
  pool it (`project_metric_daily` and the three coarser grains,
  `metric_facts`).
- **`memories.scope='project'`** — 14 rows today, so cheap to migrate now and
  progressively less so.
- **`sensei.metric_status`** cross-joins repositories × metrics; it would need
  the same treatment as any other per-repository read.
- **79 empty projects' folders**, which hold scan cursors. Deleting a project
  cascades to them.

---

## 5. Recommendation

Take Option **A** now and Option **C** later, in that order, because A is the
only one that costs nothing to reverse and it shrinks the problem C has to solve:

1. **Stop creating projects for repo-less scans.** This is a bug fix, not a model
   change. It prevents the 79 from becoming 179.
2. **Decide the fate of the existing 79.** Archive is safer than delete: their
   folders carry scan state, and a project nobody chose is not obviously
   disposable data.
3. **Introduce the `repository:` rung and populate `level`.** Additive, testable
   on its own, and useful immediately — it gives `metric_status` and the sharing
   verdicts a common vocabulary.
4. **Then** decide whether project becomes optional. By then the ladder exists,
   memories can hang off a namespace instead of a project, and the choice is a
   small one rather than a rewrite.

The one thing worth *not* doing is adding `namespace:project` aggregation on the
assumption that `namespace:repository` is already there. It is not, and building
on it would produce aggregation over an empty set — which reads exactly like
"you have no insights" rather than "this was never wired".
