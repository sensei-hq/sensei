## Graph Indexing Regression — Atlas Lost Its Hierarchy

_Atlas degraded from a structured code map (docs vs code, packages, modules, classes, methods, sub-projects) to "scattered circles of different sizes" — because the pipeline stopped producing the grouping data Atlas needs, and the one grouping it does produce (communities) covers 1.1% of the graph while being drowned by 1.7M duplicate `covers` edges._

**Context.** The user reports Atlas "used to render project structure … now shows scattered circles of different sizes with no hierarchy." This section pins the pipeline stage and the timing. Three independent regressions stack: (1) community write-back reaches only 1.1% of nodes, (2) a `covers` edge duplication bug inflated the edge set 900×, and (3) the two hierarchy levels that would carry "packages / sub-projects / docs-vs-code" — the `package` node kind and the `subtree`/`workspace_member` folder kinds — are defined in the enums but produced **zero** rows.

**What the data shows.**

- **Community coverage is 1.14% of the graph.** Only 5,443 of 476,988 nodes carry a `community_id`; every one is `> 0` (no zero-sentinels). Atlas clusters by community, so 98.9% of the graph has no cluster to belong to.
  ```sql
  SELECT count(*) total, count(community_id) populated,
         round(100.0*count(community_id)/count(*),2) pct
  FROM sensei.nodes;                       -- 476988 | 5443 | 1.14
  ```
- **The coverage cliff is sharp and cohort-shaped.** The June-15 bulk index (451,181 nodes = 94.6% of the graph) has **0** communities; only the most-recently re-indexed cohort (Aug-03, 9,305 nodes) is 56% covered. Assigning a community bumps `modified_at = now()` (see `update_node_community`, pg_store.rs:1702), so any node that *ever* got a community migrates into a recent week — which is exactly why the old bulk shows a clean zero.

  | week (modified_at) | nodes | with community | % |
  |---|---:|---:|---:|
  | 2026-06-15 | 451,181 | 0 | 0.0 |
  | 2026-06-29 | 2,436 | 138 | 5.7 |
  | 2026-07-06 | 426 | 0 | 0.0 |
  | 2026-07-13 | 3,209 | 12 | 0.4 |
  | 2026-07-20 | 3,710 | 11 | 0.3 |
  | 2026-07-27 | 6,721 | 71 | 1.1 |
  | 2026-08-03 | 9,305 | 5,211 | 56.0 |

  ```sql
  SELECT date_trunc('week',modified_at)::date wk, count(*),
         count(community_id), round(100.0*count(community_id)/count(*),1)
  FROM sensei.nodes GROUP BY 1 ORDER BY 1;
  ```
- **The community table claims 8× more coverage than it delivers.** `inference.communities` holds 1,814 rows across 38 folders whose `node_count` sums to **43,290**, but only **5,443** nodes actually point back. The table accumulates stale generations: label-propagation emits non-deterministic integer labels each run, `upsert_community` keys on `(folder_id, community_id)`, and re-index deletes+recreates member nodes (new UUIDs, null community) while the community rows survive the cascade.
  ```sql
  SELECT count(*) rows, sum(node_count) claimed,
         count(*) FILTER (WHERE description IS NOT NULL) with_desc,
         count(*) FILTER (WHERE array_length(god_node_ids,1)>0) with_god
  FROM inference.communities;              -- 1814 | 43290 | 0 | 0
  ```
- **The best-developed repo has communities but zero cluster membership.** For the `senseid` folder, 95 community rows claim 1,528 nodes yet **0** nodes carry an id; `sensei` claims 31,534 across 1,133 rows but only 2,164 real. Atlas therefore draws ghost bubbles sized by phantom counts.

  | folder | community rows | claimed node_count | nodes actually tagged |
  |---|---:|---:|---:|
  | sensei | 1,133 | 31,534 | 2,164 |
  | torii | 249 | 2,280 | 338 |
  | senseid | 95 | 1,528 | **0** |
  | dbd | 57 | 1,634 | 735 |
  | rokkit | 53 | 2,129 | 400 |

  ```sql
  WITH f AS (SELECT folder_id,count(*) r,sum(node_count) c
             FROM inference.communities GROUP BY 1 ORDER BY 2 DESC LIMIT 5)
  SELECT fo.name,f.r,f.c,
    (SELECT count(*) FROM sensei.nodes n
      WHERE n.folder_id=f.folder_id AND n.community_id IS NOT NULL)
  FROM f JOIN sensei.folders fo ON fo.id=f.folder_id ORDER BY f.r DESC;
  ```
