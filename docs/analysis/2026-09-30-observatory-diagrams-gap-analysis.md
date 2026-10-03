---
name: Observatory Diagrams — gap analysis
description: The 11 diagram views in the Observatory mockup, against what the index, the metric catalog and rokkit can actually deliver today
date: 2026-09-30
status: analysis
---

# Observatory Diagrams — what is built, what is missing

The Observatory project window gained a **Diagrams** section with eleven views in
two groups. This document compares each against three things that have to line up
before a view can ship: **the data** (is it in the index?), **the derivation** (is
there a query or metric that shapes it?), and **the component** (can rokkit draw
it?).

## Method, and what "verified" means here

Every claim below is one of:

- **(M)** measured — a query against the live `sensei` database on 2026-09-30, or
  a file read in this repository / `~/Developer/rokkit`.
- **(I)** inferred — a reasonable read of names and structure that I did not
  execute or open.

Rokkit component *existence* is (M), from the package file listing at
`@rokkit/graph@1.8.0` and `@rokkit/chart@1.8.0`. Whether a component's **props
fit our data shape** is (I) except where noted — I listed the components, I did
not read each one's API. That distinction matters for sizing the work and is
called out per row rather than smoothed over.

## Where the mockup actually lives

- Nav and grouping: `docs/mockups/Sensei/lib/project/project-pages.jsx:292-303`
  (`PROJ_DIAGRAMS`).
- The views themselves: `docs/mockups/Sensei/Sensei Schema and Call Graph v8.dc.html`,
  embedded in an iframe with `?tab=<calls|schema>&view=<view>` and driven by
  `postMessage({type:"sensei-diagram"})`.
- The six architecture views added most recently are gated by
  `const AXV={ dsm:1, layers:1, cycles:1, cochange:1, poly:1, own:1 }` (v8:1375),
  with their titles and subtitles in `AXH` (v8:1376-1389). Those subtitles are the
  clearest statement of intent and are quoted per view below.

## The scoreboard

| # | View | Group | Data | Derivation | Component | Status |
|---|---|---|---|---|---|---|
| 1 | Zones | Architecture | ✅ | ⚠️ partial | ✅ `ScatterPlot` | **Near-ready** |
| 2 | Dependency matrix | Architecture | ✅ | ❌ | ✅ `DependencyMatrix` | **Derivation only** |
| 3 | Layers | Architecture | ✅ | ❌ | ✅ `LayersDiagram` (1.9.0) | **Derivation only** |
| 4 | Cycles | Architecture | ✅ | ❌ no SCC | ⚠️ collapse unverified (1.9.0) | **Derivation only** |
| 5 | Hidden coupling | Architecture | ❌ **no git history** | ❌ | ✅ `ArcDiagram` (1.9.0) | **Blocked on data** |
| 6 | Complexity | Architecture | ⚠️ partial | ❌ | ✅ `PolymetricTree` (1.9.0) | **Derivation + data** |
| 7 | Ownership | Architecture | ❌ **no blame** | ❌ | ✅ `Treemap` | **Blocked on data** |
| 8 | Structure | Code | ✅ | ✅ | ✅ `StructureDiagram` | **SHIPPED 2026-10-01** |
| 9 | World | Code | ✅ | ✅ | ✅ `Graph` + pack | **Buildable now** |
| 10 | Neighbourhood | Code | ✅ | ✅ | ✅ `Neighborhood` | **Buildable now** |
| 11 | Schema | Code | ✅ | ✅ | ✅ `ErDiagram` | **Buildable now** |

