---
name: Architecture metrics — proposed family
type: feature-detail
kind: functional
module: project
parent: ./feature.md
status: proposed
updated: 2026-09-08
tags: [metrics, architecture, graph, coupling, cohesion, proposal]
---

# Architecture metrics — proposed family

sensei computes 26 live metrics and **none of them read the code graph.** This
document proposes the missing `architecture` family, with the feasibility of each
candidate *measured against the real graph* rather than assumed.

## Why this gap exists and why it's worth closing

The raw material is substantial and already indexed:

| table | rows | what it holds |
|---|---|---|
| `sensei.edges` | 811,071 | `calls`, `imports`, `references`, `extends`, `implements`, `depends_on`, `covers`, `duplicates`, `similar_to` |
| `sensei.nodes` | 395,016 | symbols with `kind`, `is_exported`, `is_test`, `line_start`/`line_end`, `fqn`, `community_id`, `embedding` |
| `inference.communities` | 32,495 | label-propagation clusters |

The nine metrics in the `quality` family measure *code hygiene* — duplication,
smell density, churn, coverage. None of them measure **structure**: whether
modules depend on each other in one direction or many, whether a change in one
place forces a change in another, whether the dependency graph has cycles.
That is the difference between "is this code clean?" and "is this system sound?",
and it is the question a maintainer actually asks before a refactor.

## What is measurable — and what is not

Two structural facts constrain everything below. Both were measured, not assumed.

### Fact 1 — `folder_id` is repo-grained, not module-grained

Of 9,378 folders, only **179 have any nodes**, and the ones that do are
repository roots (`kind = 'git'`). Every resolved edge has its source and target
in the *same* `folder_id`:

```
kind        total    cross_folder
calls      245,828             0
imports    141,980             0
references  78,700             0
```

So `folder_id` cannot serve as a module boundary. **Modules must be derived from
`nodes.file_path`** (repo-relative for 98.3% of nodes; the 338 absolute ones are
directory entries from the folder walk). At a 3-segment prefix
(`crates/senseid/src`), sensei resolves to 246 modules; at 2 segments, 93.

### Fact 2 — cross-module edges are too sparse today to support module metrics

Using the depth-3 prefix on sensei's own graph:

| kind | resolved edges | cross-module | % |
|---|---|---|---|
| `calls` | 9,985 | 579 | 5.8% |
| `imports` | 2,272 | 738 | 32.5% |
| `references` | 2,409 | 2,409 | 100% |
| `implements` | 80 | 0 | 0% |
| `extends` | 3 | 0 | 0% |