- **`covers` is 86% of all edges and is 99.9% duplicate noise.** 1,714,462 `covers` edges resolve to only **1,866 distinct** `(source,target)` pairs — a **918.8× duplication factor**. `insert_edge` (pg_store.rs:1711) is a bare `INSERT … RETURNING id` with **no `ON CONFLICT`**, and both `build_connections` and `reconcile_connections` (resolve.rs:130, 193) re-emit the entire doc×file cartesian product on every scan/reconcile.
  ```sql
  SELECT count(*), count(DISTINCT (source_id,target_id)),
         round(count(*)::numeric/count(DISTINCT (source_id,target_id)),1)
  FROM sensei.edges WHERE kind='covers';   -- 1714462 | 1866 | 918.8
  ```
- **One repo (rokkit) is 93% of the blowup: 632 distinct pairs inflated to 1,597,725 rows — 2,528×.** rokkit is a monorepo where doc/file stems collide (`index`, `mod`, `README`), so the stem-equality match in resolve.rs already over-connects, then duplicates on every reconcile.
  ```sql
  SELECT count(DISTINCT (source_id,target_id)), count(*)
  FROM sensei.edges e JOIN sensei.folders f ON f.id=e.folder_id
  WHERE e.kind='covers' AND f.name='rokkit';   -- 632 | 1597725
  ```
- **The `covers` explosion is recent — it did not exist in the June bulk.** In the 2026-06-15 week `covers` = 3,851 edges; it detonated to 437,279 (07-13), 478,450 (07-20), 536,398 (07-27), 257,698 (08-03). The onset (~2026-07-13) coincides with repeated reconcile passes on the newly-added repos.

  | edge kind | 2026-06-15 | 2026-07-13 | 2026-07-20 | 2026-07-27 | 2026-08-03 |
  |---|---:|---:|---:|---:|---:|
  | covers | 3,851 | 437,279 | 478,450 | 536,398 | 257,698 |
  | imports | 122,927 | 1,113 | 3,000 | 3,522 | 1,268 |
  | calls | 1,577 | 2,535 | 905 | 10,450 | 9,654 |
  | extends | 62,632 | 490 | 69 | 945 | 892 |

  ```sql
  SELECT kind, date_trunc('week',modified_at)::date, count(*)
  FROM sensei.edges GROUP BY 1,2 ORDER BY 1,2;
  ```
- **Structural edges are outnumbered 6.5:1 by containment noise.** Real layout edges (imports+calls+extends+references) = 261,871; `covers` = 1,714,462. A force layout that ingests all edges is dominated by the doc→file duplicates, collapsing structural separation.
  ```sql
  SELECT count(*) FILTER (WHERE kind IN
      ('imports','calls','extends','references')) structural,
         count(*) FILTER (WHERE kind='covers') covers
  FROM sensei.edges;                       -- 261871 | 1714462
  ```
- **Community adjacency is built from an edge kind that is never produced.** `build_adjacency` (community.rs:85) reads `calls`, `implements`, `imports` — but `implements` has **0** rows, and `const` (302,290 nodes = 63%) participates in none of these, so the majority of nodes are singletons that `detect_communities_for_folder` skips (`members.len() < 2`, community.rs:42).
  ```sql
  SELECT kind,count(*) FROM sensei.edges GROUP BY 1 ORDER BY 2 DESC;
  -- covers 1714025 | imports 132741 | extends 65163 | references 38270 | calls 25697
  -- (implements: absent → 0)
  ```
- **The two grouping levels Atlas needs for "packages / sub-projects" are defined but empty.** `node_kind` has 21 values; only 14 are produced. `package`, `section`, `property`, `field`, `parameter`, `enum_variant`, `rationale` = **0 rows**. So nothing sits *above* `file`/`module` — `file`, `module`, `doc` are all `parent_id IS NULL` (top-level), and there is no package/namespace node to nest them under.

  | node kind | count | parent_id populated |
  |---|---:|---:|
  | const | 302,290 | 100% |
  | method | 65,163 | 100% |
  | file | 31,487 | **0% (top-level)** |
  | module | 5,384 | **0% (top-level)** |
  | doc | 3,473 | **0% (top-level)** |
  | package | **0** | — |
  | section | **0** | — |

