# 構 · Diagrams · Structure

**Segment:** 04 · Observatory / Project window
**Route:** `/project/[id]/diagrams/structure`
**Source mockup:** [`lib/project/project-pages.jsx`](../../mockups/Sensei/lib/project/project-pages.jsx) → `PROJ_DIAGRAMS` `diag-structure`, rendered by [`Sensei Schema and Call Graph v8.dc.html`](../../mockups/Sensei/Sensei%20Schema%20and%20Call%20Graph%20v8.dc.html) view `structure`
**Data:** `sensei.graph_nodes` + `sensei.edges`, both already populated. **No new indexing.**
**App file:** _greenfield_
**Daemon files:** _greenfield_ — no HTTP handler serves a structure payload yet
**Rokkit:** `StructureDiagram` + `BundleControl` — both ship in `@rokkit/graph@1.8.0`. **No rokkit work required to ship v1**, but the app was pinned at `^1.6.0`, which exports neither; all ten `@rokkit/*` packages were moved to 1.8.0 together (a split version across them is its own hazard).
**Status:** SHIPPED 2026-10-01 (`d2e8f181` endpoint, `6f8ea574` screen). The FIRST screen of the Diagrams set precisely because it needs nothing new from the indexer or from rokkit.

## Purpose

Show where the code's hierarchy holds and where it breaks. Files sit on the rim
grouped by module and crate; edges bundle through the hierarchy they travel.
Tight bundles follow the structure. **Lines through the centre cut across it** —
those are the finding.

This screen is first because it is the whole path — index → derivation →
endpoint → screen → verification — with **zero** new data and **zero** new rokkit
components. It proves the pipeline end to end before anything harder is attempted.

Kanji is 構 — *structure*.

## Data invariants

- **Nodes come from `sensei.graph_nodes`**, which already carries everything the
  view needs: `fqn`, `name`, `kind`, `language`, `file_path`, `locality`,
  `project`, `parent_id`/`parent_name`/`parent_kind`.
- **Hierarchy is the fqn**, not the filesystem. `fqn` segment 2 is the package
  and segment 3 the module (the same decomposition `abstractions.component` and
  `graph_placement` already use). The filesystem path is for display only —
  deriving the hierarchy from directories would disagree with the graph on every
  workspace member.
- **Only placed edges are drawn.** `edges.target_id IS NULL` means the resolver
  reached no conclusion; drawing those would assert a relationship the index does
  not have. Measured on sensei 2026-09-30: calls are 44.7% placed, so this
  discards more than half the rows — that is correct, and the count of what was
  discarded must be shown (see Signals), never silently dropped.
- **Internal by default.** `locality='external'` nodes are library surface, not
  this project's structure. They are a toggle, off by default.
- **An unindexed project renders an empty state, never a partial graph.**
  `folder_completeness` says whether the scan finished; a structure diagram of a
  half-parsed repo shows fake islands, which is worse than showing nothing.

## Signals shown

- **The diagram** — files on the rim, grouped module → crate, edges bundled with
  tension bound to `BundleControl` (mockup default `stBeta: 0.85`).
- **Edge-kind filter** — `calls` / `references` / `imports`, matching the
  mockup's `stKinds` (`local`, `crate`, `cross`, `external`, `bridge`).
- **Coverage line, stated plainly.** "N edges drawn · M unplaced (not shown)"
  reading from `sensei.graph_placement`. Without it a sparse diagram reads as a
  simple codebase rather than an unresolved one.
- **Selection panel** — on click, the file's module and crate, its in/out degree,
  and its cross-cutting edges listed.
- **Empty / loading / error**, as three distinct states. A DB failure must not
  render as an empty graph (`CLAUDE.md`: honest-empty only when genuinely empty).

## Derivation

One new read-only view, `sensei.structure_graph`, so the endpoint does no
assembly:

- one row per internal node with `project`, `fqn`, `package`, `module`, `name`,
  `kind`, `language`, `file_path`
- plus a companion `sensei.structure_edges` with `source_fqn`, `target_fqn`,
  `kind`, `occurrences`, and a `span` flag saying whether the edge stays inside
  its module, crosses modules, or crosses packages

`span` is computed once here rather than in the client, because it is the thing
the view is *about* and three consumers would otherwise each re-derive it.

## API

`GET /api/projects/:id/diagrams/structure?level=file|module|package&kinds=calls,references`

Returns `{ nodes, edges, coverage: { drawn, unplaced }, level, kinds }`.

**`module` MEANS THE MODULE'S TOP SEGMENT.** Written as this spec first had it —
grouping on the fqn's module segment whole — the level does not collapse
anything: measured on sensei, 1,751 files become 1,446 groups (1.21x) with 1,390
of them holding exactly one file, because that segment is per-file across most
of the tree. Its FIRST segment gives 150 groups over the same files (11.7x), and
they are the units a reader names (`senseid/tasks`, `senseid/api`,
`@sensei/desktop/routes`). `package` was added as the third level (16 nodes,
109x) once the middle one earned its place.

`level` and `kinds` are VALIDATED — an unrecognised value is a 400, never an
empty graph. An empty graph would be a third silent meaning alongside "no edges
here" and "nothing resolved", which are the two this screen exists to separate.

`kinds` defaults to `calls` alone: all five at file level is 3,215 edges over
1,751 nodes, which is the hairball named under Wrong gate.

Each node carries `path` — package → top module → leaf, truncated at whatever
level the node IS. The diagram draws its rim from it, derived server-side so
three consumers cannot each re-derive it.

Fails with 500 on a DB error — never an empty payload
(`map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?`, the daemon idiom).

## Done gate

1. `GET /api/projects/<sensei>/diagrams/structure` returns ≥ 1,700 file nodes
   (sensei has 1,747 parsed files) and a non-zero edge count.
2. The screen renders sensei's own structure with modules visibly grouped.
3. The coverage line shows a non-zero unplaced count and matches
   `graph_placement` for the same project.
4. Switching `level=module` collapses to modules; the node count drops by at
   least an order of magnitude. **Measured: 1,751 → 150, 11.7x** — and only
   because `module` means the module's top segment (see API). Grouping on the
   whole module path gives 1.21x and fails this gate.
5. Selecting a file shows its module, crate and degree.
6. Loading, empty and error states each reachable and distinct.
7. e2e passes against the live daemon with the real index.

## Wrong gate

- **A dense hairball.** If every edge is drawn at file level with no bundling the
  picture is unreadable and the screen has failed even though the data is right.
- **Unplaced edges silently dropped.** A sparse diagram that does not say what is
  missing misrepresents the index as complete.
- **Hierarchy taken from directories.** Disagrees with the graph for every
  workspace member; the fqn is the source of truth.
- **Empty state on a DB error.** Indistinguishable from an unindexed project.
- **Loading everything at symbol level.** sensei is 27,441 nodes at project
  grain; symbol level across a client monorepo is not renderable and must wait
  for drill-down ([rokkit#165](https://github.com/jerrythomas/rokkit/issues/165)).

## Not in scope

Drill-down into a module (rokkit#165), and any view that needs git history or a
new rokkit component. This screen ships on what exists today.

## Related

- [Observatory Diagrams gap analysis](../../analysis/2026-09-30-observatory-diagrams-gap-analysis.md)
- rokkit#165 — drill-down events, which this screen will adopt when it lands
