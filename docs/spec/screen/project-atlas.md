# 図 · Project window · Atlas (code graph)

**Segment:** 04 · The project window
**Route:** `/project/[id]/atlas` (proposed — a 10th `ProjectSidebar` section; the mockup's `ProjectAtlasWindow` renders `<ProjectSidebar … active="atlas"/>`)
**Source mockup:** [`lib/project/project-atlas.jsx`](../../mockups/Sensei/lib/project/project-atlas.jsx) → `ProjectAtlasWindow` (screen: `ProjectAtlas`; canvas: `AtlasGraph`; inspector: `AtlasInspector`; legend: `AtlasLegend`)
**Data:** the node/edge graph (`GET /api/graph/nodes`, `GET /api/graph/call-flow`) · communities (`GET /api/graph/communities/info` = `get_communities`, **blocked by G5a on the MCP path**) · `GET /api/graph/callers` / `/api/graph/callees` (`get_callers`/`get_callees`)
**App file:** _project-window Atlas is greenfield_ — an **observatory-scoped** graph view already exists at `app/src/routes/(observatory)/atlas/+page.svelte` (2 levels: Communities/Symbols, keyed by repo *name*, default `sensei`). The project-window Atlas (repo→package→module→symbol drill + docs layer, scoped to `[id]`) is not built.
**Daemon files:** `crates/senseid/src/api/handlers/query.rs` (`query_communities` already aggregates via `resolve_scope_ids`), `crates/senseid/src/api/handlers/mcp.rs` (`get_communities` still uses single-folder `resolve_folder_id` — **the G5a bug**), `crates/senseid/src/api/handlers/codebase.rs` (`detect_communities` / `list_communities`)
**Status:** backend graph **EXISTS and is populated** (post-#101 clean, 157k nodes, 158 communities on the sensei root); `get_communities` under-scopes on the MCP dispatch path (G5a); the project-window Atlas view is greenfield (observatory Atlas is a partial precedent, not the same screen)

## Purpose

One project spans several repos, each with crates (Rust) and
packages (JS), which contain modules, which contain classes and
functions — plus the docs that describe all of it. The Atlas is the
**code-graph visualization** that lets the user move between those
granularities and see how the pieces relate. It is the spatial,
"show me the shape of this codebase" counterpart to the list-shaped
Overview and Traceability panes.

The screen is a graph canvas + an inspector, driven by four moves:

1. **Pick a granularity.** A segmented control — `Repos · Packages ·
   Modules · Symbols` — sets how coarse the nodes are. Package level
   shows the whole project; deeper levels scope to one container.
2. **Read the clusters.** At the coarse levels the node radius encodes
   size (community `node_count` / node `degree`); colour encodes kind.
   This is "what are the major pieces, and which is biggest?"
3. **Select a node.** Selecting dims everything but the node and its
   graph neighbours, and fills the inspector with *depends-on* (out-
   edges), *used-by* (in-edges = `get_callers`), linked docs, and a
   drill/reveal action. This is the `get_callers`/`get_callees`
   answer, rendered.
4. **Drill in.** A second click on a container node (or the inspector's
   *Open →* button) descends a level and pushes a breadcrumb; the
   breadcrumb walks back up.

A **Docs layer** toggle (default on) overlays doc nodes (small dashed
squares) with dashed reference edges to the code they describe, tinted
`warning` for drift and `danger` for a broken reference — the same
drift signal the Traceability pane owns, seen in graph space.

Kanji is 図 — *diagram / map*.

## Data invariants

_(the graph is per-project; nodes are owned one-per-file by the repo root [one-owner invariant]; communities are per-folder clusters; **G5a must be fixed for communities to render on the MCP path**)_

### The graph is real, and per-project

- **`sensei.nodes`** (16 kinds — `file, module, package, class,
  interface, function, method, property, field, parameter, type,
  const, enum, enum_variant, section, rationale`, plus
  `struct/component/hook/doc/extension`) and **`sensei.edges`** (11
  kinds — `calls, implements, extends, imports, depends_on, traces_to,
  references, covers, rationale_for, duplicates, similar_to`) are
  **built and populated** — this is not a Relay-style empty backend.
  `edge_confidence ∈ extracted | inferred | ambiguous`; the graph is
  clean post-#101.
- **`nodes.degree`** (precomputed in+out edge count) drives the
  symbol-level radius; **`nodes.community_id`** (Leiden cluster int)
  ties a symbol to its community. Both are batch-computed and null
  until computed — the view must degrade, not fabricate, on nulls.
- The mockup's authored `ATLAS` object (lumen-cloud, 780×520 hand-laid
  positions) is **layout scaffolding only**. Real nodes carry no
  coordinates — the view computes a force/hierarchical layout from the
  wire graph. Do not ship the mockup's fixed `x/y` as data.

### One-owner invariant (why duplicate nodes are a bug, not a level)

- Every file is owned by **exactly one folder — the repo/git-root**.
  Structural subfolders (`kind='folder'`) are members with a
  role/kind and own **no** code nodes (`docs/architecture/data.md`).
  A node appearing under a `folder`-kind owner, or two nodes for the
  same symbol, is the pre-#101 double-index residue — the reconcile
  (`dedup_structural_folder_nodes`) exists to prevent it. The Atlas
  must never render a symbol twice.

### Communities are per-folder; the endpoint must aggregate

- **`inference.communities`** stores one row per `(folder_id,
  community_id)` with `label`, `node_count`, `god_node_ids` (top-5
  highest-degree), `description`. There are **158 real communities on
  the sensei root** right now.
- **G5a — the live scoping bug.** A project can span several folders.
  The HTTP query path (`query.rs::query_communities`) already loops
  `resolve_scope_ids(...)` and appends `list_communities(fid)` per
  folder — correct. **But the MCP dispatch (`mcp.rs::get_communities`)
  still calls `resolve_folder_id`, which returns
  `resolve_scope_ids(...).into_iter().next()` — the single lowest-UUID
  leaf folder** — so `get_communities` reports the clusters of one leaf
  and silently drops the rest (often returning empty for the project as
  a whole). The wire type is `CommunityInfo { id, label, node_count }`.
  **The project-window Atlas must read the aggregated path (or the fix
  must land on the MCP path) — communities will not render otherwise.**
- The observatory Atlas's `load()` calls `detectCommunities(repo)`
  (POST refresh) *then* `getCommunities(repo)` so the read reflects the
  current index rather than a stale Leiden run; the project Atlas
  should follow the same refresh-then-read order.

### Edge shapes and the "out-of-scope target" rule

- `GET /api/graph/nodes` → `{ nodes: GraphSymbolNode[], edges:
  GraphCallEdge[] }`. A `GraphCallEdge` with `target_id = null`
  resolves to an out-of-scope / stdlib symbol (only `target_name` is
  known) — **those are dropped from the rendered graph**, never drawn
  as a phantom node. `get_callers`/`get_callees` are keyed by symbol
  *name* within the repo scope, matching the inspector's used-by /
  depends-on lists.
- **Cross-language / cross-repo edges** are drawn dashed (the mockup's
  `boundary: true` edges, e.g. a JS package → a Rust service). Legend:
  *cross-language*.

### Scope resolution (repo name, not project UUID)

- Every graph endpoint is keyed by **repo name** (`get_repo_by_name`),
  not project UUID. The route is `/project/[id]/…` but `load()` must
  resolve `[id]` → repo name(s) before calling the graph reads (the
  observatory Atlas hardcodes `DEFAULT_REPO = 'sensei'`; the project
  window has the id and must resolve it). The solution roll-up
  (`getProjectGraph(projectUuid)`) is keyed by UUID and contributes
  repo/node/edge **counts**, not the primary graph — it is empty until
  repo↔project membership is populated.

## Signals shown

_Real mockup content: the graph canvas, community clusters, node kinds, edges, the inspector, the docs layer._

### Granularity control (segmented)

| Level | Node = | Radius encodes | Scope |
|---|---|---|---|
| Repos | repo / git-root | crate + package count | whole project |
| Packages | crate (Rust) / package (JS) | module count | whole project; a repo id just *spotlights* one cluster (focus), context stays |
| Modules | `module` node | child count | one container (drilled-into crate) |
| Symbols | `function` / `class` / `method` / `type` | `nodes.degree` | one container (drilled-into module) |

The shipped observatory Atlas collapses this to two levels
(`Communities`, `Symbols`); the project-window mockup is the fuller
four-level drill. `initialLevel(totalSymbols, communities.length)`
picks the opening granularity from payload size.

### Node kind → colour (token-only, from `atlasFill`)

| Kind | Fill token | Notes |
|---|---|---|
| repo / fn | `accent` | |
| crate / class(type) | `ink` | text `paper` on the fill |
| package (JS) | `success` | |
| module | `paper-mute` (`--paper-3`) with `ink` ring | the only ringed / dark-text node |
| doc | `warning` (drift) / `danger` (broken) | dashed square, `¶` glyph, `paper` fill |

Selected node gets an `accent` halo ring (`r+6`). Non-neighbours dim
to `0.22`; on a package-level focus, off-focus repos dim to `0.34`.

### Inspector (right rail, `AtlasInspector`)

Empty state ("This view") — derived from the current graph:

| Element | Value |
|---|---|
| Nodes | `graph.nodes.length` (rendered nodes at this level) |
| Relations | `graph.edges.length` |
| Docs linked | `graph.docs.length` (docs layer) |
| Doc drift | count of docs with `status != 'ok'` — shown `warning`, only when docs layer on and drift > 0 |

Selected node:

| Element | Value | Source |
|---|---|---|
| Identity | kanji chip + `KIND_LABEL[kind]` eyebrow + mono name + `sub` | node kind + name |
| Depends on · N | out-edge targets as clickable mono chips | `edges where source = node` (`get_callees`) |
| Used by · N | in-edge sources as clickable chips | `edges where target = node` (`get_callers`) |
| Linked docs | doc label + `in sync` / `drift` / `broken` status chip | docs layer edges attached to node |
| Doc banner | "Broken reference — points at a symbol that no longer exists." / "Drifted — code changed after this doc was last reviewed." | doc node `status` |
| Action | *Open {label} →* (`zs-btn-primary`) when `node.children`; else *Reveal in editor* (`zs-btn-secondary`); none for docs | drillability |

Community-level inspector (observatory precedent) shows `label`,
`kind`, `path`, `node_count`, and `sharePct = node_count /
communityNodeTotal`.

### Header + chrome

- Kanji 図 (`accent`), eyebrow "Project · Atlas", title "Code graph".
- **Docs layer** toggle (top-right), default **on**; `warning`-tinted
  when active.
- Breadcrumb (`Project name › crate › module …`) — mono, last crumb
  bold; clicking a crumb pops back to that level and clears selection.
- Legend (bottom-left, over canvas): repo/fn · crate/type · JS package
  · module (ringed) · cross-language (dashed) · doc(drift) (when docs
  on).

## Done gate

- Opens on the **sensei project's real indexed graph** — nodes/edges
  from `GET /api/graph/nodes` on `[id]`-resolved repo(s), not the
  mockup's authored lumen-cloud `ATLAS`. No hand-laid `x/y` shipped as
  data.
- **Communities render.** After the aggregated read (or the G5a fix on
  the MCP path), the sensei project shows its real community clusters
  (up to 158 on the sensei root), each sized by `node_count`, labelled
  from `communities.label`. `detectCommunities` runs before the read so
  the clustering is current.
- Selecting a symbol fills the inspector with real **depends-on**
  (`get_callees`) and **used-by** (`get_callers`) lists; the counts
  match the rendered incident edges; clicking a chip re-selects that
  node.
- Container nodes drill a level and push a breadcrumb; the breadcrumb
  walks back up and clears selection.
- Node radius reflects real magnitude (`node_count` at community level,
  `nodes.degree` at symbol level) — bigger cluster = bigger circle.
- Docs-layer toggle overlays doc nodes with dashed reference edges;
  drift is `warning`, broken is `danger`; toggling off removes them and
  the "Doc drift" inspector row.
- Edges with `target_id = null` (out-of-scope / stdlib) are **dropped**,
  not drawn; cross-language edges render dashed.
- Dark mode: every kind colour + the dimmed (`0.22`/`0.34`) and
  selected-halo states stay legible; module nodes keep their ring and
  dark text.

Optional check:
```
# 1) communities exist on the sensei root (the data G5a hides)
psql -A -t -d sensei -c \
  "select count(*) from inference.communities c
     join sensei.folders f on f.id = c.folder_id
    where f.name = 'sensei'"
# expected: ~158

# 2) the aggregated HTTP path returns them; the single-folder path may not
curl -s 'http://localhost:7744/api/graph/communities/info?repoId=sensei' \
  | jq 'length'
# expected: > 0  (0 here == still reading one leaf folder → G5a)

# 3) node/edge graph is populated
curl -s 'http://localhost:7744/api/graph/nodes?repoId=sensei' \
  | jq '{nodes: (.nodes|length), edges: (.edges|length)}'
# expected: both large; no node appears twice for one symbol
```

## Wrong gate

- **Empty graph / no clusters while the DB has 158 communities.**
  The classic **G5a** symptom — the view read the single lowest-UUID
  leaf folder (`resolve_folder_id`) instead of aggregating all scope
  folders (`resolve_scope_ids`). Fix the read path; do not paper over
  it with the mockup's authored graph.
- **A symbol rendered twice, or code nodes hanging off a `folder`-kind
  owner.** The **one-owner invariant** was violated — pre-#101 double-
  index residue that `dedup_structural_folder_nodes` should have
  cleaned. One repo = one owner; one symbol = one node.
- **Fabricated edges.** Drawing an edge whose `target_id` is null (an
  out-of-scope/stdlib call) as a real in-graph relation, or inventing
  cross-repo edges the graph doesn't contain. Never worse than the
  wire — drop unknowns, don't guess.
- **Mockup layout shipped as data.** The lumen-cloud `ATLAS` positions,
  labels, and doc statuses are prototype scaffolding — rendering them
  instead of the daemon's real graph is a fake screen.
- **Inspector counts disagree with the canvas.** Depends-on / used-by
  don't match the incident edges (e.g. reading global callers instead
  of scope-filtered `get_callers`/`get_callees`).
