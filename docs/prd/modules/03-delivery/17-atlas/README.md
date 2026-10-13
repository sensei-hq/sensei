# Atlas · M17

**Plane:** delivery · **Status:** [roadmap/03-delivery.md](../../../../roadmap/03-delivery.md) ·
**Serves:** team developer ([persona 2](../../../03-personas.md#2--team-developer--the-inheritor)),
manager / delivery lead ([persona 6](../../../03-personas.md#6--manager--delivery-lead--the-roll-up))

## Purpose

Atlas is the spatial view of the code graph: move between granularities
(repo → package → module → symbol) and see how the pieces relate, with a docs /
drift overlay. Its job is **legibility at every scale** — a birds-eye that a manager
can read without opening a file, and a detail view a developer can click through to
the code. The user feels this as *I can see the shape of this codebase at a glance —
and then zoom into the piece I care about.*

> **Status honesty.** Atlas ships today as a **two-level** canvas (Communities →
> Symbols) plus a full Diagrams sub-tree (structure · layers · cycles · neighbourhood
> · schema · world) — all built and tested. The **three readable levels L0/L1/L2**
> the reorg §9.7 deliverable asks for are **not built**: this is gap
> [G-1](../../../../roadmap/06-gaps.md). The screen spec
> (`spec/screen/project-atlas.md`) calls the project Atlas "greenfield", which is
> stale — the app ships `(project)/project/[id]/atlas/` with canvas, inspector,
> legend and unit specs. The old G5a bug (`get_communities` resolving a single folder)
> is **fixed** — `mcp.rs::get_communities` now uses `resolve_scope_ids` +
> `list_communities_scoped`.

## In scope / out of scope

- In: the Atlas canvas at `/project/{id}/atlas`; communities (per-folder aggregation);
  the Diagrams sub-tree (`structure`, `layers`, `cycles`, `neighbourhood`, `schema`,
  `world`); graph reads (`nodes`, `communities`, `communities/info`, `callers`,
  `callees`, `call-flow`, `{repoId}/tree`); the docs/drift overlay; and the **three
  levels of detail L0/L1/L2** (the §9.7 deliverable, gap G-1).
- Out: a global force layout over every edge (every view must be scoped); a
  *declared* layering (a different question/source, `observatory-diagrams-layers.md`);
  drill-down from Structure into a module (rokkit#165); indexing documentation as
  declarations into World (the missing docs ring, #246).

## Surfaces

- **Routes** — `/project/[id]/atlas`; `/project/[id]/diagrams/{structure,layers,cycles,
  neighbourhood,schema,world}`.
- **API** — `GET /api/graph/nodes` · `/communities` (POST) · `/communities/info` ·
  `/callers` · `/callees` · `/call-flow` · `/{repoId}/tree`; `/api/projects/{id}/diagrams/
  {structure,layering,zones,world,neighbourhood,schema}`.
- **Daemon** — `api/handlers/codebase.rs` (`graph_nodes`, `community_info`,
  `community_counts`, `fn_callers`, `fn_callees`, `graph_tree`, `call_flow`);
  `api/handlers/diagrams.rs`; `api/diagram_cache.rs`; `db/pg_store/{graph.rs,
  neighbourhood.rs}`; `indexer/{community.rs,structure.rs,impact.rs,quality/reachability.rs}`.
- **App** — `atlas-graph.svelte.ts` (`buildAtlasPage`, `buildCommunities`,
  `buildSymbolGraph`, `initialLevel`, `SYMBOL_CAP=200`, `COLLAPSE_THRESHOLD=500`),
  `atlas-view.svelte.ts`, `AtlasCanvas` / `AtlasInspector` / `AtlasLegend`; diagram
  states under `(project)/project/[id]/diagrams/`.
- **Data** — `sensei.nodes` / `sensei.edges`, `sensei.graph_nodes` (view),
  `inference.communities` (one row per `(folder_id, community_id)`),
  `sensei.structure_graph` + `structure_edges` (views), `sensei.module_edges`,
  `sensei.graph_placement`, `sensei.schema_entities/_columns/_indexes/_refs`.

## Data invariants

- **Lead.** One owner. Every file is owned by exactly one folder (repo / git root);
  a `folder`-kind node owns no code nodes; a symbol never renders twice.
- **Lead.** Communities are per-folder; the endpoint must aggregate.
  `inference.communities` is one row per `(folder_id, community_id)`
  (`unique(folder_id, community_id)`), so a per-project read aggregates across folders
  — never a single `resolve_folder_id`.
- **Lead.** Drop unplaced, never fabricate. An edge with `target_id = null` is dropped,
  not drawn.
- **Lead.** Node budgets. Atlas caps at 200 symbols (`SYMBOL_CAP`); L0 stays ≤ 60
  blocks (reorg §9.7).
- **Lead.** Derived layering; skip ≠ violation. The view filters `conformance ===
  'up'`; a self-dependency is not a cycle.
- **Lead.** World is cross-project; path id scopes nothing. `groupBy=project|repository|
  kind`; `groupBy=docs` is a 400.

## Signals shown

| Element | Meaning | Worked example |
|---|---|---|
| Granularity control | coarseness of nodes | `Repos · Packages · Modules · Symbols` |
| Community cluster | radius = `node_count` | ≤ 158 on the sensei root |
| Symbol node | radius = `nodes.degree` | top-degree symbols only, cap 200 |
| Depends on · N | out-edges (`get_callees`) | inspector chips |
| Used by · N | in-edges (`get_callers`) | inspector chips |
| Docs layer | doc drift/broken overlay | `¶` dashed square, warning/danger |
| Coverage line | drawn vs unplaced | `N edges drawn · M unplaced` |
| Layers histogram | conformance counts | `91 down / 63 level / 25 up / 15 skip` |
| Cycles card | weakest link named | `Cut senseid/adapters → senseid/tasks ×2` |
| World circle | declarations held, log scale | one ring per project/repo |

## Done gate

- Atlas opens on the `[id]`-resolved real repo graph, with no authored `x`/`y`.
- Communities render (an aggregated read; detect-before-read).
- The inspector's depends-on / used-by match the incident edges; a chip click
  re-selects.
- Drill pushes and restores the breadcrumb; radius encodes a real magnitude.
- The docs toggle overlays doc nodes; off removes the drift row.
- Diagram endpoints validate `level`/`kinds` → 400, never an empty payload.
- **L0/L1/L2 (G-1):** each level is complete at its own scale — L0 ≤ 60 labelled
  blocks with weighted band edges, no symbols; L1 a module's files/types + stub edges;
  L2 a symbol and its neighbourhood.

```bash
curl -s "http://localhost:7744/api/graph/communities/info?repoId=$ID" | jq 'length'
curl -s "http://localhost:7744/api/projects/$ID/diagrams/structure?level=module" | jq '.nodes|length'
```

## Wrong gate

- **Empty graph while the DB holds 158 communities** — the old G5a symptom.
- **A symbol rendered twice, or a node under a `folder`-kind owner** — one-owner
  violated.
- **Fabricated edges; the mockup lumen-cloud `ATLAS` shipped as data.**
- **Inspector counts disagree with the canvas; radius encodes nothing.**
- **A dense hairball at file level; unplaced edges silently dropped.**
- **A docs ring with nothing in it** (World, #246).
- **An empty canvas on a DB error; a stale payload under an error banner.**
- **Claiming three readable levels when only Communities/Symbols ship** — must read as
  two levels until G-1 lands.

## Scenarios

- [`scenarios/atlas.feature`](scenarios/atlas.feature) — communities, the graph reads,
  resolution and impact (pinned + `# gap: G-1`).
- [`scenarios/diagrams.feature`](scenarios/diagrams.feature) — structure, layers,
  zones, neighbourhood, schema, world and the diagram cache (pinned).

## Open gaps

- [G-1](../../../../roadmap/06-gaps.md) — graph/layered view unreadable at granular
  scale, no L0/L1/L2 (P1). This is the §9.7 deliverable and a prerequisite for reading
  the architecture metrics at any scale.
- [A-1](../../../../roadmap/06-gaps.md) — the kanji ⇄ solar icon toggle the mockups and
  rokkit support is not wired (M2 · M17, P3). The visual style is a display mode, not a
  schema (reorg §9.7 rule 4).
- [F-4](../../../../reorg.md) (M4) — graph 40.3% resolved, no published target — thin
  resolution undercuts every graph view.
- [M-4](../../../../reorg.md) (M8) — no registered metric reads the code graph.
- [D-7](../../../../reorg.md) (T3 · M8) — architecture-metric targets blocked
  (#156 ← #146/#151/#152).

## Related

- [spec/screen/project-atlas](../../../../spec/screen/project-atlas.md) ·
  [observatory-diagrams-{world,layers,structure,neighbourhood,schema}](../../../../spec/screen/) —
  Atlas + diagrams (legacy depth, folds here).
- [design/code-graph-schema.json](../../../../design/code-graph-schema.json) — node/edge
  kinds and the scoping rule.
- [reorg.md §9.7](../../../../reorg.md) — the three levels of detail.
- [prd/02-objectives.md §Dōjō](../../../02-objectives.md) · [prd/03-personas.md §6](../../../03-personas.md#6--manager--delivery-lead--the-roll-up).
- Crates: `crates/senseid/src/{api/handlers/codebase.rs, api/handlers/diagrams.rs,
  api/diagram_cache.rs, db/pg_store/graph.rs, db/pg_store/neighbourhood.rs,
  indexer/community.rs, indexer/structure.rs, indexer/impact.rs,
  indexer/quality/reachability.rs}`.
