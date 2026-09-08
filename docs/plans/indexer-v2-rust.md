# Build plan — indexer v2, rust

Spec: `docs/design/indexer-v2.md`. Requirements are cited as R1..R8, acceptance
as A1..A6. This document is only the sequence: what to build, how to prove it,
what goes wrong.

Rules for every step: failing test first, run it, watch it fail. Gate before
moving on — `cargo fmt --all`, `cargo clippy --workspace --all-targets -D
warnings`, `cargo test --workspace`. No step is done while the next one is
started.

---

## Step 1 — Fact types

**Build** `crates/senseid/src/indexer/facts.rs`. Types only, zero logic:
`FileFacts`, `Symbol`, `SymbolKind`, `Reference`, `RefKind`, `Resolution`,
`Reason`, `Evidence`, `Relation`, `Import`, `Fqn`, `Span`.

**Verify**
- A test constructs one value of every enum variant.
- A test asserts `Resolution` has exactly two variants and neither is empty.
- `rg 'Option<Fqn>' crates/senseid/src/indexer/` returns nothing.

**Watch out**
- `Reason` must be an enum, not a `String`. A string reason cannot be
  exhaustively matched and the histogram silently grows junk categories.
- No `Option` may stand for "unresolved" anywhere in these types (R2).
- Do not add convenience constructors that default a field. A defaulted field is
  how fabricated data enters (R4).

---

## Step 2 — FQN construction

**Build** `crates/senseid/src/indexer/fqn.rs`. The grammar from spec §2, one
builder per form.

**Verify**
- For a table of `(package, module, type, member)` inputs, assert the
  DEFINITION-side builder and the REFERENCE-side builder produce byte-identical
  strings. This is the merge contract; if it fails nothing else matters.
- Assert empty segments are dropped identically in both.
- Round-trip: parse a built fqn back into parts.

**Watch out**
- Two call sites building an fqn by hand instead of calling the builder is how
  the sides drift apart. There must be no `format!("{}·{}", ..)` outside this
  file — enforce with a test that greps the tree.

---

## Step 3 — Rust walk: declarations

**Build** `crates/senseid/src/indexer/lang/rust.rs`, declarations only. Emit a
`Symbol` for every function, method, struct, enum, trait, type alias, const,
module, **field, tuple-struct field, enum variant**. Parameters are typed props
on their function (D2). Record the declared type for fields and returns (R3).

**Verify**
- Fixture file with one of each. Assert symbol count equals a hand-written
  expected count, and assert each expected fqn is present.
- Independent counter: a second tiny walk that counts declaration nodes by
  tree-sitter kind. Symbol count must equal it. Not a self-check by the
  producer.
- Assert a field and a method with the SAME name on the same type produce
  distinct fqns and distinct identities.

**Watch out**
- `nodes_unique_identity` is `(folder_id, file_path, kind, name, parent_id,
  line_start)`. A field and a method of the same name at the same line collide
  unless `kind` separates them. Test it explicitly.
- A tuple-struct field has no name. Decide its identity (index) and write it
  down in the test.
- Do not emit parameters as symbols (D2).

---

## Step 4 — Rust walk: references, all of them

**Build** reference emission. Every call, member access, path use and
construction yields exactly one `Reference`. Resolution may be `Unresolved` —
omission is not permitted (R2).

**Verify — this is the load-bearing check (A2)**
- Write an independent counter that walks the same tree and counts use sites by
  tree-sitter kind, with no knowledge of the resolver. Assert
  `references.len() == that count`, over this repo's own rust sources, not a
  fixture.
- Deliberately malformed source (`x.();`, `().0();`, unterminated) must not
  panic and must not silently drop.

**Watch out**
- A catch-all match arm returning early is the exact defect this rewrite exists
  to remove. There must be no arm that yields nothing; the fallback yields
  `Unresolved { reason: UnhandledForm, .. }` carrying the node kind so the
  histogram names it.
- Macro invocations are a distinct tree-sitter kind from calls. Decide whether
  they are references and record the decision; do not let them fall through
  unnoticed.

---

## Step 5 — Resolution ladder and reason codes

**Build** `crates/senseid/src/indexer/resolve.rs`. Shared across languages (R7).
Local declarations, then imports, then external classification. Everything else
`Unresolved` with a specific reason.

**Verify**
- One test per `Reason` variant, asserting that exact variant is produced.
- Histogram test over this repo's rust: every unresolved reference has a reason
  and the reasons sum to 100% (A3).
- Order independence (A6/R6): resolve the same file with an empty "already
  scanned" set and with a full one — identical output.

**Watch out**
- Never infer externality from absence. A symbol missing from the current file
  set may simply not be scanned yet; that makes the graph depend on scan order.
  Externality comes from the import (spec §2).
- The denylist (plumbing like `clone`, `unwrap`) is filtering, not failure. Give
  it its own reason so a reader can exclude it without losing genuine misses.

---

## Step 6 — Relations

**Build** `extends`, `implements`, impl blocks, trait impls, member ownership,
from the same walk (D5).

**Verify**
- Fixture with an inherent impl, a trait impl, and a generic impl. Assert the
  inherent impl produces NO relation and the trait impl produces one.
- Assert every relation's child and parent are resolvable or explicitly
  unresolved — never a bare name with no reason.

**Watch out**
- An inherent `impl Foo { }` is not inheritance. Emitting one is a false edge
  that pattern detection will read as real.

---

## Step 7 — Persistence

**Build** the emit path from `FileFacts` to the database (R3, D1). The wire
format carries everything the walk captured, including declared types and
unresolved reasons.

**Verify**
- Round-trip: index a fixture file, read the rows back, assert every field of
  every `Symbol` and `Reference` survived. Specifically assert a declared type
  reaches the database — that is the seam that lost data before.
- Assert an `Unresolved` reference produces a row, not an absence.

**Watch out**
- A positional-argument insert with no slot for a new field silently drops it.
  Use a struct, not positional args.
- A definition arriving after a reference must merge onto it, not create a
  second node (spec §2, the merge contract).

---

## Step 8 — Differential harness

**Build** a harness that runs v1 and v2 over the same corpus and diffs the facts.
Not a test — a tool that prints a report.

**Verify**
- Every difference is classified: improvement, regression, or explained. A
  regression blocks cutover.
- v2's resolved set must be a superset of v1's, or each exception justified in
  writing.

**Watch out**
- v1 drops references silently, so v2 will show far MORE references. That is the
  intended result, not a regression — compare resolved edges and node identity,
  not raw counts.

---

## Step 9 — Cutover, reindex, acceptance

**Build** the switch in the file processor for rust only.

**Verify** — run the acceptance list (A1..A6) against the live graph after a full
reindex. `sensei scan` will not pick up an indexer change; force it by resetting
`daemon.last_version` and restarting, then set it back.

**Watch out**
- Reindexing all 48,654 files takes ~20 minutes plus embedding backfill. A shell
  command may cap at 10 minutes — poll across several.
- Record the before numbers BEFORE deploying. They cannot be recovered after.

---

## Step 10 — Retire

Move the superseded rust module to `to_be_discarded/`. Only after step 9 passes.
Other languages follow the same ten steps, one at a time (D4).
