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

---

## 9. One `package` kind, edge coverage, and what can be rendered today

Four questions, three of which the data answers outright.

### 9.1 One `package` kind with a DERIVED flag — yes, and it is unambiguous

Measured: `module` is **100% filed** (23,092 nodes, zero file-less) and
`lib_package` is **100% file-less** (1,091 nodes, zero filed). The two populations
are perfectly disjoint on `file_path`, so collapsing them into one `package` kind
introduces no ambiguity at all.

Derive `is_external` in a VIEW, not a column — `file_path IS NULL` already carries
the fact, and a stored flag would be a second copy of it. That is the same rule
that settled `library_id` in §7, and the failure it avoids has now bitten five
times in one day.

The UI should read the view's flag rather than re-deriving from `file_path` itself,
for the same reason: one owner. Render external groups distinctly (muted fill,
different border) — the distinction matters to a reader because an external bubble
is a boundary they cannot edit.

Caveat carried from §7: "file-less" means "no file **in this folder**". A library
also checked out locally has files, elsewhere. For a per-repository graph view that
is exactly the right scoping; for a cross-repo view it would need care.

### 9.2 Are all edges covered? No — 36% is untouched

```
kind             total   resolved   name_only   neither
calls          320,715    207,874     112,841         0
references     250,072          0     250,072         0
imports        136,484          0     136,484         0
extends          7,863          0       7,863         0
covers             601        601           0         0
implements           0          –           –         –
depends_on           0          –           –         –
traces_to            0          –           –         –
rationale_for        0          –           –         –
duplicates           0          –           –         –
similar_to           0          –           –         –
```

The plan covers `calls` + `imports` = 457,199 of 715,735 edges (**64%**).

Untouched: **`references` 250,072 (35%)** and **`extends` 7,863 (1%)**, both at 0%.
`references` is the single largest unresolved block in the graph and has no plan
attached to it.

Also: **6 of 11 edge kinds have zero rows** — declared in the enum and never
written. `rationale_for` is the striking one: 1,700 `rationale` nodes exist and are
parented, but nothing links a rationale to the thing it justifies, which is the
entire point of having them.

### 9.3 Can the view render nested bubbles today? Yes — no schema change needed

`parent_id` is **already on the wire**: the graph node query selects
`id, kind, name, file_path, parent_id, line_start, line_end, degree, community_id,
folder_id, language, fqn, resolved, is_test`. And external nodes are parented too
(7,424 of 7,424 `lib_symbol`).

So the nesting the visualisation wants is fully derivable from data the endpoint
already returns. **The gap is in the client, not the model** — which also confirms
§2.2's conclusion: do not add a `contains` edge kind. `parent_id` is sufficient and
already delivered.

### 9.4 Edges with no target — none, but 71% cannot be drawn

Two readings, and they differ sharply:

* **No target at all** (`target_id` and `target_name` both null): **zero**, across
  all 11 kinds. Every edge names something.
* **No target NODE** (`target_id` null): **507,260 of 715,735 — 70.8%.** These name
  a target without pointing at a node, so a renderer cannot draw them as a line
  between two bubbles.

After the plan (`calls` + `imports` resolved) that would fall to roughly the
`references` + `extends` share — about **36%** — and the remaining gap would be the
largest single thing still owed to the visualisation.

Worth stating plainly: the render is possible today, but seven of every ten edges
have nowhere to land.

---

## 10. The six empty edge kinds, and why 71% of edges have no target node

### 10.1 The empty kinds: declared, never written

Six of eleven `edge_kind` values have **zero rows**, and none has a writer
anywhere in the crate:

| kind | intent (from the code that would use it) | status |
|---|---|---|
| `implements` | interface/trait implementation | no writer. `extends` is used for everything structural instead |
| `depends_on` | module/package dependency | no writer. `imports` covers the same ground |
| `traces_to` | requirement/spec → code traceability | no writer. The traceability screen exists; the edge does not |
| `rationale_for` | a rationale → the decision it justifies | no writer — **and 1,700 `rationale` nodes exist**, parented, linked to nothing |
| `duplicates` | near-duplicate symbols | no writer. `get_duplicates` computes them on the fly and returns JSON |
| `similar_to` | semantic similarity | no writer. Embedding search is done at query time |