- **Folders *can* encode sub-projects but don't.** `folder_kind` supports `subtree` and `workspace_member`; the DB has only `folder=6,294`, `git=79`, `standalone=53` — **zero** subtree/workspace_member rows. `folder_role` is NULL on 6,302 of 6,426 folders (only `library=82`, `website=27`, `tool=14`, `docs=1` classified). The "separation by sub-projects/subtrees" grouping has no populated key to render from.
  ```sql
  SELECT kind,count(*) FROM sensei.folders GROUP BY 1;  -- folder 6294|git 79|standalone 53
  SELECT count(*) FILTER (WHERE role IS NULL) FROM sensei.folders;  -- 6302 / 6426
  ```
- **Atlas never reads `community_id` at all.** `atlas-graph.svelte.ts:11-12` states outright: *"communities and symbols are both coloured by KIND, because the communities endpoint carries no per-node membership."* The overview level renders one flat bubble per `inference.communities` row, sized by (inflated) `node_count`, colored by the kind parsed out of the `"{kind} ({dir})"` label string. Above `COLLAPSE_THRESHOLD = 500` nodes it auto-opens on this flat bubble cloud (`initialLevel`, line 352) — so every real repo lands on "scattered circles."
- **The bubbles are near-monochrome.** Community labels resolve to `file` (800 communities), `function` (700), `method` (310) — i.e. paper-mute / accent / accent — so hundreds of same-colored circles of varying size is *exactly* the rendered artifact.
  ```sql
  SELECT split_part(label,' ',1) kind, count(*), sum(node_count)
  FROM inference.communities GROUP BY 1 ORDER BY 2 DESC;
  -- file 800|8791 · function 700|28448 · method 310|6042 · (rest ≤1)
  ```

**Root cause / interpretation.**

The regression is a **data-pipeline collapse compounded by an over-simple viz contract**, not a rendering bug alone. Atlas has exactly two levels: a flat community-bubble overview (fed by raw `inference.communities` rows) and a top-200-by-degree symbol call-graph (`buildSymbolGraph`). Neither expresses the docs-vs-code / package / module / class / method tree, and neither expresses sub-project separation. For any real repo (>500 nodes) Atlas auto-collapses to the bubble overview, so the *only* structure the user sees is the community layer — and that layer is broken three ways.

First, the **community layer covers 1.14% of nodes and is polluted with stale rows**. Label propagation runs on an adjacency built from `calls`/`implements`/`imports` only; `implements` is never emitted and `const` (63% of the graph) has no such edges, so most nodes never join a community of size ≥2 and are skipped. Worse, `community_id` is stored on the `nodes` row, but re-indexing recreates member nodes (new UUIDs → null community) while the `inference.communities` rows survive the FK cascade (they key on `folder_id`, not on node ids). Across 55,665 `detect_communities` executions this leaves 1,814 orphaned community rows whose `node_count` sums to 43,290 while only 5,443 nodes still point back — and folders like `senseid` show 95 communities with 0 live members. `god_node_ids` and `description` are never populated (0/1,814), so the bubbles carry no anchor or explanation either.

Second, the **`covers` edge kind is a duplication bomb**. `insert_edge` has no `ON CONFLICT`, and both `build_connections` and `reconcile_connections` re-insert the full doc-stem×file-stem cartesian product every time they run. Since ~2026-07-13 this inflated `covers` from ~3.8k to 1.71M rows (918× duplication; 2,528× for rokkit alone), making doc→file containment 86% of all edges and outnumbering true structural edges 6.5:1. Any layout or degree metric that ingests the raw edge table is swamped by these duplicates.

Third, the **hierarchy levels that would give "packages" and "sub-projects" were never produced by the current indexer**. The `package`/`section` node kinds and the `subtree`/`workspace_member` folder kinds exist in the enums (evidence the pipeline once intended them) but have zero rows today — so even a hierarchy-aware renderer would have nothing above `file`/`module` to draw, and no sub-project boundary to separate on. The grouping data that *does* exist and is fully populated — `folder_id` (100% of nodes, 177 folders) and the folder tree (`parent_id` on 6,294/6,426 folders) — is simply not consumed by Atlas.