**Four of eleven are buildable today** with no new data and no new rokkit work;
**one of those four has since shipped** (Structure, 2026-10-01).
**Two are blocked on a single missing input** — git history — and that one input
also unblocks the third dimension of a fourth (Complexity's churn axis).

> **Update 2026-10-03.** See [What changed since 2026-09-30](#what-changed-since-2026-09-30)
> at the foot of this document. One scoreboard column needs reading differently now:
> a ✅ under **Data** means the rows exist, NOT that a project-scoped screen will
> show them. Project membership moved in #211 and was briefly recorded nowhere on a
> fresh install — every diagram rendered empty over a fully indexed codebase.

---

## 1. Zones — abstractness vs instability

> *"Robert C. Martin's abstractness against instability. Modules on the diagonal
> are balanced; the further off it, the more a change will cost."*

**Data (M).** `sensei.abstractions` exists and was built for exactly this — it
derives abstractness from `implements`/`extends` EDGES rather than from
`kind='interface'`, because the keyword means two different things (rust records
a trait as `interface` and 21 of 21 have an implementer; TypeScript has 606 with
one between them). `Ca`/`Ce` are derivable from `sensei.edges` grouped by the
fqn's package segment.

**Gap.** There is no view that computes `A`, `I` and `D = |A+I−1|` per component
and persists them. The mockup already renders `sub:"I "+f2(m.I)+" · A "+f2(m.A)+"
· Ca "+m.Ca+" · Ce "+m.Ce` (v8:1971), so the shape is settled — it needs a
`component_zones` view.

**Component.** `ScatterPlot` + `abline` (chart has `abline.js` (M)) draws the
main sequence. `Region` can shade the two zones.

**Caveat worth stating.** `A` is only as good as `implements`/`extends`
resolution. Those are the two best-resolved kinds (92.4% / 97.9% on files indexed
since the cutover) but a seam whose implementers have not resolved is
under-reported. `abstractions`' own doc comment says so; the view must not present
`A` as exact.

## 2. Dependency matrix (DSM)

> *"Every dependency as a cell — the row uses the column — with foundations
> first. Marks below the diagonal point down the layers; anything above it is a
> cycle or a layer violation."*

**Data (M).** `sensei.edges` + the fqn's package/module segments. Everything
needed is present.

**Derivation gap.** Two parts, and the second is the hard one:
1. A module×module (and file×file — the mockup has `axDsm:"module"` as a state
   knob, so both grains) aggregation of edge counts.
2. **Seriation** — "with foundations first". A DSM is only readable if rows and
   columns are ordered so dependencies fall below the diagonal. That is a
   topological sort with cycle-breaking, not a `GROUP BY`.

**Component (M).** `DependencyMatrix.svelte` exists in `@rokkit/graph`. Whether
it accepts a pre-seriated order or does its own is (I) — needs reading.

## 3. Layers

> *"The intended layers, top to bottom, with every dependency between modules.
> Arrows should only point down; the vermillion ones climb."*

**Data (M).** Edges exist. **Layer assignment does not.**

The proposal from discussion — derive layers from call-graph depth, the way dbd
groups dependencies — is sound and needs no new indexing: longest-path depth over
the module dependency DAG gives a level per module, and conformance is "same
level or one level down". That is a recursive CTE over `edges`, which Postgres
does natively.

**Blocker to name.** Longest-path depth is only defined on a **DAG**. The module
graph has cycles (see view 4), so layering requires SCC collapse first. **Views 3
and 4 share a dependency and should be built together, cycles first.**

**Component.** No layered/Sugiyama layout in `@rokkit/graph` (M — not in the
component listing). This is a genuine new component.

## 4. Cycles

> *"Each group of mutually dependent modules collapsed into one node. What
> remains reads as a hierarchy, and each collapsed node names the weakest link to
> cut."*

**Data (M).** Edges exist.

**Measured, and encouraging:** at *symbol* level in sensei's own placed internal
call graph there are **29 mutual pairs and 23 triangles, and every one is inside a
single file** — recursive-descent tree walkers (`node ↔ children`,
`statement ↔ block`). **Zero cross-file, zero cross-package.** So the view's value
here is confirming health, not finding rot. On a client codebase it will find rot.

**Derivation gap.** No SCC computation anywhere. Tarjan's or Kosaraju's over the
module graph, persisted as a component id per module. "Names the weakest link to
cut" additionally needs an edge-weight heuristic (fewest occurrences / most
recently added).

**Component.** `Graph` can draw the condensed DAG, but a **collapsed node that
expands to its members** is the drill-down affordance discussed below — it is the
same gap as #12.

## 5. Hidden coupling (co-change) — BLOCKED

> *"Pairs that change in the same commits but have no import between them.
> Imports on the left, shared commits on the right — the vermillion arcs are
> coupling the graph cannot see."*

**This is the highest-value view in the set and it has no data at all.**

**Measured (M):** the `sensei` schema has **no commits table, no per-file
authorship, no co-change** — the only git-shaped columns anywhere are
`commit_sha` stamps on `metric_facts`, `project_metrics` and
`repository_metrics`.

What *does* exist (M) is the machinery: `tasks/handlers/metrics/churn.rs` already
shells out to `git log` per repository checkout root, resolved via
`repository_roots_for_project`, tolerating git's absence and emitting **no row**
rather than a fabricated value. Co-change extends that pattern; it does not
invent it.

**Note a catalog drift found on the way (M).** `sensei.metrics` still records
`churn_rate`'s formula as *"count(process_file task_executions) per file per
day"*, but `churn.rs`'s header says those metrics were migrated off
`task_executions` precisely because "a re-index spiked `churn_rate`". The code is
right and the catalog text is stale — worth fixing, because the stale formula is
what a reader of the metric would believe. (And it would have been actively
misleading this week: until the barrier fix landed today, every structure pass
re-parsed every file, so a `task_executions`-based churn would have been
measuring the indexer's own bug.)

## 6. Complexity (Lanza's System Complexity)

> *"The containment tree, with each file a box whose width, height and shade show
> three metrics at once."*

State knobs are explicit (v8): `axPW:"fns"`, `axPH:"loc"`, `axPC:"churn"` — width
= function count, height = lines, colour = churn.

**Data.** Two of three are available now: `sensei.nodes` has **`line_start` AND
`line_end`** (M), so per-symbol and per-file LOC is derivable, and function count
is a `COUNT(*)` over nodes of function kind. **Churn is the git gap again.**

**Component.** `Treemap` exists but a polymetric view is not a treemap — box
width and height encode *independent* metrics while position encodes containment.
New component.

## 7. Ownership — BLOCKED

> *"The codebase as a treemap sized by lines. Colour by main author, by how many
> people know a file, or by how long it has been left alone."*

Three colour modes (`axOwn`) = three derived facts: principal author, author
count, last-touched age. **All three need `git log`/`git blame` per file.** None
exist (M).

**Component (M).** `Treemap.svelte` exists — this is the one blocked view whose
component is already there. Sizing by lines works from `line_start`/`line_end`
today; only the colour dimension is blocked.

**Privacy note.** Author data is personal data, and this repo has a live
constraint about client identifiers in the tree. Ownership must store an author
*identity* the collective layer can anonymise — `crates/senseid/src/git_identity.rs`
already exists and should own that mapping rather than a second one appearing here.

## 8-11. Structure, World, Neighbourhood, Schema — buildable now

> **Structure shipped 2026-10-01.** The other three are unchanged and still buildable.

| View | Data | Component |
|---|---|---|
| Structure | file-level nodes + edges, module/crate grouping — all present | `StructureDiagram` + `BundleControl` (edge bundling, `stBeta:0.85` in the mockup) |
| World | all nodes; `wLayout:"graph"|"pack"` | `Graph`; circle-pack for the nested mode |
| Neighbourhood | `get_callers`/`get_callees`; `depth` state | `Neighborhood` + `DepthControl` |
| Schema | dbd-indexed DDL entities | `ErDiagram`, `EntitiesView`, `EntityView` |

These four need **no new data and no new rokkit component.** They need an API
endpoint and a UI screen. That makes them the right first slice.

Structure proved that claim: endpoint `d2e8f181`, screen `6f8ea574`, no indexer
change. It also showed the slice is not free — the first endpoint took 26,461 ms
until #206 reshaped the `graph_nodes` read (610,830 → 109,021 buffers, 863 ms),
and the coverage count was wrong in a way that looked precise. Budget derivation
work per view even where the data is "already there".

---

## The data gap, consolidated

Everything blocked traces to **one missing input: git history.**

| Needed | Feeds | Source |
|---|---|---|
| commits × files changed | co-change, churn | `git log --numstat` |
| per-file author attribution | ownership (author, author count) | `git log --format` / `git blame` |
| per-file last-touched | ownership (gone quiet) | `git log -1` |

One scanner produces all of it. It is **not** a new subsystem: `churn.rs` already
establishes the pattern (resolve repository roots → shell out to git → tolerate
absence → no row rather than a fake value), and `git_identity.rs` already exists
for author identity.

**What it should NOT be.** Not a metric computed on the fly per view — co-change
over 90 days of history is far too expensive to recompute per page load, and the
mockup's own meta line says *"git history, last 90 days"*. It wants a persisted,
incrementally-updated table keyed by commit, with the pair aggregation derived.

**Transcript scanner:** nothing here needs it. Every gap is git- or
graph-derived. The transcript feed already powers the session/outcome families
(18 of 30 metrics (M)) and is not the constraint for Diagrams.

## The tooling gap, consolidated

**Rokkit is further along than expected.** `@rokkit/graph@1.8.0` already ships
`DependencyMatrix`, `StructureDiagram`, `DependencyDiagram`, `Neighborhood`,
`CallTree`, `Treemap`, `Sunburst`, `ErDiagram`, plus `DepthControl`,
`BundleControl`, `DensityControl`, `EdgeStyleControl` and `ZoomControl` (M).
`@rokkit/chart@1.8.0` ships `ScatterPlot`, `Heatmap`, `Hexbin`, `BubbleChart`,
`Arc`, `Ribbon`, `Timeline`, `CrossFilter`, `FilterBar` and `ChartExporter` (M).

Four component gaps, and one cross-cutting one that matters more than all of them:

### The cross-cutting gap: drill-down is over pre-loaded data only

`GraphState` already has `focusPath: string[]`, `levels`, `depth` and `focus`
(M) — so the *navigation* primitives exist. What does not exist (M — no
`onDrill`, no loader, no `async`, no `Promise` in `GraphState.svelte.ts` or
`Graph.svelte`) is **a callback that lets the host supply the next level's data.**

That is precisely the control being asked for. A top-level dendrogram showing
crates / external libs / packages cannot hold the crate-level detail in the same
payload — at 1.18M nodes the mockup's own meta line is aspirational unless levels
load on demand. Without a drill event the choice is "load everything" (unusable)
or "one static level" (not a drill-down).

**Required shape:** `ondrill(path, node)` fired on expand, `ondrillup(path)` on
collapse, with the host free to return data asynchronously and the component
showing a pending state for that branch. `focusPath` is already the right
identifier to hand back.

### The four component gaps

> **CLOSED 2026-10-03 by `@rokkit/graph@1.9.0`.** All four now ship. See
> [Rokkit 1.9.0 closes the component column](#rokkit-190-closes-the-component-column).
> Kept below as the record of what was asked for.

1. **Layered DAG layout** (view 3) — ranks top to bottom, edges that climb styled
   as violations.
2. **SCC-collapsed graph node** (view 4) — a node standing for a group, expandable.
   Overlaps the drill-down work.
3. **Dual arc / co-change diagram** (view 5) — two arc sets over a shared axis
   (imports vs shared commits). `Arc` and `Ribbon` exist; whether they compose
   into this is (I).
4. **Polymetric tree** (view 6) — containment layout where width, height and
   colour are three independent metrics.

## Metric consolidation

The catalog is healthy and is the right substrate: `sensei.metrics` has 30 active
metrics with `family`, `purpose`, `how_to_read`, `direction`, `weight`, `target`,
`rating_scale` and `capture_source` (M).

Current families (M): `quality` 9, `outcome` 6, `usage` 5, `architecture` 3,
`velocity` 3, `autonomy` 2, `knowledge` 1, `cost` 1.

Against the areas named in discussion — quality, maintainability, performance,
resilience, velocity, effectiveness — the mapping is mostly a **regrouping, not
new metrics**:

| Area | Have | Missing |
|---|---|---|
| Quality | 9 | — |
| Maintainability | (inside `architecture`) | Martin's D, cycle count, layer violations — all from views 1-4 |
| Velocity | 3 | — |
| Effectiveness | `outcome` 6 (ftr, rework) | arguably already this, under another name |
| Performance | — | nothing measures runtime/build performance |
| Resilience | `autonomy` 2 | thin |

Two honest observations. **`family` is already doing this job** — the task is to
agree the vocabulary and remap, not to build a grouping mechanism. And
**Performance has no metrics at all**, so it is a new capture source (build times,
bundle size), not a regrouping — it should not be promised alongside the others.

## Suggested sequence

Forward-only: nothing earlier depends on anything later.

1. **Ship the four unblocked views** (Structure, World, Neighbourhood, Schema) —
   endpoints + screens. Proves the whole path end to end with zero new data.
   — **Structure done** (2026-10-01). World, Neighbourhood, Schema remain;
   Structure is the worked template for each.
2. **Drill-down events in rokkit** — needed by every view at real scale, and by
   #1 the moment a real project is loaded.
3. **Git history scanner** — one indexer addition, unblocks views 5, 7 and
   Complexity's third axis.
4. **SCC + layering derivation** — cycles first, then layers (layering needs the
   condensation).
5. **Zones + DSM derivations** — `component_zones` view, DSM seriation.
6. **The four rokkit components** — layered DAG, collapsed node, dual arc,
   polymetric tree.
7. **Metric family remap**, and decide whether Performance is in scope.

Each stage gets its own spec under `docs/spec/`, with issues sized to one
sitting, per the agreed definition of done.

## Open questions

- **DSM seriation**: computed in the database (stable, cacheable) or in the
  component (interactive reordering)? Affects whether `DependencyMatrix` needs a
  prop.
- **Co-change window**: the mockup says 90 days. Configurable, or fixed?
- **Author anonymisation**: how does Ownership interact with the collective's
  anonymisation? Must be settled before author data is persisted, not after.

---

## What changed since 2026-09-30

Measured on 2026-10-03 against the live `sensei` database and a from-scratch
`dbd deploy`, unless marked (I).

### Structure shipped, and what it cost

Endpoint `d2e8f181`, screen `6f8ea574`, closing #205. No indexer change, as this
document predicted. Two things it did NOT predict:

- **#206** — the first endpoint answered in 26,461 ms. The fix was the shape of
  the `graph_nodes` read, not an index: 610,830 → 109,021 buffers, 863 ms. My
  first three hypotheses were measurably wrong; the cause was an `edges` seq scan
  at 80% of cost.
- **Coverage counted the wrong population.** `graph_placement`'s outcomes are
  `placed`/`missed`/`no verdict` — there is no `'resolved'` — so `outcome <>
  'resolved'` matched every row: 69,747 unplaced reported where the truth is
  38,578, and 107 of the endpoint's 122 seconds. A precise-looking number that
  said nothing.

Read the "Derivation ✅" column accordingly: it means a query *exists*, not that
it is shaped for a screen's latency budget.

### A ✅ under Data no longer implies a screen will show anything

**#211 dropped `folders.project_id`.** A folder's project now resolves only
through `folders.repository_id` → `repositories_in_projects` → `projects`, exposed as
`sensei.folder_projects`. Every diagram endpoint is project-scoped, so every row
in the scoreboard now depends on that chain resolving.

It did not. On a from-scratch database with `~/Developer/sensei-hq` registered:
3 repositories, 3 projects, 2,430 files, **27,860 nodes indexed — and
`repositories_in_projects` empty**. Every project came back `repos_count: 0`, tagged
`orphaned`, and the Structure diagram returned an empty graph. `ProcessGitFolder`
ran before `scan_root`'s reconcile assigned repositories, so the membership write
matched no row. Fixed in `2321920d`; after the fix the same run gives junction 3,
`folder_projects` 457, diagram 1,760 nodes / 1,282 edges.

The lesson for the remaining ten views: **a diagram that renders empty is as
likely to be a membership failure as an indexing one**, and the two are
indistinguishable from the screen. Any new endpoint should be exercised against a
freshly deployed database, not only against this machine's long-lived one.

### New derivation surface that did not exist on 2026-09-30

- `sensei.structure_edges` and `sensei.structure_graph` — built for Structure,
  and the natural base for **Cycles, Layers, Zones and DSM**, all of which need
  the same project-scoped edge set. (I) None of those four has a view yet.
- `sensei.sole_project_of(uuid)` and `sensei.namespaces_for_folder(uuid)` — the
  two resolution helpers endpoints should use rather than re-spelling the joins.
- `scripts/check-sql-against-schema.py` — PREPAREs every SQL statement in the
  tree against a live schema. Worth running after adding any diagram view: it
  catches a dropped column or a dangling alias that the compiler cannot see.

### Still blocked, re-verified

Rows 5 (Hidden coupling) and 7 (Ownership) remain **blocked on git history**.
Confirmed on 2026-10-03: no `churn`, `author` or `commit` tables exist in
`sensei`, and nothing in `crates/senseid/src` computes co-change. The consolidated
data gap below is unchanged — one scanner still unblocks both plus Complexity's
third axis.

### Schema (row 11) has groundwork

`9cc8a240` specced database structure as graph facts, and
`docs/analysis/` carries the companion piece on what a schema contributes to the
code graph. Row 11's "Data ✅ dbd-indexed DDL entities" should be read against
that spec rather than assumed.

### Open questions, updated

The three at the foot of this document still stand. One is now more urgent:
**author anonymisation** must be settled before the git-history scanner is
written, not after it persists author data — the scanner is step 3 of the
sequence and the collective's anonymisation is not yet designed for it.

### Rokkit 1.9.0 closes the component column

Rokkit's diagram work landed. All ten `@rokkit/*` packages moved `1.8.0 → 1.9.0`
together (they pin each other exactly; a split version across them is its own
hazard) in `app`, `dojo` and `website`.

Verified by unpacking the published tarball and listing its components (M), not
from the release notes:

```
ArcDiagram  BundleControl  CallTree  DensityControl  DependencyDiagram
DependencyMatrix  DepthControl  DiagramFrame  DrillBar  EdgeStyleControl
EntitiesView  EntityView  ErDiagram  Graph  GraphLegend  HiddenControl
LayersDiagram  MeasureControl  Neighborhood  NoteBlocks  PolymetricTree
StructureDiagram  Sunburst  ToggleControl  Treemap  ViolationsControl  ZoomControl
```

Against the four gaps named above:

| Gap | 1.9.0 |
|---|---|
| Layered DAG layout (view 3) | ✅ `LayersDiagram`, with `ViolationsControl` for climbing edges |
| SCC-collapsed node (view 4) | ⚠️ no component named for it — `DependencyDiagram`/`Graph` may cover it; **(I), needs a read** |
| Dual arc / co-change (view 5) | ✅ `ArcDiagram` |
| Polymetric tree (view 6) | ✅ `PolymetricTree` |

`DrillBar`, `CallTree` and `Sunburst` also appear, which bears directly on the
cross-cutting drill-down gap above — whether `ondrill` now exists is **(I)** and
should be read before that gap is treated as closed.

**What this changes.** The Component column is no longer the constraint for any
view except possibly Cycles' collapse. What remains is **data** (git history, for
views 5 and 7) and **derivation** (SCC, layering, zones, DSM seriation — none has
a view yet). Views 3 and 6 move from "gap both ends" to derivation-bound.

Gates after the bump: app 1,710 unit tests pass, `svelte-check` **1,237 files, 0
errors, 0 warnings**, dojo 1,535 tests pass, website builds. The type check
matters more than the tests here — it is what caught the two errors the 1.6
upgrade shipped past a green suite.

---

## The dōjō dimension — diagrams as a TEAM view

Added 2026-10-03. The eleven views were analysed as a **local** project-window
feature. They are also wanted on the dōjō, as a team view. That is not the same
screen pointed at a different database, and it changes this plan.

### What the dōjō has today (M)

The `dojo` scope deploys 38 tables. For our purposes the relevant ones are
`repositories`, `repositories_in_projects`, `repository_elections`,
`repository_metrics`, plus the sharing machinery (`tenant_share_policy`,
`artifacts`, `shared_rules`).

**There is no graph substrate on the dōjō plane at all** — no `nodes`, no
`edges`, no `graph_nodes`, no `structure_edges`. And the sync carries none of it:
`sensei.sync_entity` is exactly

```
repository · project · project_repository · repository_metric
metric_catalogue · dojo_sync_plan
```

So today the dōjō can show a team's **metrics**, and nothing a diagram is drawn
from. Every one of the eleven views is built on nodes/edges the dōjō never
receives.

### The decision that has to come first

Three shapes, and they are not interchangeable:

1. **Sync the raw graph** — `nodes` + `edges` per repository. Every view works
   unchanged and new views need no sync work. But it is the largest payload by
   far (745k nodes on this machine alone) and it ships **file paths and symbol
   names** to a shared plane. Those are client-identifying: this repository has
   a private-names denylist precisely because paths and identifiers leak who the
   work is for.
2. **Sync derived diagram payloads** — the shape each endpoint already returns.
   Small, reviewable, and the anonymisation boundary is explicit. But the payload
   is fixed at sync time: a new level, filter or drill is a sync change, and
   per-view sync entities multiply.
3. **Sync aggregates only, compute on the dōjō** — e.g. module-level edge counts
   rather than symbol edges. Smallest and safest; limits the team view to
   coarser diagrams than the local one, which may be exactly right for a team.

**This is a product and confidentiality decision, not a technical one**, and it
should be settled before any diagram endpoint is written with sync in mind. It
also interacts with the author-anonymisation question already open at the foot of
this document — resolve them together.

### What it implies regardless of which shape wins

- **`sensei.sync_entity` gains variants**, and the enum is a DDL change in the
  shared closure — it reaches both planes.
- **New `dojo.*` tables** for whatever is carried, inside the `dojo` scope's
  `includes` in `design.yaml` (the scope is explicit; a table that is not listed
  simply does not deploy there — that is how the metric catalogue once shipped
  empty).
- **`tenant_share_policy` gains a say over graph data** — if paths or symbol
  names cross, the policy must be able to refuse them per tenant.
- **The dōjō needs its own endpoints and screens.** The app's diagram screens are
  Tauri-local and read `/api/projects/{id}/diagrams/*`; the dōjō is SvelteKit on
  Supabase with kavach auth and RLS. The *state* classes and the rokkit
  components are shareable; the loaders are not.
- **A team view is scoped differently.** Local diagrams scope by project; a team
  view scopes by tenant → team → projects, which is `team_projects` +
  `repositories_in_projects`, not `folder_projects`.

### Sequencing impact

This slots into the suggested sequence as a parallel track, not a replacement.
Shipping World / Neighbourhood / Schema locally still proves the path and is
unblocked. The dōjō track starts with the decision above, then the schema + sync
changes, then dōjō endpoints and screens reusing the same components.
