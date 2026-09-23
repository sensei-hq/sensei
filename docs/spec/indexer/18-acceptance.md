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
| **A7** | **no two declarations mint one identity** | `persist::no_two_declarations_in_this_repos_rust_mint_one_identity` and `the_identities_this_repos_rust_cannot_keep_apart_are_a_known_and_bounded_set` | **RATCHET at ZERO** |
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

#### TypeScript's descent: 511 → 10

The ratchet test is `no_two_declarations_in_this_repos_**rust**_mint_one_identity`
— rust-scoped. The acceptance harness MEASURES every language, so TypeScript's
collisions were printed on every run and gated nothing. Measured 2026-09-23 at
**511**, and TypeScript cannot be flipped into `PRODUCTION_LANGUAGES` while a
declaration can silently overwrite another.

One rule, four shapes. A function body names what it declares — and the walk
only knew the shapes carrying an `id`:

| fix | shape that was unnamed | after |
|---|---:|---:|
| — | (baseline) | 511 |
| callback | a function passed as a call ARGUMENT — `test('…', () => {…})` | 38 |
| method body | a class method's body, via `container_at` | 14 |
| object property | a function under a key — `{ get: () => {…} }` | **10** |

The callback fix is the big one because `test(…)` sits at a file's TOP LEVEL, so
the naming chain was EMPTY and every callback in a spec shared one scope. In Rust
the same gap is nearly invisible: a closure sits inside a named `fn` whose segment
is already on the chain.

`container_at` is ported from the rust walk verbatim in intent — the `fn_scope`
DEPTH at which the container was established, which is what tells "the class this
body sits in" from "a class declared BY this body". A bare `!fn_scope.is_empty()`
would file a class declared inside a function as a local and lose its fields.

**The residue is one shape: block scoping.** All 10 are two `const` in different
BLOCK scopes of one function — `if`/`else` arms, two `for` loops — plus one type
declared twice in a file. They are genuinely distinct declarations in JavaScript,
and naming them apart needs a block-level segment. Rust has the same property and
reads zero only because its corpus does not hit it.

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
