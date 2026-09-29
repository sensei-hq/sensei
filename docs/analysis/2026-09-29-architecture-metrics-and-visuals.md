---
name: Architecture metrics and visuals
description: What the code graph can answer today, what indexing must collect to answer the rest, and the metric/visual pairs that turn it into architectural judgement
date: 2026-09-29
status: draft
---

# Architecture metrics and visuals

The goal is the one stated in the brief: pick up a client codebase and say
something true about its structure — internal vs external dependency, density of
use, where the modularity actually is, and whether the graph is convoluted enough
to be an organisational problem rather than a code problem.

Everything below is measured against the live graph (735,475 nodes / 3,157,545
edges / 251 projects, 2026-09-29) rather than assumed. Where a number is absent
it is because the fact is absent, and that is said rather than estimated.

## 1. The finding that reorders everything

**The cross-cutting edges — the exact ones that reveal architectural problems —
are precisely the ones the resolver misses.**

Placed `calls` + `references` edges inside this repo, bucketed by how far they
reach across the directory hierarchy:

| reach | edges | share |
|---|---:|---:|
| same file | 24,627 | 69.7% |
| same top-2 directories | 10,723 | 30.3% |
| same top-level directory | 0 | 0% |
| **crosses the top level** | **0** | **0%** |

Not one placed edge in this repository crosses a crate boundary. That is not a
finding about the code — `crates/senseid` genuinely depends on
`crates/bootstrap`. It is a finding about the graph:

```
crates/senseid/src/paths.rs   imports   sensei_bootstrap::config::GITHUB_ORG   → UNPLACED
```

A fully-qualified, unambiguous, same-workspace import, and the ladder does not
place it. Repo-wide, `imports` edges are **2.3% placed** (5,766 of 249,985), and
**240,439 of the 244,219 misses carry no reason at all** — not classified as
external, not classified as anything. They never reached a verdict.

Classifying this repo's own 6,533 unplaced imports by what they name:

| the import names | unplaced | ours? |
|---|---:|---|
| inside the same crate (`crate::`, `self::`, `super::`) | 2,360 | yes |
| third-party or std | 1,890 | no — correctly unplaced |
| relative path (`./`, `../`) | 1,152 | yes |
| SvelteKit alias (`$lib`, `$app`) | 1,022 | yes |
| our own npm scopes (`@rokkit/`, `@kavach/`) | 59 | yes |
| another crate in this workspace | 50 | yes |

**4,643 of 6,533 (71%) name something inside this repo.** Only 29% are
legitimately outside the indexed world. The largest single bucket is
`crate::`/`self::`/`super::` — the *easiest* case there is, an explicit path
inside one compilation unit.

### Why this must be fixed before any architecture metric ships

Every metric in §4 is a ratio over dependency edges. Computed on today's graph
they do not merely lose precision — **they inevert**:

- **Instability** `I = Ce/(Ca+Ce)`. Efferent coupling counts edges leaving a
  component. Cross-component edges are ~0, so `Ce ≈ 0`, so `I ≈ 0` for
  everything. Every component would be reported **maximally stable**.
- **Distance from the main sequence** `D = |A + I − 1|`. With `I ≈ 0`, `D ≈ 1 −
  A`, so every concrete component lands in the **Zone of Pain** and every
  abstract one looks ideal. The diagram would be a vertical line on the left
  edge, and it would be an artefact of the indexer, not the architecture.
- **Propagation cost** — transitive reachability over a graph with no
  cross-boundary edges is near zero. The most tangled codebase on earth would
  score as perfectly decoupled.

A dashboard that reports "perfectly modular" because the edges proving otherwise
are missing is worse than no dashboard: it is confidently wrong, on exactly the
question a client is paying to have answered. This is the
`no-fabrication`/honest-empty rule applied to a metric rather than a row.

**So: resolution of internal cross-boundary references is not one item on the
list below. It is the precondition for the list.** Issues #146, #151, #192 and
#193 are all facets of it; #193 (a repo-scope resolve barrier, one resolver pass
after all files) is the structural fix, because a `crate::` path can only be
placed once every file in the crate has been seen.

