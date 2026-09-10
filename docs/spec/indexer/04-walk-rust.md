# Stage 4 — the Rust walk: one parse, all the facts

Whole-system spec: `docs/design/indexer-v2.md` §2 and §2.1 (identity and
reach), §3 (the fact types), R1, R2, R3, R8, R9, D2, D5, D7. Depends on
stage 3.

## 1. Purpose

Read one Rust file ONCE and emit everything the graph will ever need from it:
declarations, every reference, the OO relations, the imports. One parse, not
three — a second pass for OO patterns is exactly what this rewrite exists to
remove.

Serves G1 (an agent needs callees and types) and G2 (fields and variants are
what make "what shape is this data" answerable; today there are **0** field and
enum-variant nodes in any language).

## 2. Inputs and outputs

    read(path: &Path, src: &str, pkg: &PackageContext) -> Result<FileFacts, ReadError>   PURE

**This stage is entirely pure and touches no database.** That is testable and
must be tested: `read` takes source TEXT, not a path to open, so its whole
suite runs on string literals.

    struct FileFacts { symbols, references, relations, imports, file_identity }
    enum   Resolution { Resolved(Fqn), Unresolved { reason: Reason, evidence: Evidence } }

`FileFacts` must have **no production constructor** other than a language
module's `read` (R10.3) — that is what makes it impossible for a failed parse
to reach reconcile as an empty fact set. Enforce with a guard test over the v2
sources.

## 3. Requirements

### Identity

- **S1** (§2). The fqn grammar is
  `<lang>·<package>·<module>·<Type>·<member>·<reach>`, with a 7-segment
  trait-impl form `<lang>·<pkg>·<mod>·<Type>·<Trait>·<member>·<reach>` (§5).
  ONE builder, in one file. No `format!("{}·{}", …)` anywhere else — enforce by
  a test that greps the tree.
- **S2** (§2.1, D7). The trailing segment is the REACH, not the namespace:
  `item | field | macro | mod`. Rust's type and value namespaces collapse into
  `item` because a path leaf does not state which it is, and one declaration
  may occupy both.
- **S3** (§2.1). A tuple-struct field's `member` segment is its DECIMAL INDEX,
  reach `field`. `Wrapper(String)` declares `…·Wrapper·0·field`; `w.0` mints
  the same. Settled in the grammar, not in a test.
- **S4** (§2.1). Macro invocations ARE references, with reach `macro`. They are
  a distinct tree-sitter kind from calls; assert exactly one `Reference` per
  `name!` site. This is decided — do not re-open it.

### Declarations

- **S5** (R3, A5). Emit a `Symbol` for every function, method, struct, enum,
  trait, type alias, const, static, module, **field, tuple-struct field, and
  enum variant**. Record the declared type for fields and for returns.
- **S6** (D2). Parameters are typed PROPS on their function, never nodes.

### References

- **S7** (R2, A2). Every call, member access, path use and construction yields
  exactly ONE `Reference`. Omission is not permitted. `Resolution` is a total
  enum with no representation for "nothing", so a reference cannot be silently
  dropped — that is the type doing the work, per R9.
- **S8** (R2). There must be NO match arm that yields nothing. The fallback
  yields `Unresolved { reason: UnhandledForm, .. }` carrying the tree-sitter
  node kind, so the histogram names what was missed.

### Relations

- **S9** (D5, R8). Emit `extends`, `implements`, impl blocks, trait impls and
  member ownership FROM THE SAME WALK. An inherent `impl Foo { }` is NOT
  inheritance and must produce no relation — emitting one is a false edge that
  pattern detection reads as real.
- **S10** (R8, D6). Before this stage is done, take R8's seven patterns and
  assert the union of facts they need is actually emitted, and that each
  pattern is derivable from NODES AND EDGES ALONE with no return to source.
  This is D6's gate and it is the requirement most likely to be skipped.

## 4. Failure modes