**When it broke:** the `covers` duplication onset is datable to the 2026-07-13 week from the edge cohort table; Atlas itself shipped 2026-07-12 (`ff8299af feat(app): Atlas — code-graph visualization screen (#5)`) and was relocated into the project window 2026-08-04 (`c7f4a945`). The community detector (`community.rs`) has not changed since before 2026-06-18, so the community sparsity is not a new code change — it is the *interaction* of a stable, structurally-limited detector with the June-15 bulk re-index (451k nodes) that was never re-processed for communities, plus the write-back/re-index churn. The visible "used to have hierarchy" refers to the mockup/design intent (nested docs/code/packages/modules) that the shipped Atlas never implemented against real data.

**Recommendations.**

1. **[P0] Make `covers` idempotent.** Add a unique index on `sensei.edges(folder_id, source_id, target_id, kind)` (partial for non-null targets) and change `insert_edge` to `ON CONFLICT DO NOTHING`/`DO UPDATE`. Then one-time dedup the existing 1.7M rows down to 1,866. This alone removes 86% of edge volume and unblocks any layout. (`crates/senseid/src/db/pg_store.rs:1711`, `database/` DDL for `edges`.)
2. **[P0] Stop drowning the layout in containment edges.** In the Atlas graph API and `buildSymbolGraph`, exclude `covers` (and `references`) from the force layout; use only `calls`/`imports`/`extends`/`implements` for positioning. Keep `covers` as an on-demand overlay. (`atlas-graph.svelte.ts`, `crates/senseid/src/api/handlers/codebase.rs`.)
3. **[P0] Render the hierarchy that already exists.** Add a folder/parent-based level to Atlas: group nodes by `folder_id` → folder `parent_id` tree, and within a file by `nodes.parent_id` (method→class→file, 100% populated). This restores docs-vs-code/module/class/method nesting without needing communities at all. (`atlas-graph.svelte.ts`, new `/api/graph/tree` endpoint.)
4. **[P1] Fix community write-back durability.** Either (a) preserve `community_id` across re-index by upserting nodes instead of delete+insert, or (b) make communities membership a join table keyed by node identity that is rebuilt atomically per detect run, and delete stale `inference.communities` rows for a folder before re-upserting. Populate `god_node_ids` and `description`. (`crates/senseid/src/indexer/community.rs`, `pg_store.rs:2601`.)
5. **[P1] Broaden community adjacency and drop the dead kind.** Include `extends`/`references` (and real containment via `parent_id`) in `build_adjacency`; remove `implements` until it is actually produced, or start producing it. Re-run detection over the full graph, not just recently-touched folders. (`community.rs:85`.)
6. **[P1] Backfill the missing grouping levels.** Emit `package`/namespace nodes above `module`/`file`, and classify `folders.kind` as `subtree`/`workspace_member` during scan (the scan_logic already finds sub-projects — `find_subprojects_covers_members_and_standalone_apps`). This gives Atlas a real sub-project boundary. (`crates/senseid/src/tasks/handlers/scan_logic.rs`.)
7. **[P2] Re-index the June-15 bulk cohort.** 94.6% of the graph predates the current community + edge pipeline; schedule a full re-process so coverage metrics reflect current code, not June state.

**Proposed metrics & instrumentation.**

| Metric | Definition / formula | Source (table.column) | Cadence | Current gap |
|---|---|---|---|---|
| Community coverage % | `count(community_id)/count(*)` over nodes | `sensei.nodes.community_id` | per index run | 1.14% today; no dashboard |
| Community table integrity | `sum(node_count)` vs `count(nodes with id)` per folder | `inference.communities.node_count` vs `nodes.community_id` | per detect run | 43,290 claimed vs 5,443 real; unmeasured |
| Edge duplication factor | `count(*) / count(DISTINCT (source_id,target_id,kind))` | `sensei.edges` | per index run | 918× on `covers`; no guard |
| Structural:containment ratio | structural edges / `covers` edges | `sensei.edges.kind` | per index run | 6.5:1 inverted; untracked |
| Hierarchy completeness % | non-top kinds with `parent_id` / total; folders with `role` set | `sensei.nodes.parent_id`, `sensei.folders.role` | per index run | roles 1.9% set; no metric |
| Grouping-level presence | rows of `package`/`section` nodes; `subtree`/`workspace_member` folders | `sensei.nodes.kind`, `sensei.folders.kind` | per scan | 0 rows; silently missing |
| Index freshness by cohort | node count by `week(modified_at)` vs current commit | `sensei.nodes.modified_at` | daily | 94.6% stuck at 2026-06-15 |
| Community enrichment % | rows with `description` and `god_node_ids` set | `inference.communities` | per detect run | 0% enriched |
