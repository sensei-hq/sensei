# The graph's end state — sketched with worked examples

**Status:** design sketch, no indexing changed. Written 2026-09-01 against live
measurements (430,874 nodes · 715,985 edges · 18 node kinds · 10 edge kinds).

Asked for before touching indexing, and the measuring changed what needs building:
**most of the target shape already exists.** Three things are wrong with it, and one
of them is corrupting the call graph today.

---

## 1. What already exists

| piece | state |
|---|---|
| `lib_package` node kind | **exists**, 1,091 nodes |
| `lib_symbol` node kind | **exists**, 7,424 nodes, each carrying `props.package` |
| library → symbol containment | **exists** — 7,424 of 7,424 `lib_symbol` nodes are parented |
| package → library link | **exists** as of `6eea76bf` — `sensei.library_packages`, declared in `sensei.library.json` |
| containment for local code | **exists** — `nodes.parent_id`, 296,660 of 430,874 nodes (68.9%) |
| calls as links | **exists** — 320,692 edges, 60–73% resolved in every language |

So `library → package → module → item` is not a new hierarchy to invent. It is
built, populated, and — for external symbols — complete.

## 2. What is wrong

### 2.1 30% of "library packages" are local code

```
props.package on lib_symbol nodes, top values:
  @/lib                 1,396     ← tsconfig alias — LOCAL
  java                    514
  $lib                    425     ← SvelteKit alias — LOCAL
  @omniroute/open-sse      423
  std                      421
  @/shared                  292   ← LOCAL
  @/services                 91   ← LOCAL
```

**2,204 of 7,424 (30%)** local modules are recorded as external library packages,
with FQNs like `lib·$lib·$lib/server/metrics-ingest·ingestMetrics`. `$lib/server/
metrics-ingest` is this repository's own code.

Same alias-vs-package confusion the import classifier had (`91df2956`), reached
independently by the call path. Which is the real diagnosis: **each path decides
"is this external?" on its own, and they disagree.**

### 2.2 `contains` is not a relation

`edge_kind` has ten values — `calls`, `implements`, `extends`, `imports`,
`depends_on`, `traces_to`, `references`, `covers`, `rationale_for`, `duplicates`
— and **no `contains`**. Containment is `nodes.parent_id`.

That is defensible as storage (exactly one parent, FK-enforced, cheap tree reads)
but it makes containment **invisible to every edge-based consumer**. The graph view
calls `get_edges_scoped_kinds(["calls","imports","extends"])`, so it cannot render
the grouping at all — the bubbles have to be assembled from a different query
against a different shape.

### 2.3 The shared wire format is partly ignored

`FileProcessResult` IS a shared contract, and it has a `language` field. Measured:
`code::process` fills it, `doc::process` sets it to `None`, and **nothing reads
it** — `upsert_node_ex` re-derives language from the file extension instead. A
field three writers touch and no reader consumes.

That is why `.mdx` and `.txt` came out with no language at all (fixed in
`6159c6b7`): the decision had moved to a path-only function that cannot see
content.

---

## 3. The end state, worked

### Example A — a local call, resolved

```
file    app/src/routes/(observatory)/settings/metrics/+page.svelte
  ├─contains→ module  settings/metrics
  │             ├─contains→ function  toneClass
  │             └─contains→ function  addExclusion
  └─calls→    function  worstReason        (in metric-status-state.svelte.ts)
```

`contains` gives the bubbles; `calls` gives the line that crosses out of the
bubble. Today the first three arrows are `parent_id` and only the last is an edge.

### Example B — an external call, grouped under its library

```
library  rokkit                                   ← sensei.libraries
  └─contains→ lib_package  @rokkit/ui             ← via library_packages (6eea76bf)
                └─contains→ lib_symbol  List
                                 ↑
app/…/+page.svelte ─calls→ ──────┘
```

The pieces exist: `lib_package` nodes, parented `lib_symbol` nodes,
`props.package`, and now the package→library link. What is missing is that
`props.package` is a STRING, not an edge to the `lib_package` node — so the walk
from `List` up to `rokkit` is a string join, not a traversal.

