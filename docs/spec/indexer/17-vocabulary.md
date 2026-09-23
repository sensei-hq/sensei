---
name: Vocabulary
description: What a node, an edge, a via and every measured number actually mean
date: 2026-09-23
status: current
---

# Stage 17 — vocabulary

Every term and every number the indexer reports, defined once. Read this before
any figure in a commit message, a report or a review makes sense.

---

## §1 The two things in the graph

**A NODE is a declaration.** One row in `sensei.nodes`, one thing somebody wrote
down: a function, a struct, a field. A FILE is not a node — files live in
`sensei.files` and a node REFERENCES its file through `nodes.file_id`.

**An EDGE is a claim that one declaration reaches another.** One row in
`sensei.edges`. It has a source node, a target, a kind, and a `via` recording
WHY the indexer believes the two are connected.

Every node is named by an **fqn** — a fully qualified name, the join key for the
whole graph. `rust·senseid·indexer::repo·discover·item` is language, package,
module, name, reach. Two declarations minting one fqn is a defect (see `A7` in
§4), because one silently overwrites the other.

## §2 Node kinds

`SymbolKind`, in `indexer/facts.rs`:

| kind | what it is |
|---|---|
| `Function` | a free function |
| `Method` | a function owned by a type |
| `Class`, `Struct`, `Enum`, `Interface`, `Trait` | a type declaration |
| `EnumVariant` | one arm of an enum |
| `TypeAlias` | a name for another type |
| `Const`, `Static` | a named value |
| `Module` | a namespace, including the file's own |
| `Macro` | a macro definition |
| `Field` | a member of a struct or class |
| `Property` | a member reached like a field but backed by an accessor |

Each kind declares how it is REACHED — `ReachedBy::Call` or `ReachedBy::Import`
— which is what lets the coverage reports ask "is this symbol reached at all?"
without knowing the language.

## §3 Edge kinds

### Structural — `RelationKind`

| kind | claim |
|---|---|
| `Owns` | this type owns this member. Feeds `nodes.parent_id`. |
| `Contains` | this module holds this declaration directly. |
| `Extends` | every child IS a parent (`trait Sub: Super`, `class A extends B`). |
| `Implements` | this type provides this interface. |
| `TraitImpl` | rust's `impl Trait for Type`, kept apart from `Implements`. |
| `Mixin` | included rather than inherited. |
| `Decorates` | a decorator applied to a declaration. |
| `Variant` | this enum arm belongs to this enum. |

A declaration takes EXACTLY ONE of `Owns` or `Contains`, never both: both feed
`nodes.parent_id`, and a child with two parents has none.

### Behavioural — references

A reference is a USE: a call, a type mention, a name read. It carries the
`via` that placed it (§4) and a span, so one edge can record many use sites.

## §4 `via` — how a reference was placed

`Rung`, in `indexer/facts.rs`. The resolver is a LADDER: rungs are tried in
order of how strong their evidence is, and the FIRST that answers wins. `via`
records which one did, so every edge can be audited back to its reason.

Ordered strongest first:

| via | the evidence |
|---|---|
| `DeclaredHere` | this file declares the target itself. Nothing is inferred. |
| `ThroughAnImport` | an import in scope binds the head of the path. |
| `NamedByThisFile` | this file's own text names the target — it declares the type or imports it by a package-rooted path, so it has STATED where the type lives. |
| `DeclaredByItsType` | a type in this scan declares the target as its member, in ANOTHER file. Rust puts an `impl` anywhere, so a method and its type are often apart. |
| `ThroughAGlob` | a glob in scope covers the module the target sits in. The name was never written down, which makes this the weakest first-party rung. |
| `RootedInThisPackage` | a path rooted at this package, needing no import. |
| `InThePrelude` | the language puts it in scope everywhere. |
| `FullyQualifiedExternal` | a fully spelled path outside this scan. |

**An unresolved reference is not an error.** It carries a REASON, and the reason
histogram is measured (`A3`, §5). A reference nothing placed is an honest gap.

## §5 What the measured numbers COUNT

This is the part that has been reported carelessly. Every figure below has a
unit and a population, and both must be stated.

**The CORPUS is this repository's own source.** Not a user's codebase, not a
fixture: 395 rust files under `crates/`, and 1,720 TypeScript/JavaScript/Svelte
files under `app/`, `dojo/` and `website/`. A number moves when this repo's
code changes, which is why a ceiling that drifts with unrelated work is a
ceiling nobody trusts.

| figure | unit | population |
|---|---|---|
| "1,201 dangling" | EDGES | first-party edges whose target fqn no declaration mints |
| "254 distinct targets" | IDENTITIES | distinct fqns those edges point at — one identity can absorb hundreds of edges (`AppState·pg·field` alone takes 423) |
| "161 of 1,201" | EDGES | the subset that were calls to a derive-generated `default` |
| "49 types" | IDENTITIES | the distinct types those 161 edges named |
| "landed" | EDGES | the complement: the target IS declared somewhere in the scan |
| "8 collisions" | IDENTITIES | fqns minted by more than one declaration |
| "16,153 symbols" | NODES | declarations the scan produced over the corpus |

So **1,201 → 1,040 is edges, and 254 → 205 is identities**, and the two deltas
(161 and 49) are the same fix counted two ways. Quoting one unit and labelling
it the other is how "993" got published; it was an arithmetic slip in a shell
pipeline, and the lesson is that a number with no unit cannot be checked.

## §6 Ceilings, and the difference between a ceiling and a ratchet

A **RATCHET** is at the value the code currently achieves. It fails the moment
anything regresses. `A7`'s collision ratchet is at ZERO, reached by a
19 → 3 → 2 → 1 → 0 descent where each step repaired a defect rather than
tolerating it. That is a bound with no slack, and it is not a budget to spend.

A **CEILING** is set above the measurement, with headroom. `A4`'s rust ceiling
is 1,200 against a measurement of 1,040 — 160 of slack. It does not ratchet:
it silently absorbs new dangling edges until the slack runs out. That is
exactly what happened here — the derive gap accumulated inside the headroom for
months and only surfaced when the count hit 1,201.

**Say which one you mean.** They behave differently, and calling a ceiling a
ratchet implies a guarantee it does not give.