Two different situations. `duplicates` and `similar_to` are computed at query
time — the enum value is aspirational and arguably should go, since a value nothing
can write is a promise the schema cannot keep. `rationale_for` and `traces_to` are
real gaps: the nodes and the screens exist, the connection does not.

### 10.2 Why 507,260 edges (70.8%) name a target but point at no node

Four different causes, one per kind. None is "the resolver tried and failed".

**`imports` — 136,484, by construction.** `process.rs:1351` inserts every one with
`target_id = None`, iterating a field named `unresolved_imports`. Nothing has ever
attempted resolution. Analysed in §4.1 and #146.

**`references` — 250,072, doc mentions with no resolver.** Written only for
documents:

```rust
if result.kind == "doc" {
    for file_ref in &result.file_refs { insert_edge(…, Some(file_ref), …, "references") }
    for fn_ref in &result.fn_mentions { insert_edge(…, Some(fn_ref), …, "references") }
}
```

So a `references` edge is *this doc mentions this file path* or *this doc mentions
this symbol name*. Both are resolvable in principle — a path like a relative
import, a symbol by name within the repository — and nothing tries. This is the
largest single unresolved block in the graph and the one with no plan attached.

**`extends` — 7,863, mislabelled containment from the wrong source.** The writer:

```rust
// Parent refs (HAS_METHOD: type → method).
for pref in &result.parent_refs {
    insert_edge(&folder_id, &file_node_id, None, Some(&pref.parent_name), None, "extends")
}
```

The comment says HAS_METHOD — type owns method, which is **containment**, not
inheritance. And the source is the FILE node, not the type. So each edge reads
"this file extends a name" when it means "a type in this file owns a method".
`nodes.parent_id` already records that same relation correctly (method → type).

So `extends` is redundant with `parent_id`, sourced from the wrong node, and named
after a relation it does not represent. It is the one kind here that should
probably be deleted rather than resolved.

**`calls` — 112,841, receiver type unknown.** The honest residual, and uniform
across languages (60–73% in each, §7.1). Sampled TypeScript targets: `stringify`,
`isArray`, `now`, `toLowerCase`, `prepare`, `subscribe`, `floor`. The AST gives the
call site and the method name; resolving needs the receiver's TYPE, which needs
inference the parser does not do. `x.toLowerCase()` parses perfectly and is
unresolvable without knowing what `x` is.

Note most of those are runtime builtins (`JSON`, `Array`, `Date`, `Math`), so they
belong in §8's package·module rung rather than being resolution failures at all.

### 10.3 Depth, kind, and root class — all three axes already exist

The proposed controls need no new data:

* **depth / level** — `parent_id` chains, already on the wire (§9.3)
* **kind** — 18 node kinds, already returned
* **root class** — derivable today:

```
code    258,421      everything else
doc     147,579      kind in (section, doc, rationale)
test     24,904      is_test = true
```

So "show me only the code graph", "show me only packages", "collapse to depth 2"
are all client-side filters over data the endpoint already sends. `nodes.tags` is
NOT the axis to use for this — only 1,685 of 430,874 nodes carry any tag (0.4%),
so it is effectively empty.

---

## 11. The uniform pipeline, get-or-create, and where patterns belong

Three propositions, and the evidence splits them: one is already true, one is
already implemented **and is the cause of #146**, and one is right with a
correction.

### 11.1 "parser → AST → nodes + edges → persist, for all adapters" — already the shape

That pipeline exists:

```
router::process → code::process → adapter.parse(content) → FileProcessResult {
    symbols, parent_refs, unresolved_imports, unresolved_calls, fqn
} → process.rs writes nodes + edges
```

Every language adapter goes through it. So making it uniform will not, on its own,
recover the missing links — because the three biggest gaps are not per-language
disagreements:

