# ADR — language capability traits + one graph-fact shape

Status: **proposed** (your call). Arises from the import/reference resolution
work of 2026-09-02 (`cc534c63`, `d910c291`, `6bee2674`) and the `extends`
investigation that stopped before writing code.

---

## 1. The repo already does this — for manifests

`ManifestAdapter` (`adapters/manifest.rs:30`) is a capability trait with **ten**
implementations — cargo, npm, maven, gradle, go, pyproject, ruby, composer,
dotnet, swiftpm — each owning one ecosystem's parsing, discovered through
`accepts(filename)`.

So capability-per-ecosystem is an **established pattern in this codebase**, not a
new idea. This ADR proposes applying it to the layer that never got it.

| layer | shape today | verdict |
|---|---|---|
| manifests | `ManifestAdapter`, 10 impls | correct |
| parsing | `LanguageAdapter`, 12 impls | trait, but monolithic + mixed-concern |
| **resolution** (imports, libraries, inheritance, components) | **no trait** — free functions with per-language branches and hardcoded lists | the gap |

Every defect fixed on 2026-09-02 lived in that third row.

## 2. The schema already declares what the code cannot produce

Measured on the live graph (728,702 nodes) after a full reindex:

```
edge_kind declared: 11     actually used: 4
  calls 336,714 · references 241,515 · imports 162,691 · extends 7,901
  implements 0 · depends_on 0 · traces_to 0 · covers 0
  rationale_for 0 · duplicates 0 · similar_to 0

node_kind declares `component` and `hook`
  → 2 components, 1 hook   across Svelte AND Vue applications
```

`implements` has been a first-class edge kind since the schema was written and
has **never held a row**.

### The gap is PERSISTENCE, not extraction

An earlier draft of this ADR claimed nothing populates `IRClass.extends` /
`implements`. That was wrong, and the correction makes the case stronger. Three
adapters already extract real inheritance:

| adapter | what it extracts |
|---|---|
| `java.rs:266,269` | superclass **and** implemented interfaces |
| `python.rs:141,195` | base class |
| `rust_lang.rs:299,301,318` | trait impls |

And **nothing downstream of the IR reads them** — zero references to those fields
outside `ir.rs` and `languages/`. So the business layer already computes the
fact, there is no portable structure to carry it to the data layer, and it is
dropped on the floor. 0 `implements` edges from data that already exists.

That is the whole thesis of this ADR in one example.

### The concrete damage

All 7,901 `extends` edges have `source_kind = 'file'`. They are built from
`parent_refs`, where `sym.parent` is documented as *"Parent class/struct name for
methods (e.g. `Foo` for `Foo.bar`)"* — **containment, not inheritance**. The emit
site's own comment says `HAS_METHOD: type → method` while the edge kind says
`extends`.

So each edge asserts *"this file extends Foo"* when the fact is *"Foo has method
bar"* — and 626,441 of 728,702 nodes **already** carry that containment in
`parent_id`, which `graph_nodes.ddl` explicitly says makes a `contains` edge kind
unnecessary. `codebase.rs:55` feeds these into the graph view alongside `calls`
and `imports`, so the UI renders containment as inheritance.

## 3. Four output shapes, four hand-written persistence policies

The adapter layer returns four different shapes:

| shape | fields |
|---|---|
| `ParsedFile` | symbols, edges, imports |
| `IRParsedFile` | modules, classes, docs, is_test_file, file_hash |
| `FqnFileOutput` | defs, refs, package, module |
| `FileProcessResult` | the processor's own flattening: `unresolved_imports`, `unresolved_calls`, `parent_refs`, `file_refs`, `fn_mentions` |

`process.rs` then persists each list with bespoke code, and **the persistence
policy was written separately for each concern**:

| concern | policy today |
|---|---|
| `calls` | get-or-create target by FQN |
| `imports` | lookup-first across candidates, create stub on total miss |
| `references` (file) | lookup-first, **never** create — a broken link is a fact |
| `references` (symbol) | lookup-first, resolve **only if unambiguous** |
| `extends` | none — raw insert with `target_id = None` |

Four policies, one function. I wrote two of them on 2026-09-02, a few hours
apart, and had to restate the same lookup-first rule both times. `extends` got no
policy at all, which is why it is 0% resolved.

**This is the strongest argument for a shared fact shape**: the rules that must
never vary — lookup before create, never fabricate a target, ambiguity is not an
answer — currently live in whichever branch a developer happened to be editing.

## 4. Proposal

### 4a. Capability traits, opt-in, reached through the adapter

```rust
pub trait LanguageAdapter: Send + Sync {
    // ...existing parse surface, unchanged...

    /// `None` means this language HAS NO inheritance concept (C, SQL) — a FACT,
    /// not an unimplemented stub. That distinction is the whole point: today
    /// `fqn_output()`'s `None` default cannot tell "no such concept" from
    /// "nobody wrote it yet", and kotlin/swift/c silently produce no FQN nodes
    /// with nothing flagging it.
    fn inheritance(&self)  -> Option<&dyn Inheritance>  { None }
    fn components(&self)   -> Option<&dyn Components>   { None }
    fn imports(&self)      -> Option<&dyn ImportPaths>  { None }
    fn libraries(&self)    -> Option<&dyn LibraryOrigin> { None }
}
```