## 2. What the graph answers today, honestly

Real capability, not aspiration:

| question | answerable now | on what |
|---|---|---|
| What exists, and how is it nested? | **yes** | 735k nodes with `parent_id`; file → class → method, doc → section |
| Which symbols are exported? | **yes** | `is_exported` |
| Is a target inside or outside the indexed world? | **yes, for placed edges** | `graph_nodes.locality` — 665,111 internal / 481,521 external |
| What libraries does a project depend on, at what versions, with conflicts? | **yes** | `libraries` (1,905), `library_versions`, `project_library_version_conflicts`, `folder_dependencies` |
| Which libraries are actually used vs merely declared? | **partly** | `library_usage` / `referenced_libraries` — 7,020 rows |
| How confident is each edge? | **yes, and this is unusual** | `resolved_via` (8 rungs) / `unresolved_reason` (10 codes, `fault` vs `refusal`) |
| Who calls this symbol / what does it call? | **yes, within a file or directory** | see §1 for the boundary |
| Is this file test, doc, or production? | **yes** | `is_test`, folder `role` |
| What clusters does the graph fall into? | **recorded, trust it carefully** | `community_id`, 37,700 communities — but #149 measured 62% of them as restatements of the file boundary and 36% as stub singletons |

The verdict columns are worth dwelling on, because they are the thing most code
graphs lack. Every edge can say *how* it was placed — `declared_here` (77,462)
versus `in_the_prelude` (36,832) are both "resolved" and are not the same claim —
or *why* it was not, split into `fault` (6 codes, someone can close it) and
`refusal` (4 codes, correctly unplaceable). **That makes every metric below
able to publish its own confidence**, which is what separates a number a
consultant can defend from one they cannot.

## 3. What indexing must collect

Ordered by how much each unlocks, not by effort.

### 3.1 Resolve internal cross-boundary references — *the precondition*

Covered above. Unlocks: every component metric, propagation cost, cycles,
internal-vs-external as a real ratio, impact analysis worth running.

**Fact needed:** none new. This is resolver work, not collection — the edges are
already recorded, with their target names, waiting to be placed.

### 3.2 Per-file magnitude — LOC, and complexity per symbol

`sensei.files` carries `file_path`, `mtime`, `content_hash`, `indexed_at` and
nothing about size. Every "area = magnitude" visual in §5 currently has to fall
back on declaration counts, and every hotspot metric needs complexity.

**Facts needed:** `files.loc`, `files.bytes`; `nodes.complexity` (cyclomatic) and
`nodes.line_end − line_start`. qlty already computes complexity but only
project-wide (#156), so the per-symbol number is new.

### 3.3 Git history — commits, authors, churn

**There are no git tables at all.** This is the single highest-value *collection*
addition for client work, because it unlocks a family of metrics that need no
resolver improvement whatsoever:

- **Hotspots** (complexity × change frequency) — where refactoring actually pays
- **Change coupling** — files that change together but have no edge between them.
  This is the direct empirical measure of the brief's thesis: a hidden
  cross-cutting dependency that the *structure* does not show but the *history*
  does. It is often the single most persuasive artefact in a client review.
- **Knowledge map / bus factor** — ownership concentration per module
- **Code age** — stable core vs churning edge
- **Architecture erosion over time** — every metric below, as a trend

**Facts needed:** a `commits` table (sha, author, timestamp) and `commit_files`
(sha, path, added, deleted). Cheap to collect (`git log --numstat`), no parsing,
no language support, works on any repo on day one of an engagement.

### 3.4 Build artifacts — bundle size and build time

Nothing in the graph knows either. Both were named in the brief.

- **Bundle size** needs the bundler's own output: a `stats.json` (webpack/rollup
  /vite `--metafile`), read per artifact and attributed to modules. Without it,
  "impact on bundle size" can only be approximated by transitive dependency
  weight, which is a different question wearing the same words.