| input | this stage does |
|---|---|
| the file will not read from disk | `Err(ReadError::Io)`. Never an empty `FileFacts`. |
| the grammar is unavailable | `Err(GrammarUnavailable)`. |
| tree-sitter refuses | `Err(NotParsed)`. |
| the file identity cannot be minted | `Err(NoFileIdentity)`. A file with no module path is not a file with an empty one. |
| a DAMAGED file that still parses | `Ok` with fewer facts. **This is undetectable and is not this stage's problem** — R10.3 measured it: truncating 294 files produced 0 errors, and `has_error()` is wrong in both directions. Do not add a gate. Stage 7 owns the consequences. |
| malformed source (`x.();`, `().0();`, unterminated) | must not panic and must not drop. Each yields an `Unresolved` with a reason. |
| an unsupported syntactic form | `UnhandledForm` with the node kind (S8). |

**Nothing may turn an `Err` into an empty fact set.** `.unwrap_or_default()` on
a `Result<FileFacts, _>` is the exact shape the DRY/no-fabrication rule forbids,
and a guard test must assert it appears nowhere in the v2 sources.

## 5. Verification

| test | mutation that must break it |
|---|---|
| **the merge contract** — for a table of `(lang, package, module, type, trait, member, reach)` inputs, the DEFINITION-side and REFERENCE-side builders produce byte-identical strings | change either builder's separator, or its handling of an empty segment |
| the same table includes a unit enum variant, a unit struct as a value, a field-vs-method pair, a `mod`-as-prefix path, and a tuple-struct index | drop `reach` from the inputs — this is the segment that was MEASURED to break, 94 references minting `ty` against declarations minting `val` |
| a field and a same-named method on one type mint DISTINCT fqns and distinct identities | collapse `field` into `item` |
| `x.0` and `Wrapper(String)`'s field mint the same string | change S3's index rule on one side |
| one `Reference` per `name!` site | treat macro invocations as non-references |
| reference count equals an INDEPENDENT counter's walk over this repo's real Rust, not a fixture | add any early-return arm |
| an inherent impl produces NO relation; a trait impl produces one | emit a relation for both |
| every relation's parent and child are resolved or explicitly unresolved — never a bare name | allow a bare-name relation |
| each of R8's seven patterns is derivable from the emitted facts alone | remove one required fact from the walk |
| `read` compiles and runs with no database in scope | give it a `&PgPool` |
| no `format!` builds an fqn outside `fqn.rs` | build one by hand in `rust.rs` |
| `FileFacts` has no public constructor outside `read` | add `FileFacts::default()` |

The independent counter is the load-bearing check (A2) and it must run over the
REAL corpus. A fixture proves the counter agrees with the walk on cases the
author thought of; the corpus is where the unthought-of forms live.

## 6. Watch out

**`nodes_unique_identity` is `(folder_id, file_path, kind, name, parent_id,
line_start)`** — becoming `file_id` after stage 0. A field and a method of the
same name at the same line collide unless `kind` separates them. Test it
explicitly; this collision already happened once and was caught by the gate
before it spread.

**Do not infer a kind from a `RefKind`.** "`Constructs`, therefore a type" is
exactly the inference that produced the enum-variant break. A stub whose kind
is not yet known says so, using the value stage 0 added to `node_kind` — the
`parameter` placeholder is WITHDRAWN and must not reappear.

**The denylist is filtering, not failure.** Plumbing like `clone` and `unwrap`
gets its OWN reason code, so a reader can exclude it without losing genuine
misses in the same bucket.

**Deferred capabilities still capture their evidence** (R11): a binding's
provenance for later type inference, a receiver's stated bound and every impl
for later trait dispatch, generic parameters and bounds for blanket impls, the
invocation site and resolved path for macro expansion, and a DISTINCT reason
for "no annotation exists" versus "an annotation exists and we did not read
it". It is legitimate to defer work; it is not legitimate to discard evidence,
because re-parsing 48,646 files to recover a field we already had is the
expensive version.

## 7. Definition of done

- `read` is pure, database-free, and its suite runs on string literals.
- The merge-contract table covers all seven segments including `reach`, and
  passes.
- Field and enum-variant symbols are non-zero over the real corpus.
- The independent reference counter agrees over this repo's real Rust.
- R8's seven patterns are each shown derivable from the emitted facts (S10).
- No `Option<Fqn>` and no early-return arm exists in the v2 sources.