One registry (`adapter_for_ext`) stays. Absence becomes **queryable**: "C has no
inheritance" is something a test can assert, instead of a silent no-op default.

### 4b. `Inheritance` fixes a modelling bug, not just a location

```rust
pub struct TypeRelation { pub child: String, pub parent: String, pub kind: RelationKind }
pub enum RelationKind { Extends, Implements, TraitImpl, Mixin }
```

`IRClass.extends: Option<String>` holds **one** parent. Java has one superclass
*plus N interfaces*; Rust has no inheritance but N `impl Trait for`; Python has N
bases. That field could not faithfully represent any real language — plausibly
why it was never populated. `RelationKind` maps 1:1 onto the already-declared
`extends` / `implements` edge kinds.

### 4c. Three layers, one portable contract between them

The shape this is really reaching for:

```
BUSINESS LAYER          language adapters — traits + options
                        knows syntax; knows nothing about SQL
        │
        │  emits GraphFacts  ← the portable contract
        ▼
DATA LAYER              backing · retrieval · merge
                        knows upsert/lookup/conflict; knows nothing about syntax
```

`process.rs` violates this today: it calls `ctx.pg().insert_edge(...)` **inline,
inside the per-concern emit loops**. Business logic reaches straight into
persistence, which is precisely why each concern grew its own policy (§3) — there
was no boundary at which a shared rule could live.

The `merge` half matters as much as the write. The data layer already owns a real
merge rule — `upsert_node_by_fqn` returns an existing row, creates a stub on a
miss, and ENRICHES that same row in place when the definition later arrives,
keeping its id. That rule is what makes emit-time resolution order-independent.
It belongs to the data layer and should be reachable by every capability without
each one re-deriving it.

### 4d. One shared fact shape, one persister

Every capability emits the same thing:

```rust
pub struct GraphFacts { pub nodes: Vec<NodeFact>, pub edges: Vec<EdgeFact> }

pub struct EdgeFact {
    pub source: SymbolRef,
    pub target: TargetRef,
    pub kind:   EdgeKind,
    /// What the persister may do when `target` does not resolve. Stated by the
    /// capability, ENFORCED centrally — so "never fabricate" cannot be forgotten
    /// in one branch while being honoured in another.
    pub on_miss: OnMiss,
}

pub enum TargetRef { Fqn(String), Path(String), Name(String), Candidates(Vec<String>) }
pub enum OnMiss    { LeaveUnresolved, CreateStub, RequireUnambiguous }
```

The persister owns lookup-first, stub creation, and the never-fabricate rule
**once**. A capability declares intent; it does not hand-roll SQL. The five
policies in §3 collapse into three `OnMiss` values that a reviewer can see at a
glance.

## 5. Sequencing (forward-only, each independently shippable)

1. **This ADR.** Cheapest point to disagree.
2. **`Inheritance`** — SMALLER than it looks: java, python and rust already
   extract it (§2), so the first slice is persistence, not parsing. Emit real
   `extends` / `implements` edges from `IRClass`, and retire the 7,901
   mislabelled containment edges in the same slice (`parent_id` already carries
   that fact). Typescript/kotlin/swift extraction follows behind the trait.
3. **`GraphFacts` + persister** — migrate `Inheritance` onto it first, since it
   is the newest and smallest producer, then move the four existing policies
   (§3) onto `OnMiss` one at a time.
4. **`ImportPaths`** — behaviour-preserving, so the existing suite is the whole
   gate. Retires `import_target`'s rust special-case and the shadow-classifier
   class of bug (`typescript.rs` had a second `classify_import` that was wrong
   about `@/` for a day while the owner was right).
5. **`LibraryOrigin`** — retires `libraries.rs:62-119`'s inline skip-list (a
   third drifted copy of `classify_import`), then unblocks externals →
   `lib_symbol`, 136,642 edges. A trait makes that work *less* risky: each
   language declares what "external" means for it (Java can state that it cannot
   distinguish its own packages by string alone), while lookup-first stays
   central.
6. **`Components`** — smallest, genuinely new; 2 components + 1 hook today.

## 6. Risks and open questions

- **Scope.** ~10 adapter files per capability. Mitigation: one capability per
  slice, 2,699-test suite green between each. Not a big-bang.
- **`Box<dyn>` ergonomics.** `adapter_for_ext` returns `Box<dyn LanguageAdapter>`;
  the accessors return `Option<&dyn Capability>` borrowed from `&self`, which
  works for the current zero-sized adapters but needs checking if any adapter
  ever holds state.
- **Do the 7 unused edge kinds all deserve producers?** `traces_to`,
  `rationale_for`, `covers`, `similar_to`, `duplicates`, `depends_on` are out of
  scope here. This ADR only claims `implements` has an obvious owner.
- **Retiring the `extends` edges removes data a UI currently draws.**
  `codebase.rs:55` would switch to `parent_id` for containment. Worth confirming
  the graph view still renders before/after.
- **Open:** should `RelationKind::TraitImpl` be its own edge kind, or
  `implements` with a discriminant in `props`? Rust trait impls are not Java
  interface implementation, but the query "what implements X" wants both.