- **Build time** needs the build tool's timing (cargo `--timings`,
  `tsc --diagnostics`, turbo/nx profiles) attributed to units.

**Judgement:** these are adapter work against tools that already emit the data,
not new analysis. They are lower priority than §3.3 because they are per-ecosystem
and only apply where a build exists — but for a JS/TS client engagement, bundle
size is frequently the presenting complaint.

### 3.5 Abstract vs concrete classification

`A` in the Zone-of-Pain plot needs a per-language rule for what counts as
abstract. The inputs exist in this repo — 645 `interface`, 692 `struct`, 227
`class`, 472 `type`, 209 `enum` — but no node says "abstract". Note that 0 nodes
carry kind `trait` despite the enum having it, so Rust traits are landing
somewhere else and the rule cannot be assumed.

**Fact needed:** an `is_abstract` flag written by each language walk, with a
stated rule per language (Rust `trait`; TS `interface`/`type`/`abstract class`;
Java/C# `interface`/`abstract`; Python `ABC`/`Protocol`). A guessed rule makes
`A` unfalsifiable, so this is a walk change, not a heuristic.

### 3.6 Schema entities from dbd — *and this is not blocked*

I previously called the schema/ER view blocked. That was wrong, and the spec
already says why: **`docs/spec/indexer/20-schema-entities.md` needs no new table
and no enum change.** A table is a `struct` node with `declared_type = 'table'`,
a column is a `field` with `parent_id` to its table, an index is a `property`, a
foreign key is a `references` edge. The two-level vocabulary the SQL adapter
already uses carries all of it.

dbd **v0.22.0 is released** and exposes exactly the entry point needed:

```rust
dbd_core::schema_model::build(design: &Design, scope: Option<&ResolvedScope>) -> SchemaModel
```

Pure, offline, no database connection — given a `design.yaml` and its `ddl/`
tree it returns `tables` (with `columns`: `pk`/`nn`/`fk`/`uq`/`def`/`ty`, and
`indexes`: `def`/`unique`/`name`), `entities` (views, matviews, routines), `refs`
(column-to-column foreign keys) and `deps` (a pre-resolved read/write/call graph
whose `kind` vocabulary — `reads | writes | calls | member` — already matches
`facts::RefKind`).

The workspace is on **dbd-core v0.19.0**, so the work is:

1. Bump to v0.22.0 (both `crates/senseid` and `crates/bootstrap`; today's usage
   is only `parser::Dialect`, `parse_sql_as`, `design::Progress` and
   `adapter::postgres`, so the surface at risk is small).
2. Add `Rung::StatedBySchema` — the spec argues this correctly: the eight
   existing rungs all answer *"what in this file told us where the target
   lives"*, and a `SchemaModel` arrives already resolved with no file behind it.
   Reusing `DeclaredHere` would make the audit trail lie.
3. `Symbol::schema` (`SchemaFacts`) persisted into `props`.
4. Emit from a dbd-managed folder. Both producers run — the model has the
   resolved structure, the walk has the spans and file attribution, and neither
   is a superset.

One decision must be made first, and the spec flags it: 1,291 columns for 121
tables is ~10× the node count for the SQL half, and `nodes.embedding` is
`vector(384)`. Recommendation stands — **tables, views and routines embed;
columns and indexes do not.**

## 4. The metric catalogue

Each row: what it tells a reader, what it needs, and what it would say today.
"Component" means a package node where a manifest defines one (7,966 exist),
otherwise the top-level module.

### 4.1 Martin's component metrics

| metric | formula | needs | today |
|---|---|---|---|
| **Ca** afferent coupling | components depending *on* this one | §3.1 | ~0 — unusable |
| **Ce** efferent coupling | components this one depends on | §3.1 | ~0 — unusable |
| **I** instability | `Ce / (Ca + Ce)` | Ca, Ce | would report everything maximally stable |
| **A** abstractness | abstract types / all types | §3.5 | inputs present, rule absent |
| **D** distance from main sequence | `\|A + I − 1\|` | A, I | the Zone-of-Pain plot, blocked on both |
| **SDP** stable-dependencies violations | edges from lower-I to higher-I | I | the *actionable* form of D — each violation is one edge to name |
| **SAP** stable-abstractions violations | stable (low I) but concrete (low A) | A, I | |
| **ADP** acyclic dependencies | SCCs in the component graph (Tarjan) | §3.1 | the highest-value item here — a cycle is a defect with a name, not a score |

On **SDP over D**: `D` gives a component a number, which invites arguing about
the number. SDP yields *a list of specific edges pointing the wrong way*, each of
which is a concrete piece of work. For a client engagement the list is the
deliverable and the scatter plot is the executive summary of it.

### 4.2 Structural metrics beyond Martin

| metric | what it says | needs | note |
|---|---|---|---|
| **Propagation cost** (MacCormack/Baldwin/Rusnak) | density of the transitive dependency closure — "if I change a random file, what fraction of the system can feel it" | §3.1 | the best single number for *"is this convoluted"*; a percentage a non-engineer understands |
| **Core / Peripheral / Shared / Control** (same paper) | classifies every file by fan-in/fan-out relative to the largest cycle | §3.1 | turns propagation cost into a map of *which* files are the problem |
| **Fan-in / fan-out, Henry–Kafura** | information flow through a unit | §3.1 | cheap once edges resolve |
| **Modularity Q** (Newman) | how well the detected communities match the actual edges | §3.1 | sensei already clusters, but #149 measured 62% of communities as restating the file boundary — Q would have said so numerically |
| **Community ↔ folder agreement** | do the natural clusters match the directory layout? | §3.1 | **the brief's thesis, as a number.** High agreement = the structure reflects reality. Low = the folders are a fiction and the real coupling runs elsewhere |
| **Layering violations** | edges pointing against a declared layer order | §3.1 + a declared layer map | needs the client to state intended layers; the gap between stated and actual is the finding |
| **Dependency freshness / drift** | how far behind each library is | `library_versions` | **answerable today** |
| **Declared-but-unused / used-but-undeclared** | manifest vs actual imports | `library_usage` + §3.1 | partly answerable today |

### 4.3 Evolutionary metrics (need §3.3 only — no resolver work)

| metric | what it says |
|---|---|
| **Hotspots** (Tornhill) | complexity × change frequency — where refactoring pays, ranked |
| **Change coupling** | files that change together without an edge between them — hidden cross-cutting, the thing structure alone cannot see |
| **Knowledge map / bus factor** | ownership concentration per module; organisational risk |
| **Code age** | stable core vs churning frontier |
| **Conway alignment** | do team boundaries match module boundaries? (`change coupling` across authors) |

These deserve emphasis: they need **no resolver improvement at all**, they work
on any repository from day one, and in a client review they are usually the most
persuasive artefacts because they describe behaviour rather than opinion.

### 4.4 Design-level smells

Lower priority — these are per-symbol judgements that need §3.2 and a per-language
rule, and they tend to generate argument rather than agreement: God class (size +
fan-in), Feature envy (a method using another type more than its own), Shotgun
surgery (change coupling + fan-out), LCOM cohesion.

## 5. Visuals, and which question each answers

A visual is only worth building if it answers a question the data can support.
Paired accordingly. (The Observable notebooks named in the brief are referred to
by idiom — force-directed, radial tree, sunburst, circle pack — rather than
described as read.)

| question | visual | why this one | status |
|---|---|---|---|
| What is in here, and how big? | **circle pack** / **sunburst**, area = LOC or declarations | containment + magnitude in one read; pack labels legibly at depth, sunburst shows proportion better | **buildable now** (area = declarations until §3.2) |
| Where does the hierarchy break? | **hierarchical edge bundling** — radial tree of the hierarchy, cross-cutting edges bundled through the centre | the single best fit for the brief's thesis: hierarchy on the rim, every violation of it drawn through the middle. Convolution is *visible as ink density* | needs §3.1 |
| How tangled is it, at scale? | **DSM / adjacency matrix**, ordered by clustering | node-link fails past ~1k nodes; a matrix does not. Cycles appear as blocks below the diagonal | needs §3.1 |
| Architectural health per component | **A vs I scatter** with the main-sequence line, Zone of Pain and Zone of Uselessness shaded; dot area = size | Martin's own diagram. Add a **ranked SDP violation list** beside it — the plot persuades, the list is the work | needs §3.1 + §3.5 |
| Internal vs external dependency | **disjoint force-directed graph**, internal/external encoded, sized by usage count | disconnection is meaningful here — an isolated cluster is either a clean boundary or dead code | partly now (locality is real for placed edges) |
| Where should we refactor first? | **hotspot map** — circle pack, area = complexity, colour = change frequency | Tornhill's; needs no resolver work | needs §3.2 + §3.3 |
| What changes together? | **change-coupling chord / matrix** | reveals the coupling the structure denies | needs §3.3 |
| What does one symbol touch? | **neighbourhood** — callers left, callee right, by depth | already designed in the v3 mockup; the daemon carries the verdict as of `cfcefe62` | **buildable now** |
| What is the data shape? | **ER diagram** — tables as cards, FK edges | `@rokkit/graph`'s `cluster` layout is built for exactly this | needs §3.6 |
| Where is the graph weakest? | **any of the above, shaded by unresolved share** | the mockup's SHADE BY. Publishing the metric's own confidence beside it is what makes it defensible | needs rokkit #164 |

Three notes on rendering:

- **Rounded rects, not circles**, for the containment views — a circular cluster
  discards `1 − π/4` of its bounding box before anything is placed in it, and
  `@rokkit/graph`'s own `points.ts` already measured and acted on that. A rect
  also holds a legible label at small sizes.
- **Force-directed is for exploration, not for reporting.** It is
  non-deterministic, so the same codebase looks different each run and two
  reviews cannot be compared. For anything a client keeps, prefer a deterministic
  layout (the hierarchy, or a matrix with a stable ordering).
- **rokkit gaps are filed**: #161 (size by a supplied measure), #163 (containment
  beyond two levels + drill-down), #164 (a quantitative shade channel), #160,
  #162. The containment and shading views depend on them.

