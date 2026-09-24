---
name: Acceptance criteria
description: What A1 to A9 mean, which test enforces each, and what bound it holds
date: 2026-09-23
status: current
---

# Stage 18 — acceptance criteria

`A1`–`A9` are the indexer's acceptance criteria. They are DEFINED in
`docs/design/indexer.md` §6 (lines 761–805); this page is the working index —
what each one means in a sentence, which test enforces it, and what bound that
test holds today.

Quote the criterion, not the letter. "A4" tells a reader nothing; "every edge's
target is declared somewhere" tells them what broke.

---

## §1 The index

| | criterion | enforced by | bound today |
|---|---|---|---|
| **A1** | import-mediated references resolve at ≥99.9% | `acceptance::an_import_named_target_resolves` | ceiling: rust 20,000 / ts 8,000 unresolved |
| **A2** | zero references dropped — the count out equals the count in | `acceptance` counting pass | exact |
| **A3** | every unresolved reference carries a REASON, and the histogram accounts for all of them | `acceptance::every_miss_is_accounted_for_in_every_language` | exact |
| **A4** | after a completed scan, every edge whose target is not a declaration falls into one of three NAMED populations, each counted | `acceptance::every_first_party_edge_names_a_declaration_this_scan_holds` | **ceiling**: rust 1,200 / ts 700 dangling EDGES |
| **A5** | fields, properties and enum variants exist for every type that declares them | `acceptance::every_type_owned_declaration_says_which_type_owns_it` | exact |
| **A6** | re-indexing in a different file order produces an identical graph | order-independence tests over the corpus | exact |
| **A7** | **no two declarations mint one identity** | `persist::no_two_declarations_in_this_repos_rust_mint_one_identity`, `…_typescript_mint_one_identity`, and `the_identities_this_repos_rust_cannot_keep_apart_are_a_known_and_bounded_set`; per-language `#[ignore]`d corpus gates in `lang::{java,python,csharp,kotlin,php}::tests` | **RATCHET at ZERO**, rust and typescript; at the measured value for the five languages this repository holds none of |
| **A8** | re-indexing a file removes what it stopped claiming and nothing else | `persist` re-index tests (R10.6's four clauses) | exact |
| **A9** | an unparseable file is ACTIONABLE and REACHABLE — the parser's verbatim message with line and column | `files.skip_detail`, `acceptance` | exact |

## §2 The two that carry numbers

Everything else is exact — it passes or it does not. Two hold a count, and they
are NOT the same kind of bound (see `17-vocabulary.md` §6).

### A7 — the collision ratchet, at zero

The strongest guarantee here. Every fqn the walk produces is minted by exactly
ONE declaration; a violation names both sides and fails, rather than letting one
silently overwrite the other.

It reached zero by repair, not by tolerance — the assertion's own message
records the descent: **19 → 3 → 2 → 1 → 0**, each step a defect fixed.

> 19 before a local was named under its enclosing function, 3 before that rule
> reached a METHOD body, 2 before a `cfg`-gated declaration became a callable
> plus its arms, and 1 before an anonymous `const _` stopped being a symbol it
> never was.

**Zero is not a budget.** A change that adds a collision is a change that loses
a row, and the fix is the identity rule — not the bound. Adding derived members
for `Debug` tripped it at 8 and the restriction, not the ceiling, was the
answer.

#### TypeScript's descent: 511 → 0

The ratchet test is `no_two_declarations_in_this_repos_**rust**_mint_one_identity`
— rust-scoped. The acceptance harness MEASURES every language, so TypeScript's
collisions were printed on every run and gated nothing. Measured 2026-09-23 at
**511**, and TypeScript cannot be flipped into `PRODUCTION_LANGUAGES` while a
declaration can silently overwrite another.

One rule, four shapes. A function body names what it declares — and the walk
only knew the shapes carrying an `id`:

| fix | shape that was unnamed | after |
|---|---|---:|
| — | (baseline) | 511 |
| callback | a function passed as a call ARGUMENT — `test('…', () => {…})` | 38 |
| method body | a class method's body, via `container_at` | 14 |
| object property | a function under a key — `{ get: () => {…} }` | 10 |
| **local values are not nodes** | a `const`/`let` in a body was a row nothing read | 1 |
| type scoping | `module_of` now reads `module_here`, so a type in a body is scoped like an item | **0** |

The callback fix is the big one because `test(…)` sits at a file's TOP LEVEL, so
the naming chain was EMPTY and every callback in a spec shared one scope. In Rust
the same gap is nearly invisible: a closure sits inside a named `fn` whose segment
is already on the chain.

`container_at` is ported from the rust walk verbatim in intent — the `fn_scope`
DEPTH at which the container was established, which is what tells "the class this
body sits in" from "a class declared BY this body". A bare `!fn_scope.is_empty()`
would file a class declared inside a function as a local and lose its fields.

**The last 10 were not a naming problem, they were a scope problem.** JavaScript
lets sibling BLOCKS redeclare a name, so two `const result` in an `if`/`else` are
two declarations that only a block-level segment could tell apart — and such a
segment rewrites identities whenever a line moves inside a body.

The answer was to stop declaring them. **A local value is not a node**: the graph
answers "what does this function call", and `barrier.rs::a_node` — "something a
reader navigates to" — already excluded its kind from every coverage measurement.
It was a row nothing read. A local FUNCTION keeps its node (a call needs a target),
and a module-level `const` keeps its node (it is importable). The TYPE BINDING is
untouched: `flow.bind` is keyed by name and is what types `api.getLogs()`.

Measured cost, the whole of it:

| | before | after |
|---|---:|---:|
| declarations | 28,328 | 23,254 |
| ts references resolved | 26,347 | 26,125 |
| ts references unresolved | 33,048 | 33,270 |
| **ts barrier nodes** | **2,186** | **2,186** |

222 references moved from resolved to unresolved — exactly, so nothing was dropped
— and every one of them named a local. The barrier numbers are UNCHANGED to the
digit (2,186 / 1,454 / 935 / 767 / 422), which is the evidence that what was
removed was never reader-facing.

`no_two_declarations_in_this_repos_typescript_mint_one_identity` now holds the
bound. It did not exist before, which is why 511 could accumulate: the ratchet
beside it is rust-scoped, and the acceptance harness measures every language while
gating none.

#### Languages this repository holds none of

Rust and TypeScript are measured by acceptance because this repository IS rust
and typescript. Java, Python, C#, Kotlin and PHP are not here at all, so the
denominator would be empty and a gate over it would pass by vacuity.

Each therefore carries its own `#[ignore]`d gate in its adapter module, pointed
at an external checkout by `SENSEI_CORPUS`. They share four rules, each of which
was learned by getting it wrong first:

1. **Partition by repository.** An identity is scoped to a folder because the
   scan indexes per repo. Pooling asks a question production never asks — on
   Java's corpus it read 16,561 where the real figure was 409, purely because
   one repo vendored a copy of another.
2. **Decompose before concluding**, and print WHICH files rather than six
   capped examples. A capped sample is how "it is all one shape" gets believed
   without being true.
3. **A copy is identical CONTENT, not an identical filename.** PHP's run had 287
   collisions under a bucket labelled "same filename — CORRECT"; asking the
   bytes moved 216 of them out of it.
4. **The bound sits AT the measurement**, never above it. A ceiling with slack
   absorbs the next defect silently, which this repository has already paid for
   once (A4's rust ceiling).

| language | corpus | declarations | colliding | what the residue is |
|---|---|---:|---:|---|
| java | Dayamed, 5,088 files | — | 14 | overload sets, after the callable-plus-arm split |
| python | — | — | 0 | — |
| csharp | Ethico, 8,647 files | 137,859 | 392 | file copies, partial types, one duplicated class, and a vendored Syncfusion tree tree-sitter cannot recover from |
| php | 3 repos, 2,263 files | 72,420 | 312 | two checked-in copies of one protoc-generated tree (204), and CakePHP 2.x global-namespace reuse (108) |
| c | pljava, 129 files | 1,906 | 11 | `JNIEXPORT x JNICALL` read as a declaration of `x` — a macro a walk without a preprocessor cannot expand |
| kotlin | 245 files | 3,038 | 39 | anonymous objects, Android product flavours |

**C's `one file` bucket is ZERO too**, on both corpora of real C, and reaching
it took five fixes that are each a fact about the language rather than a patch:

| | | |
|---:|---|---:|
| 32 → 22 | tags are their own namespace (C 6.2.3), so `struct node` and a typedef `node` are two names | −14, all the `typedef struct X { .. } X;` idiom |
| 22 → 18 | a conditionally redefined macro is a callable plus one arm per branch — the split rust uses for `cfg` | |
| 18 → 12 | an anonymous aggregate is named by what BINDS it — the rule TypeScript needed for a callback | |
| 12 → 11 | a definition beats a declaration, however the forward one is spelled | |
| 11 → 11 | a tentative definition (C 6.9.2) is one object | |

**PHP's `one file` bucket is ZERO** — the only one of the five where it is. That
is the bucket the walk alone controls, and PHP gets it for free: the language
forbids redeclaration outright, so the overload split Java, C# and Kotlin each
needed has no shape to represent here. Its 312 are all CROSS-FILE, and both
groups are the source declining to discriminate rather than the walk failing to:
a file duplicated verbatim, or two classes of one name in the global namespace
that PHP itself would refuse to load together.

The global-namespace ones are deliberately NOT patched. The directory would tell
them apart, but the ladder has no directory — every cross-file reference to a
global-namespace class mints `<package>·<name>`, so moving the declaration side
to a path-derived identity trades these collisions for thousands of dangling
edges, which R4 ranks worse.

### A4 — the dangling-edge ceiling, with slack

"Dangling" means the resolver placed a reference onto an fqn, and then no
declaration anywhere mints that fqn. The edge points at nothing.

| | rust | typescript |
|---|---|---|
| ceiling | 1,200 | 700 |
| measured (2026-09-23) | 1,040 | 567 |
| slack | 160 | 133 |

**This is a ceiling, not a ratchet**, and the distinction has already cost
something. It was created together with the test in `67fff329`, set ~200 above
what rust measured that day — so it has never been a bar rust met and regressed
from. It absorbed new dangling edges silently until the slack ran out at 1,201,
which is the only reason the derive gap surfaced at all.

What the rust residue is, decomposed rather than absorbed:

- **RELATION parents** — an `Owns` relation from an `impl PgStore` block mints
  its parent at the module the impl sits in, where the struct is declared one
  module out. The member's own identity is right; the relation's parent is not.
- **imports and rooted paths** — the remainder.

And typescript's 567: **523 in one re-export barrel** (`e2e/fixtures` passes
Playwright's `test` and `expect` straight through) plus 38 in
`lib/components/kit/index`. Both are `export { x } from './y'`, where the target
IS the module named and the declaration is one module further on. Following a
barrel needs a cross-file re-export table.

## §3 Where the measurements live

The harnesses are `#[ignore]`d — they are on-demand measurements, not gates, so
a normal `cargo test` does not pay for a full-corpus walk:

```
cargo test -p senseid --bin senseid indexer::acceptance -- --ignored --nocapture
```

`indexer/barrier.rs` holds the two coverage barriers over any corpus, and
`SENSEI_CORPUS` points the web reader at a codebase nobody here wrote — our own
front ends were written alongside this indexer, so agreement with them is
weaker evidence than it looks.

## §4 A known mislabel

`acceptance.rs` prints `## A8: first-party edges, and whether the target is
declared`. That heading is WRONG: the dangling-edge criterion is **A4**. A8 is
re-index removal. The heading has been corrected in the code; it is recorded
here because the wrong letter reached commit messages and review notes before
anyone checked it against the design doc.