- **Radius encodes nothing** — every node the same size, so `node_count`
  / `degree` isn't driving magnitude.
- **Docs layer shows drift with no real signal** — doc statuses
  fabricated rather than sourced from the traceability/drift signal
  (and note **G3**: the current drift signal over-fires 4,420/4,425
  `broken`; the Atlas should reflect that caveat, not amplify it).
- **Solution roll-up mistaken for the graph.** `getProjectGraph(uuid)`
  (empty until repo↔project membership lands) rendered as the primary
  canvas instead of the per-repo node/edge graph.

## Related

- [[architecture/data]] — `nodes` (16 kinds) / `edges` (11 kinds) / `communities`; the one-owner invariant
- [[architecture/mcp]] — `get_communities`, `get_callers`, `get_callees`; the `get_communities` empty-despite-158 note
- [[plan/README]] — **G5a** (community single-folder scoping); G3 (doc-drift over-fires); G9 (`DetectCommunities` not in the watcher barrier chain → clusters stale between full scans)
- [[pipeline/semantic-search]] — the structural mode walks `edges` from a symbol / community; same graph substrate
- [[screen/project-traceability]] — the list-shaped view of the same doc-drift signal the docs layer overlays
- [[screen/project-overview]] — the project-window shell + `ProjectSidebar` this pane joins as a new section