* **`imports` 136,484** — no adapter is ASKED to resolve them. The field is named
  `unresolved_imports`. Uniform adapters all emitting unresolved imports still
  yields 0%.
* **`references` 250,072** — written by the DOC processor, not a language adapter
  at all. No adapter uniformity touches it.
* **`calls` 112,841** — already uniform: 60–73% resolved in *every* language
  (§7.1). The residual is receiver-type inference, which no adapter contract fixes.

Uniformity is necessary — the `language` field being filled by one writer, nulled
by another and read by nobody (§2.3) is exactly the contract rot it would prevent —
but it is not sufficient. **The links are missing because nothing attempts
resolution, not because adapters disagree.**

### 11.2 Get-or-create-then-enrich — already implemented, and it is the bug

This is the current design, verbatim from `process.rs`:

> the target is get-or-created by FQN (a stub if its definition isn't indexed yet —
> **enriched later, keeping the same id**; a `lib_symbol` for an external crate)

And it produced the 108,174 misattributed stubs of §7.1. So the approach is not
untried; it is in place and failing.

**Why it fails is the useful part, and it is one thing: the stub's IDENTITY is
wrong.** A stub for `HashMap::get` is minted as

```
rust·senseid·api::handlers::observatory·HashMap·get
```

— the caller's namespace. When the real `HashMap::get` is later encountered its FQN
is `rust·std·std::collections·get`, which is a *different* key, so the enrichment
never matches. A second node is created and the stub is orphaned forever. That is
why 84,379 file-less `function` nodes have accumulated rather than converging.

So get-or-create-then-enrich is sound **only if the stub's identity is derivable
from the reference alone**, independent of who is referencing it. Get that right
and it is genuinely simpler than a connect pass — no second traversal, and the id
is stable so edges written early stay valid.

Which is precisely why imports come first (§8): an import statement names the
package explicitly, so it is the one source that can give a bare call name a
correct identity. `when` is unidentifiable alone; `when` in a file that imports
`org.mockito.Mockito.when` is not.

**The corollary is a rule worth stating: never mint a stub whose identity you had
to guess.** Leave the edge unresolved instead. An unresolved edge is a known gap; a
stub with a fabricated FQN is a wrong answer that also blocks its own repair.

### 11.3 Patterns are a different plane — and it already exists

Agreed, and the separation is already built. `inference.detected_patterns` has
**1,429 rows** and exactly the shape described:

```
name             family   instance_count   is_anti_pattern
rule-candidates      –               24    f
correction-prone     –               15    t
…
```

plus `instances jsonb`, `evidence jsonb`, `confidence`, `severity`, `lifecycle`.
"We have an adapter pattern and there are n adapters" is `name='adapter',
instance_count=n, instances=[…]` — a finding with evidence and a confidence, which
is not what an edge is. Edges are facts the parser can see; patterns are
conclusions drawn over many of them, and they change as the code changes without
any edge changing.

So the six empty kinds do **not** all belong to a pattern pass. They split three
ways:

| kind | where it belongs | why |
|---|---|---|
| `implements` | **the parser** — structural | `class X implements Y`, `impl Trait for Type` is in the syntax. Not a pattern; not inference |
| `extends` | **the parser**, fixed | currently mislabelled containment from the file node (§10.2). Real inheritance is equally parseable |
| `rationale_for` | **resolved `references`** | a rationale node → the code it justifies is a doc link, not a detected pattern |
| `traces_to` | **resolved `references`** | spec → code, same mechanism |
| `duplicates` | **not an edge** | computed at query time today, correctly. A finding, not a fact |
| `similar_to` | **not an edge** | embedding search at query time. Same |

So the pattern pass owns nothing currently in the enum — it owns
`detected_patterns`, which it already has. What the enum should lose is
`duplicates` and `similar_to`; what it should gain a writer for is `implements`;
and `rationale_for`/`traces_to` fall out of resolving `references`.

### 11.4 Revised order