## 6. Sequence

Forward-only; nothing earlier depends on anything later.

1. **Resolver: internal cross-boundary references** (#193's repo-scope barrier
   first — a `crate::` path cannot be placed until every file in the crate is
   seen). Gate: this repo's placed edges must cross a crate boundary at all, and
   the 2,360 same-crate unplaced imports must go to ~0.
2. **Git history collection** (§3.3). Independent of 1 — can run in parallel, and
   delivers the §4.3 family on its own.
3. **Per-file LOC + per-symbol complexity** (§3.2). Unlocks honest magnitude in
   every visual and the hotspot map with 2.
4. **Component metrics** (§4.1) once 1 lands — Ca/Ce/I first, ADP cycles next
   (highest value, no §3.5 dependency), A and the Zone-of-Pain plot after §3.5.
5. **Propagation cost + core/periphery** (§4.2) — same inputs as 4, different
   question, and the better executive number.
6. **Schema entities** (§3.6) — independent of all of the above; its own vertical
   from dbd v0.22 through to the ER view.
7. **Bundle size / build time adapters** (§3.4) — per-ecosystem, last, driven by
   whichever client engagement needs it first.

## 7. What this corrects

- The schema/ER view is **not blocked**. The spec resolved the storage question
  (no new table, no enum change) and dbd v0.22 ships the offline builder. The
  work is a version bump, one new rung, and a producer.
- The architecture metric family (#156 steps 4–6) is **not merely unimplemented,
  it is currently unsafe to implement**: computed on today's edges it would
  report every component as maximally stable and most as being in the Zone of
  Pain, and both would be artefacts of the resolver rather than facts about any
  codebase.