### Example C — what 2.1 produces today, wrongly

```
lib_package  $lib                                 ← NOT a library
  └─contains→ lib_symbol  ingestMetrics
                fqn: lib·$lib·$lib/server/metrics-ingest·ingestMetrics
```

Should be:

```
file  src/lib/server/metrics-ingest.ts
  └─contains→ function  ingestMetrics             ← a LOCAL node that already exists
```

So the fix is not new structure. It is **routing the decision through one
classifier** — the same `classify_import` that already knows `$lib` and `@/` are
local — so the call path stops minting library packages for this repository's own
modules.

### Example D — an import, after resolution

```
src/lib/metrics.ts ─imports→ lib_symbol  node:fs        (external, grouped under node:fs)
src/lib/metrics.ts ─imports→ file        src/lib/util.ts (local, resolved)
```

Today both are `target_name` strings with `target_id = NULL`. Note the constraint
found earlier: `target_id` and `target_name` are mutually exclusive across all
715,985 edges — resolving an import **erases the name**, so the target node must
carry it. A package-keyed `lib_symbol` does exactly that.

---

## 4. Node granularity — the question asked

Measured inventory, with a recommendation per kind:

| kind | nodes | keep? | why |
|---|---:|---|---|
| `function` / `method` | 190,529 | **yes** | the unit a call points at, and the unit an agent edits |
| `class` / `struct` / `interface` / `type` / `enum` | 22,669 | **yes** | the unit a method hangs off; needed for `extends`/`implements` |
| `file` / `module` | 58,432 | **yes** | the containment spine |
| `section` / `doc` / `rationale` | 147,550 | **yes** | this is the docs corpus `get_lib_docs` serves |
| `lib_package` / `lib_symbol` | 8,515 | **yes** | the external side of the hierarchy |
| `const` | 3,061 | **yes** | 2,145 of them (70%) are exported — a shared constant is a real dependency an agent must not break. The 916 unexported ones are the arguable half |
| `component` / `hook` / `extension` | 119 | **review** | 2 components and 1 hook on this install looks like an extractor that almost never fires |
| variables / locals | **0** | — | **not indexed today.** The question is moot, and the answer should stay "no": a local is invisible outside its function, so it can neither be called nor imported |

On function props (params, returns): they belong **on the function node**, not as
child nodes. A parameter is not something you call or import — it is a fact about
the signature, which is what `nodes.signature` already holds. Making each param a
node would add roughly 400k rows that no edge can ever point at.

---

## 5. What changes, in order

Forward-only. Each step is independently verifiable, and the first two are
corrections rather than new structure.

1. **One classifier decides external-vs-local, everywhere.** Route the call path's
   `is_lib` decision through `classify_import`, so `$lib` and `@/…` stop minting
   library packages. Fixes 2,204 mislabelled nodes. Verifiable: `props.package`
   should contain no `@/` or `$` values afterwards.
2. **Make `props.package` a real edge** — `lib_symbol ─contains→ lib_package
   ─contains→ library`. Lets the walk to a library be a traversal, and lets
   step 3 of the earlier plan (resolving imports to `lib_symbol` nodes) land on a
   hierarchy that already connects.
3. **Expose containment on the edge read**, without duplicating it. `parent_id`
   stays the single source of truth; the graph read presents it as a `contains`
   relation alongside edges. A `contains` EDGE table would be a second copy of the
   same fact, which is the failure that has bitten three times in one day
   (exclusion resolver, manifest readers, import classification).
4. **Then** resolve imports (the earlier step 3), which by then has a populated
   external hierarchy to point at.

## 6. What I am unsure about

- Whether `lib_package` and `library` should be one rung or two. Today
  `sensei.libraries` is a table and `lib_package` is a node kind — the same
  concept in two stores. Worth deciding before step 2 wires edges between them.
- ~~Whether `const` should be filtered by `is_exported`.~~ Measured after writing
  this: **2,145 of 3,061 (70%) are exported**, so the kind earns its place as-is.
  Only the 916 unexported ones are arguable, and they are cheap.