1. **Resolve `imports`.** They name their packages, so they are the only source
   that can give call targets a correct identity.
2. **Fix stub identity in the call path**, using (1), and never mint a guessed
   identity — leave it unresolved.
3. **Resolve `references`** (doc→file, doc→symbol). This is 250,072 edges and it
   unlocks `rationale_for` and `traces_to` as a by-product.
4. **Emit `implements`** from the adapters, and fix `extends` to mean inheritance.
5. **Drop `duplicates` and `similar_to`** from the enum, or accept that they are
   permanently unwritable.

Steps 1–2 are the same work seen from two ends, which is why they are adjacent.

---

## 12. The root cause, in one type

The diagnosis — *the parsers parse imports but do not build a registry the ref
resolution can look up* — is exactly right, and it is visible in a single struct:

```rust
/// Per-file context a producer needs: the owning crate/package name (from the
/// nearest manifest, supplied by the Phase-3 processor) and this file's
/// crate-relative module path.
pub struct FileFqnContext {
    pub package: String,   // the file's OWN package
    pub module: String,    // the file's OWN module path
}
```

**There are no imports in it.** The imports ARE parsed — they become
`FileProcessResult.unresolved_imports: Vec<String>` — but that list never reaches
the FQN producer. So when a producer encounters a call to `when`, the only names in
scope are the file's own package and module. Minting
`rust·senseid·api::handlers::observatory·HashMap·get` is not a bug in the producer;
it is the only thing the producer *can* do with the context it is given.

That single omission explains everything measured in this document:

| symptom | measured | follows from |
|---|---|---|
| stubs in the caller's namespace | 108,174 edges, 84,379 nodes | no import registry → no correct identity available |
| local aliases as library packages | 2,204 `lib_symbol` (30%) | `is_lib` decided per-path without the import that would settle it |
| imports 0% resolved | 136,484 | the list is an output, never an input |
| calls resolve to real code only 18% | of 320,715 | the two above, combined |

### The fix is one field

```rust
pub struct FileFqnContext {
    pub package: String,
    pub module: String,
    /// Local name → where it came from, built from the imports this file already
    /// parses. `when` → `org.mockito.Mockito`, `HashMap` → `std::collections`,
    /// `worstReason` → `./metric-status-state`.
    pub imports: ImportRegistry,
}
```

Everything else falls out of it, with no new mechanism:

1. **Correct identity.** A bare name found in the registry mints the FQN of its
   ORIGIN, not the caller's. `when` → `lib·org.mockito·…·when`. Get-or-create then
   converges instead of orphaning, because the second encounter computes the same
   key.
2. **One owner for external-vs-local.** `is_lib` becomes
   `classify_import(origin).is_external()` — the function that already knows `$lib`
   and `@/…` are local (`804ef1fb`). The call path stops disagreeing with the
   import path because it stops deciding independently.
3. **Imports resolve as a by-product.** Building the registry IS resolving the
   imports; the edge target is whatever the registry entry points at.
4. **Honest gaps.** A name not in the registry stays unresolved. No guessed
   identity, per §11.2's rule.

So the import work and the call-path work are not two steps that happen to be
adjacent — they are one change seen from either end, and the registry is the thing
in the middle. That collapses steps 1 and 2 of §11.4 into a single slice.

### 12.1 Class diagrams fall out of the same parse

`interface` → `implements`, `class` → `extends` are both in the syntax, so once
`implements` has a writer and `extends` means inheritance (§11.3), a class diagram
is a filter over the graph rather than a separate extraction: nodes of kind
`class`/`interface`/`trait`, edges of kind `extends`/`implements`, containment from
`parent_id`. 22,669 such nodes exist already.

### 12.2 Reusing dbd's entity graphing for the DB — noted, later

dbd already renders entity diagrams from a schema. The same surface could render
`sensei`'s own tables, and possibly any dbd-managed project's. Deliberately parked:
it shares a renderer with §12.1 but nothing else, and none of the resolution work
depends on it.