Collapsed to distinct module→module dependency pairs, the entire sensei
repository yields **13 edges**, of which 2 sit in a 2-cycle. All 13 are
TypeScript/frontend — the Rust side contributes essentially none, because Rust
call resolution is the known-broken part
([#146](https://github.com/sensei-hq/sensei/issues/146),
[#151](https://github.com/sensei-hq/sensei/issues/151),
[#152](https://github.com/sensei-hq/sensei/issues/152)).

Repo-wide, edge resolution is 58.6%, and it is very uneven by kind:

| resolution_class | edges | share |
|---|---|---|
| resolved | 474,925 | 58.6% |
| no-local-name | 248,613 | 30.7% |
| name-collision-n | 50,017 | 6.2% |
| name-collision-1 | 37,522 | 4.6% |

`imports` resolves at ~100%, `references` at 34%, `extends` at 96% but on only
1,817 edges total.

**Conclusion: the module-level metrics are blocked on the resolver, not on metric
design.** Building them now would produce confident-looking numbers over 13
edges. That is exactly the fabricated-signal failure the project's hard rules
forbid. They are specified here so they can be built the moment resolution lands.

## Tier 1 — computable today (no resolver work needed)

These read node attributes, not edge topology, so the resolution gap doesn't
touch them. All three were prototyped successfully against sensei's graph.

### `graph_confidence` — the prerequisite gauge
- **Facets:** architecture · pct · ▲ · proposed weight 0 (descriptive, gating)
- **Definition:** resolved edges ÷ total edges, per repository.
- **Calculation:** `count(edges where target_id is not null) / count(edges)`, optionally split by `kind`.
- **Source:** `sensei.edges` / `sensei.edge_resolution_class` — **fully populated today.**
- **Measured:** 58.6% overall; `imports` ~100%, `references` 34%.
- **How to read:** this is not a code-quality signal — it is a **measurement-quality**
  signal, and it must gate every other architecture metric. An architecture chart
  built on 58.6%-resolved edges has to say so; a coupling number derived from
  34%-resolved `references` is noise. Ship this **first** so the rest can carry an
  honest confidence badge.
- **Representation:** gauge + a per-kind breakdown bar; rendered as a caveat chip on
  every architecture visual, never as a standalone "score".
- **Feasibility:** ✅ computable now, one query.

### `public_surface_ratio` — encapsulation
- **Facets:** architecture · pct · ▼ · proposed weight 1
- **Definition:** exported symbols ÷ total symbols, per repository (later per module).
- **Calculation:** `count(nodes where is_exported and kind in (function, method, class, struct, interface, type, enum, const)) / count(same set)`.
- **Source:** `sensei.nodes.is_exported` — populated (97,287 of 395,016 nodes exported).
- **Measured on sensei:** 5,517 of 12,605 symbols exported = **43.8%**.
- **How to read:** a high ratio means most of the codebase is public API — every
  symbol is a commitment and nothing is free to change. Lower is generally better,
  but it is **language-sensitive** (Rust `pub` vs TypeScript `export` vs Python's
  no-op convention), so it is only comparable *within* a language, and the metric
  must carry the language mix. Read with `duplication_ratio`: a large public
  surface plus high duplication is the signature of a module that was copied
  rather than extended.
- **Representation:** gauge per language; module × ratio heatmap once module attribution lands.
- **Feasibility:** ✅ computable now. Needs a per-language denominator to be fair.

### `symbol_size_p95` — god-function detection
- **Facets:** architecture · count (lines) · ▼ · proposed weight 1
- **Definition:** the 95th-percentile function/method length in lines, plus the
  count above a threshold.
- **Calculation:** `percentile_cont(0.95) over (line_end - line_start)` for
  `kind in (function, method)`; companion count of symbols `> 100` lines.
- **Source:** `sensei.nodes.line_start`/`line_end` — populated for 9,123 of sensei's functions.
- **Measured on sensei:** 75.4% under 20 lines, 5.0% at 50–100, 1.4% at 100–200,
  **0.3% (26 functions) at ≥200 lines**.
- **How to read:** overlaps `module_quality` (qlty already flags function
  complexity), so it earns its place only by being **per-symbol and addressable** —
  it names the 26 functions, where qlty gives a ratio. Prefer p95 over mean: the
  mean is dragged down by thousands of one-line accessors and hides the tail that
  matters. Not a quality verdict on its own — a long match-arm dispatcher is fine.
- **Representation:** distribution histogram + a ranked "largest symbols" list that
  links into the file.
- **Feasibility:** ✅ computable now. Check for redundancy against qlty's
  function-complexity smell before shipping both.

## Tier 2 — specified, blocked on edge resolution

Each of these was prototyped and *works as SQL*; each is starved of input. They
become viable when cross-module resolved edges reach a usable density. Suggested
gate: **≥500 distinct module→module edges** for the repository, and
`graph_confidence` above a stated floor, or no row (honest-empty).

### `module_coupling` — cross-boundary dependency share
- **Facets:** architecture · pct · ▼ · proposed weight 2
- **Definition:** resolved dependency edges crossing a module boundary ÷ all resolved dependency edges.
- **Calculation:** derive `module` = N-segment prefix of `nodes.file_path`; over
  `edges` where `kind in ('imports','calls')` and `target_id is not null`, take
  `count(src_module <> tgt_module) / count(*)`.
- **Measured on sensei:** imports 32.5%, calls 5.8% — but on only 738 and 579 edges.
- **How to read:** the headline coupling number. High = modules are entangled and a
  change anywhere propagates. **Deeply sensitive to the module-boundary depth
  choice** (93 modules at depth 2, 246 at depth 3, and the ratio moves with it), so
  the depth must be recorded in `props` and held fixed per repository, or the trend
  measures the definition rather than the code. Consider per-language depth, or
  deriving the boundary from workspace/package manifests (`Cargo.toml` members,
  `package.json` workspaces) instead of raw path depth — sensei already indexes
  `folder_kind = 'workspace_member'`.
- **Feasibility:** ⚠️ blocked. Query works; input too sparse.

### `dependency_cycles` — cycle count
- **Facets:** architecture · count · ▼ · proposed weight 2
- **Definition:** number of strongly-connected components of size > 1 in the
  module dependency graph (and the count of modules trapped in one).
- **Calculation:** build the distinct module→module edge set, then find SCCs (a
  recursive CTE handles 2-cycles directly; full SCC needs Tarjan in the handler).
- **Measured on sensei:** 13 module-dependency edges, **2 in a 2-cycle.**
- **How to read:** the single most actionable architecture signal — a cycle means
  two modules cannot be understood, tested, or released independently. Unlike
  coupling it is not a matter of degree: a cycle is a defect with a name and a
  fix. Report the *members*, not just the count.
- **Feasibility:** ⚠️ blocked. Detection is correct on the data that exists;
  there just isn't enough of it to trust a zero.

### `instability` — Martin's I, per module
- **Facets:** architecture · ratio · ● · proposed weight 1
- **Definition:** `fan_out / (fan_in + fan_out)` per module. 0 = maximally stable
  (depended upon, depends on nothing), 1 = maximally unstable.
- **Calculation:** per module, count distinct outgoing and incoming module deps.
- **Measured on sensei:** produced sane values on the 13 edges available —
  `app/e2e/fixtures.ts` I=0.00 (fan-in 3, fan-out 0), `app/e2e/tests` I=1.00
  (fan-out 2, fan-in 0), `app/src/routes` I=0.50.
- **How to read:** `neutral` by design — neither extreme is wrong. Its value is
  the **pairing with abstractness** (Martin's main-sequence distance): a module
  that is both stable *and* concrete is the painful one, because everything
  depends on it and it can't be extended. Abstractness needs
  `kind in ('interface','type')` share, which sensei has (7,131 interfaces, 4,454
  types) — so the main-sequence chart is reachable once edges resolve.
- **Feasibility:** ⚠️ blocked on the same edge density. Note the prototype's
  cleanest results came from test folders, which is a warning that test code will
  dominate unless `is_test` is excluded.

### `layering_violations` — declared-order breaches
- **Facets:** architecture · count · ▼ · proposed weight 2
- **Definition:** dependency edges pointing "upward" against a declared layer order.
- **Blocker beyond resolution:** **there is no declared layer order.**
  `folders.role` is populated for only 154 of 9,378 folders (`library` 104,
  `website` 28, `tool` 21, `docs` 1) and 9,224 are null. This metric needs a
  per-project layer declaration (a `.sensei/` config or an inferred topological
  order) before it can exist at all.
- **Feasibility:** ❌ blocked on two things — edge resolution *and* a layer
  declaration. Lowest priority of the four.

## Tier 3 — deliberately not proposed

- **Cohesion via `community_id`.** Tempting, since 32,495 communities exist, but
  [#149](https://github.com/sensei-hq/sensei/issues/149) already measured that 62%
  of communities merely restate the file boundary and 36% are stub singletons. A
  cohesion metric over that would measure the clustering algorithm, not the
  architecture. Revisit only if #149 changes the clustering.
- **Embedding-similarity cohesion.** `nodes.embedding` (384-dim) could give
  semantic cohesion per module, but it would be a novel, unvalidated definition
  with no external reference point — and the duplication metric already retired
  its own-graph embedding approach in favour of `qlty`. Not worth reintroducing
  the same class of unverifiable signal.

## Recommended sequence

Forward-only — no step depends on a later one:

1. **`graph_confidence`** (Tier 1). Ship first: it is the honest caveat every
   later metric needs, and it is one query over fully-populated data.
2. **`public_surface_ratio`** + **`symbol_size_p95`** (Tier 1). Independent of
   edges. Check `symbol_size_p95` for redundancy against qlty first.
3. **Resolver work** — [#146](https://github.com/sensei-hq/sensei/issues/146),
   [#151](https://github.com/sensei-hq/sensei/issues/151),
   [#152](https://github.com/sensei-hq/sensei/issues/152). Not metric work, but
   nothing in Tier 2 is trustworthy until it lands.
4. **Module boundary definition** — decide path-depth vs workspace-manifest, record
   it in `props`. A prerequisite for all of Tier 2, and cheap to do while 3 is in flight.
5. **`dependency_cycles`**, then **`module_coupling`**, then **`instability`**
   (Tier 2). Cycles first: most actionable, least sensitive to the boundary choice.
6. **`layering_violations`** last, and only after a layer-declaration mechanism exists.

## Non-negotiables for whoever builds this

- **No row when the input is insufficient.** Below the edge-density gate, or below
  the `graph_confidence` floor → honest-empty. Never a 0 that reads as "no cycles".
- **Record the module-boundary definition in `props`.** A trend across a changed
  definition is not a trend.
- **Exclude `is_test` nodes from coupling and instability**, or report test and
  non-test separately — the prototype showed test folders dominating the results.
- **Per-language denominators** for `public_surface_ratio`. `pub`, `export` and
  Python convention are not the same commitment.
- Register through `database/import/staging/metrics.jsonl` like every other metric,
  with a `rating_scale` whose **unit matches the stored value** — see
  [#154](https://github.com/sensei-hq/sensei/issues/154) for what happens when it doesn't.

## Related

- [[features/metrics/catalog]] — the 26 live metrics · [[features/metrics/feature]] — why + philosophy
- [`docs/design/metrics-chart-brief/`](../../design/metrics-chart-brief/) — chart hand-off, incl. the architecture gap
- Tracking: [#156](https://github.com/sensei-hq/sensei/issues/156). Blockers:
  [#146](https://github.com/sensei-hq/sensei/issues/146),
  [#151](https://github.com/sensei-hq/sensei/issues/151),
  [#152](https://github.com/sensei-hq/sensei/issues/152),
  [#149](https://github.com/sensei-hq/sensei/issues/149)
