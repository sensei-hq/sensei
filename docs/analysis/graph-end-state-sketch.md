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

---

## 7. The identity question, answered

Asked: could `lib_package` just BE `package`, parented to a `library` the way an
internal package is, with a `library_id` reference when a library row exists — and
is `library_id` needed at all, if internal-vs-external is knowable from whether a
file is associated?

**Yes to the unification, and no to `library_id`.** Both parts hold up against the
data, with one caveat.

### `file_path` already separates internal from external

```
kind          total    no file    has file
lib_symbol    7,424     7,424           0
lib_package   1,091     1,091           0
method       59,571         0      59,571
section     140,016         0     140,016
file/module  58,432         0      58,432
...every other kind      0       all filed
function    130,958    84,379      46,579   ← the exception, and it is a BUG
```

Every kind is cleanly on one side **except `function`**, where 84,379 nodes (64%)
have no file — and those are the misattributed stubs of §7.1 below, not real
functions. **Once those are fixed, "has a file" is a clean discriminator and no
flag is needed.**

Caveat worth stating: it means "no file **in this folder**". A library that is also
checked out locally — rokkit is both an app dependency and a repo at
`/Users/Jerry/Developer/rokkit` — does have files, in a different folder. The
discriminator is a scoping fact, not an ontological one.

### `library_id` on the node would be a second copy

The lookup already has a single owner: package NAME → `sensei.library_packages` →
`library_id`, declared by the library itself and landed in `6eea76bf`. Adding
`library_id` to the node would make two places assert the same mapping, and they
would drift — which is the failure that has now bitten four times in one day (the
scan exclusion resolver, the two manifest readers, the import classifier, and §7.1
below). A join by name costs one index lookup and cannot go stale.

So: `package` nodes parented to a `library` node, `lib_symbol` parented to its
`package`, internal-vs-external read off `file_path`, and the library row reached
by name when someone actually wants its docs or skills.

### 7.1 The measurement that makes this urgent

While testing the file-path discriminator: **of 207,874 "resolved" call edges,
150,157 (72%) point at a file-less node.** Split by target kind:

```
target_kind   resolved_edges   of which file-less
function             149,386            108,174    ← stubs
lib_symbol            41,983             41,983    ← correct
method                15,541                  0
class                    806                  0
struct                   158                  0
```

Against all 320,692 call edges:

| | edges | share |
|---|---:|---|
| resolve to real local code | 57,717 | **18.0%** |
| resolve to a proper `lib_symbol` | 41,983 | 13.1% |
| resolve to a misattributed stub | 108,174 | **33.7%** |
| unresolved | 112,818 | 35.2% |

Sampled stubs: `rust·senseid·api::handlers::observatory·HashMap·get`,
`rust·senseid·api::handlers::logs·Ok`, `rust·dbd-core·adapter::mock::tests·Some`.
These are **std** symbols minted as `function` nodes inside the CALLING module's
namespace — the graph asserting that `HashMap::get` is senseid code.

So the "64.8% resolved" figure this document opened with is inflated **3.6×**. It
also explains #141: `get_callers` missing a caller is expected when the graph
reaches real local code 18% of the time. Filed as #146.

An unresolved edge is honest. A stub that names an external symbol and claims local
provenance is a wrong answer dressed as a resolved one — worse than the gap it
fills, and the reason §5 step 1 (one classifier, everywhere) is the first thing to
do rather than the third.

---

## 8. Language internals — bundle them, but not into one node per language

Asked: do we need language internals at all, or should they be bundled as
`rust:internal`?

**Bundle, yes. To one node per language, no** — that loses the only part of them
that carries signal.

### What the internals actually are

After fixing #146 those 108,174 stub edges have to land somewhere. Measured, they
are 56,699 distinct nodes over 16,146 distinct symbol names, and the top names show
they are not one population:

```
when            3,055   ← Mockito
Some            3,029   ← Rust std
new             2,701   ← constructor, every language
assertEquals    2,230   ← JUnit
Ok              2,228   ← Rust std
String          2,175   ← builtin
assertNotNull   1,940   ← JUnit
any             1,719   ← Mockito
getId           1,660   ← Lombok-GENERATED accessor
setId           1,630   ← Lombok-generated
anyLong         1,502   ← Mockito
default         1,297   ← builtin
anyString       1,079   ← Mockito
fetch           1,021   ← runtime builtin
```

Three groups with very different value:

| group | example | signal to an agent |
|---|---|---|
| language builtins / syntax | `Some`, `Ok`, `new`, `String`, `default` | **none** — every Rust function calls `Some` |
| third-party framework symbols | `when`, `assertEquals`, `anyLong` | the PACKAGE is the signal ("this is a Mockito test"), the symbol is not |
| behaviourally meaningful stdlib | `std::fs::read_to_string`, `std::process::Command`, `std::thread::spawn` | **high** — this is what a security or blocking-IO review looks for |

### Why one node per language is too coarse

`rust:internal` collapses `std::fs` and `std::sync` and `std::process` into one
bubble. That throws away the third group — and "what in my codebase touches the
filesystem / spawns a process / blocks a thread" is one of the few graph questions
an agent genuinely cannot answer from grep.

### The cut that keeps the signal

Keep **package·module** as the external rung, and drop the per-symbol node:

```
lib_package  std::fs          ← module-level, so fs / process / sync stay distinct
lib_package  org.mockito
lib_package  node:fs
lib_package  @rokkit/ui
```

and point the call edge at THAT, not at a symbol node.

- keeps: "this file depends on `std::fs`", "this is a Mockito test", "this touches
  the filesystem"
- loses: "this file calls `anyLong`" — which nobody asks
- costs: 56,699 symbol nodes collapse to a few thousand package·module nodes

`external_package` already produces this granularity for two of the three shapes —
dotted paths collapse to two segments (`java.util`, `org.mockito`) and `node:`
schemes keep their module (`node:fs`). Rust is the gap: it currently yields `std`,
while the FQN carries the module path (`lib·std·std::sync::mpsc·channel`), so
`std::sync::mpsc` is available and simply not used as the rung.

### The catch, and where it leads

For a STUB the resolver often knows only the bare symbol name — `when`, with no
package attached. That is precisely why it stubbed instead of taking the
`lib_symbol` branch.

The package is recoverable, from a source already in the graph: **the file's own
import edges.** A file calling `when` that imports `org.mockito.Mockito.when` tells
you the package unambiguously. Which means the resolution order is:

1. resolve imports (they name their packages explicitly — the earlier step 3)
2. use the file's resolved imports to attribute bare call names to packages
3. anything still unattributable stays UNRESOLVED, honestly

So the import work is not a parallel track to the call work — it is the input to
it. That is a change to the sequencing in §5: imports move ahead of the call-path
correction, because the correction needs them.
