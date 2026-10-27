# Code Map · M17

**Plane:** delivery · **Status:** [roadmap/03-delivery.md](../../../../roadmap/03-delivery.md) ·
**Serves:** solo developer ([persona 1](../../../03-personas.md#1--solo-developer--the-wedge)),
team developer ([persona 2](../../../03-personas.md#2--team-developer--the-inheritor)),
maintainer ([persona 3](../../../03-personas.md#3--maintainer--the-curator)),
manager / delivery lead ([persona 6](../../../03-personas.md#6--manager--delivery-lead--the-roll-up), through the L0 view)

## Purpose

The Code map is the one spatial view of the **code graph** that M4 builds
(`sensei.nodes` / `sensei.edges` + `inference.communities`). It replaces three surfaces
that each drew the same graph a different way: the **Atlas** canvas, the separate
**Diagrams** list, and the diagram-style charts inside **Metrics** (M8). Each used its
own grouping and none held up at scale. Real workspaces carry thousands of symbols and
tens of thousands of edges, and a node-link picture of all of them is a hairball.

> **The map opens on modules, never on symbols. You go down a level by choice.**

One canvas, three levels, and a set of **lenses** on the same canvas. The workspace
window's Diagrams list (M16) and the dōjō's Workspace › Code health tab (T3) both embed
this one map. Schema (Database · ER) is not part of it: tables, columns and keys are
not the call graph.

> **Build status, per part (2026-10-27).** The merged map exists in the desktop app, from
> the shared package `packages/codemap` (`@sensei/codemap`); the dōjō embed does not yet.
>
> | Part | Status today |
> |---|---|
> | L0 · L1 · L2 levels | **built** — `/workspace/[id]/diagrams/codemap` over `GET …/diagrams/codemap` (L0: ≤ 60 blocks, "+N more" per group, hub lane, one band per pair), `/codemap/module` (L1: role columns ≤ 12) and `/codemap/file` (L2: symbols, callers/callees ≤ 7); breadcrumb; list mode under 640px. Atlas and World are retired. |
> | Structure | **built** on the canvas: group by Layers or Folders, bands coloured by the layering's direction (upward red, skip amber). The role bar on L0 blocks is not built (roles exist at L1). The older Structure and Layers diagrams remain beside it. |
> | Wheel | **built** as a lens: modules on a ring per group, bands bundled through group anchors. Grouping by Communities still needs Fix 2 (M4). |
> | Zones | **built** as a lens over `GET …/diagrams/zones`. Illustrative until abstractness is defined per language (see Open gaps). |
> | Dependency matrix | **built** as the Matrix lens over L0's bands, ordered by group so upward sits below the diagonal. |
> | Cycles | **built** as a lens (only bands inside a cycle; members tinted), and still as `diagrams/cycles`. |
> | Hidden coupling | **built** — `GET …/diagrams/codemap/cochange` mines `commit_files` (5+ together, sweeps of 20+ modules left out), drops pairs with a static edge either way, ranks by how often the rarer changes with the other; drawn as dashed bands. |
> | Complexity | **built** from local qlty (user, 2026-10-27: rely on qlty, not a re-derivation): `measure_complexity` runs `qlty metrics --functions` at HEAD in a temporary worktree with the pinned ruler, only when HEAD moved; `sensei.file_complexity` per file; blocks tinted by share over cyclomatic 10, unmeasured blocks say so. SQL is not measured by qlty. |
> | File roles | **built at read time** — `analysis::codemap::file_role` (code · test · docs · config · generated, from the path); not stored. |

## In scope / out of scope

- In: the Code map canvas and its three levels (L0 modules · L1 files by role · L2
  symbols); grouping (layers · folders · communities); the seven lenses (Structure ·
  Wheel · Zones · Dependency matrix · Cycles · Hidden coupling · Complexity); the band,
  hub-lane and referenced-library rules; the inspector; the per-lens header; fit and
  zoom; the list form under 640px; the graph reads behind it (`nodes`, `communities`,
  `communities/info`, `callers`, `callees`, `call-flow`, `{repoId}/tree`, the
  `diagrams/*` endpoints).
- Out: the Schema / Database · ER view (its own document,
  [spec/indexer/20-schema-entities](../../../../spec/indexer/20-schema-entities.md),
  listed last in the same Diagrams list); an Ownership lens (dropped, D6 — it shows
  per-person data; a per-repository maintainer view may carry it later, never a
  viewer); a global force layout over every edge; a *declared* layering
  (`observatory-diagrams-layers.md`); the numbers and trends of the architecture
  metrics (M8 keeps them; this map explains them); building the role tags, Fix 2 and
  co-change mining (M4).

## One canvas, three levels

| Level | Shows | Budget | Edges |
|---|---|---|---|
| **L0 · modules** | modules as blocks inside group bands, each with a role bar | ≤ 60 blocks (G-1); smaller modules in a band fold into "+N more" | one **band** per module pair; width = calls + imports behind it |
| **L1 · files** | one module's files in columns by role: **code · tests · docs · config · generated** | ≤ 12 per column, smallest folded | stub lists *depends on* / *used by*, each with weight and direction |
| **L2 · symbols** | one file's symbols, with callers and callees one hop out | ≤ 7 a side | direct calls; upward or cyclic calls marked |

- **Navigation.** A single click on a module opens it (L1); a single click on a file
  opens it (L2). The breadcrumb `workspace › module › file` climbs back. Search jumps
  to a module and opens the levels on the way. A level tag (`L0 · modules`) names
  where you are.
- **Group by** (L0): **Layers** (default — the layer order UI → App → Domain → Infra),
  **Folders** (repository, then module folder), **Communities** (clusters from the call
  graph, `inference.communities`; they cut across folders and layers). The choice
  drives the L0 bands, the Wheel's arcs and the Matrix's diagonal blocks.
- **Show** filter: role chips hide a role from counts, role bars and L1 columns. Code,
  tests and docs are on by default; config and generated are off, so generated code
  (GraphQL clients, protobuf) cannot dominate sizes. Docs never enter the call graph;
  they sit beside the code they describe.

## Lenses

Every lens is the same canvas. Each carries a header with three lines — *what it
shows · use it to · how to read it* — collapsible behind "What it's for", and the
open/closed choice is remembered.

| Lens | Mark | Hint | On the canvas |
|---|---|---|---|
| **Structure** | 構 | the lay of the land | neutral blocks and weighted bands; upward bands are always red. **Colour by direction** tints every band: down · same layer · upward · skips a layer. Replaces Atlas's world view, Structure and Layers. |
| **Wheel** | 輪 | which groups lean on which | radial edge bundling: modules on a ring in their group's arc; bands bend through group anchors so dependencies between two groups bundle into one strand. Hubs and referenced libraries are left off. |
| **Zones** | 域 | hard to change, or unused | modules placed by instability (Ce ÷ (Ca + Ce)) against abstractness, with the main sequence, the zone of pain (bottom-left) and the zone of uselessness (top-right); the furthest modules listed. |
| **Dependency matrix** | 格 | dense areas, cycles | module × module grid, rows depend on columns; the diagonal blocks are the chosen groups. Cells are coloured by **dependency direction**: upward is red (a violation), a layer skip or a cycle is amber, same-layer is neutral. Position in the triangle never decides colour. |
| **Cycles** | 環 | break before refactoring | only the bands that form dependency cycles; the modules involved are tinted; the inspector lists the partners. |
| **Hidden coupling** | 縁 | change together, no edge | dashed bands between modules that change in the same commits but have no static dependency. |
| **Complexity** | 繁 | where to refactor and test | blocks tinted by the share of functions over cyclomatic complexity 10: amber over 10%, red over 20%. |

Structure, Cycles, Hidden coupling and Complexity work at every level and keep the
level and selection when you switch between them. Wheel, Zones and the Matrix are L0
views: switching to one returns to L0 and keeps the selection.

The Code map follows the app's **theme mode** (light / dark / system) and its **kanji ⇄
icons** preference (`ui.glyphs`; gap A-1, resolved). In icon mode the lens marks and the
search glyph use their icons, as everywhere else. The mockups read `sensei.theme` and
`sensei.iconStyle`; the implementation must read the app's theme mode and `ui.glyphs`
instead.

## Keeping it readable

- **Bands, not lines.** All edges between two modules aggregate into one band,
  labelled with the count.
- **Band filters:** *all* or *violations & cycles*; minimum weight ≥ 1 / ≥ 10 / ≥ 25.
- **Hub lane.** A module whose inbound edges exceed 5% of all edges (logger, errors,
  config) moves to a side lane. Its edges are counted on the block, not drawn.
- **Referenced libraries** are one block ("N packages · not members"). A workspace
  references libraries; it does not own them, and they are never gated.
- **Selection dims everything else** to the selected module's bands.
- **Budget readout** on every view: `drawing 37 of 37 modules · 58 bands for 18,940 edges`.
- **Fit to width** with a 60% floor; below that the canvas scrolls. A **Fit / 100%**
  toggle keeps true size.
- **Under 1200px** the lens list becomes a strip and the inspector drops below the
  canvas.
- **Under 640px every lens becomes a list.** Structure, Wheel, Cycles, Hidden coupling
  and Complexity show modules grouped by the chosen grouping, with a role bar and
  badges (upward, cycle, % over cx 10, co-change partner). Zones ranks modules by
  distance from the main sequence. The Matrix becomes group → group dependencies,
  heaviest first. Tapping a module opens its files as one column; a file shows
  callers, itself and callees as lists. Rows are at least 44px tall.

## Inspector

Selection-aware. A **module**: symbols, files, tests ÷ code, share over cx 10, weight
out · in, violations, what to look at. A **file**, a **hub** (with the 5% rule), a
**folded group**, the **referenced libraries**, or with nothing selected the
**workspace summary**: upward · skip · cycles · same layer · down counts (the old Atlas
layers histogram).

## Branches

The map reads the latest code graph of the synced branch in view (M4 §Branch scope):
the **development** environment's branch by default, for sprint work; **production**'s
when looking at a release (D18).

## Surfaces

- **Sensei, workspace window (M16)** — the Diagrams list: Structure · Wheel · Zones ·
  Dependency matrix · Cycles · Hidden coupling · Complexity · Database · ER. The first
  seven open this one embedded map and switch lens without reloading. The Code map
  supplies the only header for its lenses.
- **Dōjō, Workspace › Code health (T3)** — the same map, per repository, with the same
  lens list plus Database · ER. Each gated reading above it (layer conformance, cycles,
  zone of pain, complexity) opens the lens that explains it.
- **Metrics (M8)** — the architecture-metric charts (layers histogram, zones, cycles)
  link into the lens here instead of drawing their own diagram.
- **Routes today** — `/workspace/[id]/diagrams/{codemap,structure,layers,cycles,
  neighbourhood,schema}`; `codemap` is the default.
- **API** — `GET /api/graph/nodes` · `/communities` (POST) · `/communities/info` ·
  `/callers` · `/callees` · `/call-flow` · `/{repoId}/tree`;
  `/api/workspaces/{id}/diagrams/{codemap,codemap/module,codemap/file,structure,layering,
  zones,neighbourhood,schema}`.
- **Daemon** — `api/handlers/codebase.rs` (`graph_nodes`, `community_info`,
  `community_counts`, `fn_callers`, `fn_callees`, `graph_tree`, `call_flow`);
  `api/handlers/diagrams.rs`; `api/diagram_cache.rs`; `db/pg_store/{graph.rs,
  neighbourhood.rs}`; `indexer/{community.rs,structure.rs,impact.rs,
  quality/reachability.rs}`; `tasks/handlers/git_history.rs`.
- **App today** — `packages/codemap` (`CodeMapState`, `layoutLevel0`, lenses, `CodeMap`
  and its parts) through `diagrams/codemap-port.ts`; the remaining diagram states and
  `diagrams-nav.ts` under `(workspace)/workspace/[id]/diagrams/`.
- **Data** — `sensei.nodes` / `sensei.edges` → `sensei.graph_nodes` (view) →
  `inference.communities` → `sensei.structure_graph` + `structure_edges` (views) →
  `sensei.module_edges` → `sensei.graph_placement`; `sensei.component_zones()` (function);
  `sensei.commits` / `sensei.commit_files`.

## Data invariants

- **Lead.** One owner. Every file is owned by exactly one folder (repo / git root); a
  `folder`-kind node owns no code nodes; a symbol never renders twice.
- **Lead.** Communities are per-folder; the read must aggregate.
  `inference.communities` is one row per `(folder_id, community_id)`, so a
  per-workspace read aggregates across folders — never a single `resolve_folder_id`.
- **Lead.** Drop unplaced, never fabricate. An edge with `target_id = null` is not
  drawn, and the unplaced count is shown apart from the placed one.
- **Lead.** Budgets. L0 ≤ 60 blocks; L1 ≤ 12 per role column; L2 ≤ 7 a side. What
  does not fit is folded and counted, never dropped silently.
- **Lead.** One band per module pair, weighted by calls + imports.
- **Lead.** Derived layering; skip ≠ violation. Only an upward edge is a violation; a
  same-layer edge is not; a self-dependency is not a cycle. Colour follows direction,
  never position.
- **Lead.** Roles come from the graph. A file's role (code · test · docs · config ·
  generated) is the one M4 stores; the map never re-derives it from a path.
- **Lead.** Libraries are referenced, not members. They draw as one block and never
  enter module counts.
- **Lead.** A zone point with null abstractness or instability is omitted and counted
  (`coverage.unplaceable`), never drawn at an axis origin.

## Done gate

- L0 on a 2,000+ symbol workspace draws ≤ 60 blocks and one band per module pair.
- Switching group-by keeps the selection and re-bands without reloading.
- Turning off Tests shrinks every block's file count and removes the Tests column at L1.
- A hub never draws its edges at L0.
- Switching between Structure, Cycles, Hidden coupling and Complexity never resets the
  level or the selection; Wheel, Zones and the Matrix open at L0.
- In the Matrix every upward dependency is red and every same-layer cell is neutral.
- Under 640px every lens renders as a list with rows ≥ 44px.
- Diagram endpoints validate `level` / `kinds` → 400, never an empty payload.

```bash
curl -s "http://localhost:7744/api/graph/communities/info?repoId=$ID" | jq 'length'
curl -s "http://localhost:7744/api/projects/$ID/diagrams/structure?level=module" | jq '.nodes|length'
curl -s "http://localhost:7744/api/projects/$ID/diagrams/zones" | jq '.coverage'
```

## Wrong gate

- **L0 renders symbols or files.**
- **A dense hairball at any level; unplaced edges silently dropped.**
- **Tests or generated code counted as code size by default.**
- **Two diagrams of the same graph with different grouping** — a lens with its own
  grouping, or Metrics drawing its own layers / zones / cycles chart.
- **A matrix cell coloured by triangle position**, or a same-layer edge shown as a
  violation.
- **Referenced libraries drawn as workspace modules.**
- **An Ownership lens, or any per-person view, on a viewer surface** (D6).
- **A lens shown as built when it is not** — this README's build-status table is the
  record.
- **Empty graph while the DB holds communities** — the old G5a symptom.
- **Fabricated edges; mockup data shipped as data.**
- **An empty canvas on a DB error; a stale payload under an error banner.**

## Scenarios

- [`scenarios/code-map.feature`](scenarios/code-map.feature) — the graph reads,
  communities, resolution and impact (pinned), and the levels, grouping, roles and
  lenses (`# gap:` + `NO TEST`).
- [`scenarios/diagrams.feature`](scenarios/diagrams.feature) — the diagram endpoints
  behind the lenses (structure, layering, zones, neighbourhood, schema, world) and the
  diagram cache (pinned).

## Open gaps

- [G-1](../../../../roadmap/06-gaps.md) — no L0/L1/L2 levels of detail (P1). The
  prerequisite for reading the architecture metrics at any scale.
- **Fix 2** (M4) — community-edge aggregation; blocks the Wheel and Communities grouping.
- **File roles** (M4) — config and generated are not tagged; blocks the role bar and
  L1 columns beyond code / tests / docs.
- **Co-change mining** (M4) — blocks Hidden coupling.
- **Per-function complexity is not stored** — blocks the Complexity lens.
- **Open:** whether Zones' abstractness reads interfaces / traits per language. It
  needs the architecture-metrics family (#156); until then the lens is marked
  illustrative.
- [A-1](../../../../roadmap/06-gaps.md) — the kanji ⇄ icon toggle: **resolved** (`ui.glyphs`);
  the Code map must read it, not the mockups' `sensei.iconStyle`.
- [F-4](../../../../reorg.md) (M4) — thin graph resolution undercuts every lens.
- [D-7](../../../../reorg.md) (T3 · M8) — architecture-metric targets blocked
  (#156 ← #146/#151/#152).

## Related

- [16-workspace-window](../16-workspace-window/README.md) — the Diagrams list that
  embeds this map.
- [04-code-graph](../../01-foundation/04-code-graph/README.md) — the graph, file roles,
  Fix 2, co-change mining and branch scope.
- [08-metrics-and-health](../../02-insight/08-metrics-and-health/README.md) — the
  numbers and trends this map explains.
- [spec/screen/project-atlas](../../../../spec/screen/project-atlas.md) ·
  [observatory-diagrams-{world,layers,structure,neighbourhood,schema}](../../../../spec/screen/) —
  legacy depth, folds here.
- [design/code-graph-schema.json](../../../../design/code-graph-schema.json) — node/edge
  kinds and the scoping rule.
- [reorg.md §9.7](../../../../reorg.md) — the three levels of detail.
- Mockup: `docs/mockups/Sensei/Sensei Code Map.dc.html`.
- Crates: `crates/senseid/src/{api/handlers/codebase.rs, api/handlers/diagrams.rs,
  api/diagram_cache.rs, db/pg_store/graph.rs, db/pg_store/neighbourhood.rs,
  indexer/community.rs, indexer/structure.rs, indexer/impact.rs,
  indexer/quality/reachability.rs, tasks/handlers/git_history.rs}`.
