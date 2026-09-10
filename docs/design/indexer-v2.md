# Indexer v2 — specification

Status: SPECIFICATION. Nothing here is implemented.

This states what v2 must do. It is written to be buildable and testable without
reference to any prior implementation. Why a rewrite was chosen, and the measured
defects that led to it, are in `docs/analysis/graph-coverage-gaps.md` and
`docs/analysis/graph-recovery-plan.md`. They are not repeated here.

## 1. Purpose

**G1.** An LLM agent can navigate an unfamiliar repository. It can ask "where is
X defined", "who calls X", "what does X depend on", "what shape is this data" —
and receive either a correct answer or an explicitly incomplete one with a
reason. Never a plausible guess.

**G2.** A person can see a repository's structure and judge where its risk is
concentrated, without reading the code.

OO patterns serve BOTH goals. For G2 they are how a person recognises the shape
of an unfamiliar codebase. For G1 they stop an agent inventing a second way to do
something the codebase already has a convention for — an agent that can see
"these six types are adapters" writes the seventh adapter, not a novel design.

Anything that serves neither is out of scope.

## 2. Definitions

**Indexing** is scanning SOURCE FILES to extract nodes and edges. An edge may
point outside the local code — that is where libraries and language internals
live. Dependency sources are never parsed.

**FQN** — the identity of a symbol,
`<lang>·<package>·<module>·<Type>·<member>·<reach>`. It is a LOOKUP KEY, minted
independently by a reference and by a definition. If the two sides mint
different strings for the same symbol they never merge, so every rule that
builds one is shared, never per-language.

**A file's own identity** is the identity of the module it declares, minted
exactly as the `mod x;` that names it mints it — otherwise a file and its
declaration are two nodes. A CRATE ROOT is the one file no `mod` names: its
module path is empty, and a package may have several (`crates/mcp` has `lib.rs`
beside `main.rs`; a `build.rs` is a third). Reducing them all to one name filed
every import and every file-scope use site of all of them under one identity, so
a crate root is identified by its own file stem under the module segment
`crate` — a reserved word, so this can still never collide with a declaration.
The file's path is therefore a fact of the walk, carried on `FileFacts`, and not
an argument supplied again at write time: two derivations of one identity are
two identities waiting to disagree.

**Local** — declared in a scanned source file. **External** — everything else.
Externality is decided by the import that brought a name into scope, never by
whether a symbol happens to be absent (absence is scan-order dependent).

### 2.1 The trailing segment is the REACH, not the namespace

A key needs a trailing discriminator because a language may let two declarations
share one name in one scope: this repo has a `WatcherHealth::healthy` field
alongside a `healthy()` getter, and `sensei-bootstrap` has `pub mod config;`
alongside `pub fn config()`. Without a discriminator those mint one string, one
declaration overwrites the other, and references to either land on the wrong
one — a wrong edge (R4).

The first version of this section made that discriminator the Rust NAMESPACE
(`ty`, `val`, `field`, `macro`) and justified it with "a use site can tell
namespaces apart from syntax alone (`x.foo` vs `x.foo()`)". **For a path that
claim is false, and it is false in two independent ways.**

1. `Foo::Bar` is spelled identically whether `Bar` is an enum variant, an
   associated const, an associated type, or a type inside a module `Foo`. The
   use site has nothing to read.
2. Worse, and fatal to any repair on the reference side: one declaration can
   occupy BOTH namespaces at once. The Rust Reference files an enum variant and
   a unit/tuple struct under the TYPE namespace and their constructors under the
   VALUE namespace. There is therefore no single true namespace for the
   declaration side to mint either. Picking one there and picking the other at
   the use site is not a coding slip — with a namespace in the key it is the
   only available outcome.

Measured over this repo's Rust, at the point the defect was found: 17,874
references resolve to a first-party identity and 431 of them name no
declaration. **100 of those 431 (32 distinct identities, 0.56% of first-party
resolutions) differ from a real declaration only in the trailing segment** — 94
where the reference minted `ty` against a declaration minted `val` (enum
variants reached as `Enum::Variant`), 6 the other way round (unit structs used
as values: `Box::new(WindowsProvider)`). The remaining 331 are a different
defect and are not fixed here: 292 have the right name under the wrong module
(the split-impl anchoring recorded in the resolution tests, plus re-exports the
walk does not follow), 7 land on a head only a module declares, 32 name nothing
the package declares anywhere. The earlier attribution of all 431 to the
namespace break was wrong.

**The rule.** The trailing segment is the REACH: how a use site gets to the
name. There are four values, and each is decided by an observable that the
declaration side and the use site both have.

| reach | how a use site gets there | what declares it |
|---|---|---|
| `field` | `x.name` / `x.0`, a dot with no call; a struct-literal or struct-pattern field position | struct and enum-variant fields, named and positional |
| `macro` | `name!` | `macro_rules!` and proc macros |
| `mod` | never — a module appears in an identity as the `module` segment, not as a target | `mod` items, and the file that a `mod x;` names |
| `item` | anything else: a path segment `a::b`, a bare name, a dotted call `x.name()` | everything else — types, traits, type aliases, associated types, functions, methods, consts, statics, enum variants |

An external (`lib·<package>·<member>`) carries no reach: we index no
declarations for it, so there is nothing to collide, and it is reached through a
whole path (`sync::Mutex::new`) that has no single answer anyway.

The struct-literal and struct-pattern field positions in the `field` row are
stated for completeness — the walk emits no reference for either today. They are
in the table so that whoever adds them has the reach already decided, rather than
choosing one at that moment and rediscovering this section.

One consequence lands on the persistence path (R4). A stub minted for a proven
first-party target that has not been indexed yet used to read its node kind off
the trailing segment: `ty` became `type`, `val` became `function`. An `item`
states neither, and the reference's `RefKind` must not be used to fill the gap —
"`Constructs`, therefore a type" is exactly the inference that produced the
enum-variant break. A stub whose kind is not yet known says so; the real kind
arrives with the declaration.

**How it says so.** `sensei.node_kind` is `not null` and has no value meaning
"not yet known", so **stage 0 widens the enum to add one** (§7h item 4), before
any v2 code is written.

WITHDRAWN: the `parameter` placeholder. An earlier draft deferred the enum
widening to the end of the build and had `item` stubs carry `parameter` in the
meantime, on the reasoning that v2 never mints it for a declaration and no query
selects it. That reasoning was sound and the mechanism is still wrong — it is a
workaround for a constraint stage 0 removes, and it is already in committed code
(`indexer/persist.rs`) with a comment pointing at a build step that no longer
exists. Do not carry it forward. If the widened enum is not yet applied, stage 0
is not done, and no code depending on it should be written.

`ty` and `val` collapse into `item`. That is the whole change, and it is forced:
at a path leaf the namespace is not observable, so either it leaves the key
there or path leaves stop resolving. A reach cannot be per-position — a
declaration does not know how it will be reached — so "leaves the key at path
leaves" means "leaves the key for everything path-reachable".

**Both required properties hold.**

- A field and a same-named method stay distinct: `…·WatcherHealth·healthy·field`
  against `…·WatcherHealth·healthy·item`. A field is reachable in Rust ONLY
  through a dot with no call — `Type::field` is not a path Rust admits — so the
  two reaches never overlap. 20 such pairs in this repo.
- A path-reached enum variant merges with its declaration: both sides mint
  `…·Status·Active·item`, because the reference's reach comes from "this is a
  path leaf" and the declaration's from "this is not a field, a macro or a
  module".

**Nothing is guessed and nothing has to be refused.** The reach is a function of
the use syntax alone, so a use site mints exactly ONE candidate identity. There
is no candidate set, no uniqueness gate, and no ambiguity for the resolver to
adjudicate — which also means no rung has to consult what has been scanned, and
R6 keeps the property that caught the ghost-node defect.

**Every ambiguous Rust path form, and what it does under the rule.** `A::B` at a
use site, where `B` is:

| `B` is | Rust namespace(s) | reference mints | declaration mints | merges |
|---|---|---|---|---|
| unit enum variant (`Status::Active`) | type + value | `item` | `item` | yes |
| tuple enum variant (`Status::Err(e)`) | type + value | `item` | `item` | yes |
| struct enum variant (`Status::Err { .. }`) | type | `item` | `item` | yes |
| associated const (`Foo::MAX`) | value | `item` | `item` | yes |
| associated type (`Foo::Output`) | type | `item` | `item` | yes |
| associated fn or method (`Foo::new`, `x.new()`) | value | `item` | `item` | yes |
| unit struct as a value (`WindowsProvider`) | type + value | `item` | `item` | yes |
| tuple-struct constructor (`Wrapper(x)`) | type + value | `item` | `item` | yes |
| a type in a module (`config::Settings`) | type | `item` | `item` | yes |
| a fn in a module (`config::load`) | value | `item` | `item` | yes |
| a trait's own associated item (`Trait::method`) | value | `item` | `item` | yes, onto the trait's member; an impl's copy is a `TraitMember` and a separate identity by design |
| a module as a path prefix (`a::b::c`) | type | not a target — `a::b` becomes the `module` segment | `mod` | n/a, and that is the point |
| a re-exported item (`installer::install`) | either | `item` at the path it is written with | `item` under its DEFINING module | **no** — re-exports are not followed; a recorded gap, not this rule's |
| a POSITIONAL field (`x.0`, `Wrapper(x).0`) | n/a — not a path | `0` with reach `field` | `0` with reach `field` | yes |

**A tuple-struct field's `member` segment is its DECIMAL INDEX**, reach `field`.
Stated here rather than left to an implementer because both sides of the merge
contract must mint the same string and the reference side only ever sees `x.0`.
`Wrapper(String)` declares `…·Wrapper·0·field`; `w.0` mints the same. It is a
field, not an item, because it is reachable only through a dot with no call —
the same rule that separates `healthy·field` from `healthy·item` above.

The one thing the rule still cannot read off a path is WHICH segment names the
type — `a::b::c` may be item `c` of module `a::b` or member `c` of type `b` in
module `a`. That is a different question from the reach, it is decided by the
grammar's `names_a_type` predicate, and a misread there produces a dangling edge
rather than a wrong one, because the two splits put the boundary in different
places and the loser names no declaration at all (A4 catches it).

**Why `mod` is its own reach and not just `item`.** Because without it the
collapse would manufacture wrong edges of a shape this repo already contains.
`crate::installer::install(&acps, scope)` calls a function re-exported from
`installer::install`, and `installer::install` is also a MODULE. Collapse `ty`
and `val` while leaving a module in the collapsed bucket and that call edge
lands on the module — a wrong edge where today there is a dangling one. 7 such
references, 6 distinct identities. Giving modules their own reach costs nothing,
because no reference ever mints `mod`: a module is spelled as the `module`
segment of other identities, never as a target.

**What is given up, and the guard that replaces it.** The key no longer
distinguishes a type from a function that share a name in one scope. Measured on
this repo: the collapse creates ZERO new declaration collisions once modules are
moved to `mod`, and exactly one without that move (`sensei-bootstrap`'s
`mod config` / `fn config()`). Beyond this repo the residual cases —
`trait Foo` beside `fn Foo()`, `type Foo` beside `const Foo` — all require a
declaration that violates rustc's own naming lints. That residue is not argued
away, it is CHECKED: see A7. Today's namespace segment prevents one collision
class and silently tolerates the possibility of any other; A7 turns any
collision into a named failure instead.

**Rejected, with the reason.**

- *Emit `Unresolved` when the namespace is ambiguous.* Refuses 100 references
  this repo could resolve, and every path leaf in the next one, to preserve a
  discriminator that buys one collision the `mod` reach closes for free.
- *Try both namespaces and accept a unique hit.* Needs the complete declaration
  set at resolution time, so the ladder gains a rung that reads what has been
  scanned — the input R6 exists to keep out, and the one that produced 659 ghost
  nodes the last time. It also replaces "both sides mint the same string" with
  "one side mints a set and queries", which is the merge contract this whole
  design rests on, and it does not survive incremental re-indexing of one file.
- *Mint a multi-namespace declaration in every namespace it occupies.* Merges
  correctly and gives one symbol two nodes, splitting its inbound edges across
  them. That is the two-half-symbols failure the merge key exists to prevent,
  arrived at deliberately instead of by accident.
- *Drop the discriminator entirely.* Reopens 20 field-versus-method collisions
  in this repo alone.

## 3. What the walk produces

A file is parsed ONCE. One walk produces one value:

    FileFacts {
      language, package, module,
      symbols:    Vec<Symbol>,
      references: Vec<Reference>,
      relations:  Vec<Relation>,
      imports:    Vec<Import>,
    }

### 3.1 Symbol — every declaration

Functions, methods, classes, structs, enums, interfaces, traits, type aliases,
constants, modules, **fields, properties and enum variants**.

Each carries: fqn, kind, name, span, visibility, docstring, **declared type**
where the language states one (field type, return type), and **parameters as
typed props** (a parameter is not navigable, so it is not a node).

### 3.2 Reference — every use, resolved or not

    Reference { from: Fqn, kind: Calls | Reads | Constructs | ..., at: Span,
                target: Resolution }

    Resolution = Resolved(Fqn)
               | Unresolved { reason: Reason, evidence: Evidence }

`Resolution` is total. There is no representation for "nothing", so a reference
cannot be silently omitted. `Reason` is a closed enum with one variant per
distinct cause — causes are never collapsed, because telling them apart is how
coverage is measured. `Evidence` carries what the walk saw (the receiver
expression, the import in scope, a type it could not place) so a later pass has
material to work with rather than a bare name.

### 3.3 Relation — OO structure

`extends`, `implements`, impl blocks, trait impls, mixins, decorators, and member
ownership. Emitted by the same walk.

## 4. Requirements

Each is independently testable.

**R1. One parse per file.** No stage may re-read source bytes. One exception,
stated in R10.3 and nowhere else: reconcile re-reads a file whose parse produced
no declarations where it previously had some. That is not a stage recovering a
fact the walk already had — which is what this rule forbids — it is reconcile
deciding whether to trust the first read at all.

**R2. No reference is ever dropped.** Every use site in the AST yields exactly
one `Reference`. A miss yields `Unresolved` with a reason, never omission.

**R3. Nothing captured is discarded.** A declared type read from the AST reaches
the database. This binds the persistence path as well as the walk: the wire
format between producer and storage carries everything the walk captured.

Two corollaries on the READ side, both of them defects that were live:

- An occurrence list is written PER FILE. An edge is keyed
  `(folder, source, target, kind)` and two files can produce one edge, while the
  jsonb merge the graph uses replaces a key rather than appending — so a flat
  list meant the second file's write erased the first's, uncounted.
- Where a fact lands in a COLUMN as well as a prop, the read path reads the
  column. A round trip that rebuilds both sides from props compares the encoder
  with itself: measured, eight one-line inversions of production column mappings
  survived the whole suite, and three of them changed what every graph query
  would answer.

**R4. No fabrication.** A resolution that cannot be proven is `Unresolved`. A
reference must never point at a symbol that is not its target; a wrong edge is
worse than a missing one.

**R5. Externals are named, never opened.** A use of a library resolves to
`lib·<package>·<member>`. The graph answers "which libraries do we use" and
"which of their members do we use". No dependency source is parsed.

**R6. Order independence.** Indexing the same files in any order produces the
same graph. Resolution must not depend on what has been scanned so far.

**R7. Shared rules are shared.** FQN construction, the resolution ladder and the
reason codes live in one place. A language module owns only "how do I read this
grammar".

**R9. A field cannot be lost at a boundary.** Every persistence function takes a
typed row, never a positional argument list, and every conversion from a fact to
a row destructures its source EXHAUSTIVELY. Adding a field to `Symbol` or
`Reference` must fail the build until someone decides where it goes.

A positional list is a hand-copied enumeration of an object's fields: add a field
to the producer and nothing breaks, the value simply stops. That is how
`extract_return_type` ran on every function for months while the value never
reached a column. Exhaustive destructuring moves that failure from
runtime-invisible to build-time-loud. Existing offenders outside v2 are issue #161.

**R8. Pattern-detection completeness.** The facts emitted must be sufficient to
derive OO patterns from nodes and edges alone, with no return to source. Derived
from the patterns worth detecting:

| pattern | facts required |
|---|---|
| Adapter | `implements`, field type, call member -> field's member |
| Decorator | `implements`, field type |
| Strategy | field/param type, `implements` |
| Factory | return type, construction edge, `implements` |
| Singleton | field type, `static`, visibility |
| Observer | param type, `implements` |
| Facade | call edges, member ownership |

Union: field types, parameter types, return types, `implements`/`extends`,
construction edges, member ownership, and member -> field-member call edges. All
are required by §3. Pattern NAMING is a query over these; it is not part of the
walk.

**R10. Re-indexing a file replaces exactly what that file claims.** Today the
write path only ADDS. A deleted call keeps its edge and reads back as a live
fact, which is a wrong edge (R4) that no query can tell from a real one. The fix
is a per-file set diff, and every part of it that can go wrong is settled below
rather than at the keyboard.

Every number in R10 was measured over this repository's own Rust on the tree at
`d58e230d`: 372 files, 12,519 declarations, 82,913 distinct edges, 119,248 edge
sources.

### R10.1 The unit of attribution is not the same for every row

Reconcile has to know which rows belong to file F. `file_path` answers that for
one row type and for no other, and the difference is not a detail — it is where
the naive implementation destroys data.

| row | belongs to F when | not `file_path` because |
|---|---|---|
| a DEFINITION in `nodes` | F is in the node's CLAIM SET (R10.4) | the column holds one path and a claim set can hold two |
| a STUB in `nodes` | never — a stub belongs to no file | it has none: `file_path IS NULL` is what makes it a stub |
| a row in `edges` | never — an edge row is owned by the union of its contributors | `edges` has no column for the emitting file; `target_file` is the TARGET's path, not the emitter's |
| `edges.props.occurrences -> F` | always, and this is the only edge-side unit | this IS the unit |

**So: for a declaration the unit is the ROW; for an edge the unit is the
OCCURRENCE KEY inside the row.** Reconcile releases a claim on the first and
deletes a key from the second. An edge row is deleted only when its occurrence
object is empty — see R10.4.

The measured trap. `nodes.file_path` is NOT "the file whose edges hang off this
node". A file's own module node is minted by the `mod x;` that names it, which
sits in the PARENT file, while the file-scope edges sourced at that node are
emitted by the child. Measured: **356 of 372 files have their own module node
declared by another file** (`rust·sensei-bootstrap·config·mod` is declared by
`lib.rs`), 16 are crate roots no file declares, and **3,290 file-scope edges
across 355 files are sourced at a node whose `file_path` names a different
file**. `DELETE FROM sensei.nodes WHERE file_path = $F` — which is what
`delete_nodes_by_file` does today — therefore deletes every child module node of
a `lib.rs` or `mod.rs`, and `edges.source_id ... on delete cascade` takes all
3,290 of those children's edges with it, in silence, from files nobody
re-indexed. Reconcile must not use that predicate, and cutover must not keep it.

Finding F's edges without a jsonb scan. Every edge F contributes to is sourced
at a symbol F declares or at F's own module identity — with **26 exceptions out
of 119,248 edge sources** (0.02%), all of them split-impl anchors
(`impl PgStore` in `commands.rs` anchors at
`rust·senseid·db::pg_store::commands·PgStore·item`, which no file declares —
the gap §2.1 records). So the affordable predicate is

    props->'occurrences' ? F

**One test, and it is the definitive one.** The occurrence key is the ONLY unit
of edge attribution — that is what the fourth row of the table above states — so
asking which edges carry F's key IS the question, and no fqn-based approximation
of it is needed. This requires
`create index … using gin ((props->'occurrences'))`, which **stage 0 creates**
(§7h item 6).

WITHDRAWN: the two-disjunct interim predicate

    source.fqn = ANY(claims(F) ∪ {file_fqn(F)})
      OR (source.resolved = false AND props->'occurrences' ? F)

An earlier draft used this while the gin index was deferred to the end of the
build, with the first disjunct as a cheap fqn filter and the second as a
correctness clause. It is a workaround for an index stage 0 now creates, and its
first disjunct is measurably incomplete on its own — 26 of 119,248 edge sources
(0.02%) are split-impl anchors no file declares. A narrowing of this predicate
is exactly the defect already committed in `v2_edges_contributed_by`
(`AND (s.fqn = ANY($current) OR s.resolved = false)`), which never revisits an
edge whose source this file deleted but which resolves elsewhere, so its stale
occurrence survives forever. Scoping by folder is enough: **19.7ms measured
against the largest folder here (330,437 edges).**

### R10.2 Removal — SUPERSEDED by R10.7 / R10.8

**This section prescribed DEMOTION. Demotion was rejected. R10.7 and R10.8 in
§7b are the rule that replaced it; read those before implementing anything
here.** What survives in this section is the MEASUREMENT, because the corrected
design is constrained by it more tightly than the rejected one was.

The rejected rule was: a definition F no longer declares keeps its node, with
`resolved = false`, `file_path = NULL`, every definition column cleared, and its
inbound edges untouched. It fails twice.

- **It cannot fire when it is needed.** If `x.rs` is edited and `x()` deleted,
  nothing in a parse of `x.rs` distinguishes "x() was removed" from "x() was
  never here". Demotion needs a deletion detector it does not have — which is
  what R10.8 supplies by diffing the previous claim set.
- **What it leaves behind reads as resolved.** A node with `resolved = false`
  and `file_path = NULL` is still a live row with an id, so every inbound edge
  keeps a non-null `target_id`, and every consumer testing
  `target_id IS NOT NULL` calls it resolved. Measured: **67,839 such edges in
  `sensei_test`.** Demotion does not defer the loss, it disguises it.

What replaces it, one line each:

- an UNPARSEABLE file removes nothing and marks its contents DIRTY (R10.7);
- a DELETED declaration is found by diffing claims, and its node is deleted
  AFTER its inbound edges are unresolved (R10.8).

**The three measurements stand, and R10.8 is what they now constrain.**

1. **The input is not trustworthy.** R10.3 shows a damaged parse cannot be
   detected. This was the argument for demotion; under R10.8 it is the argument
   for R10.7 — an unparseable file must never be processed as an empty one,
   because emptiness is what triggers removal.
2. **The blast radius is measured and large.** 1,485 of 12,519 declarations have
   inbound edges from a file other than their own — 3,070 (declaration,
   referring file) pairs — and the worst is
   `rust·senseid·db::pg_store·PgStore·item`, referenced from **67 files**. That
   is the size of the set R10.8 must UNRESOLVE rather than allow to cascade.
3. **A node deletion cascades in BOTH directions and counts nothing.** The DDL
   is explicit: `source_id … on delete cascade`, `target_id … on delete
   cascade`, and `nodes.parent_id … on delete cascade`. There is no version of
   "remove the node, keep the edges" — so R10.8's unresolve-first step is not
   bookkeeping, it is the only way its own table can be made to satisfy it.

**Reconcile does not garbage-collect.** A stub with no edges and no children is
collected by the pass that already owns that predicate
(`prune_orphan_stubs_scoped`). Two owners of deletion is how a race gets built,
and the existing owner already runs on every reconcile tick.

### R10.3 Parse failure versus empty file — the case that must not be guessed

There are two failures here and only one of them is solvable. Both are stated
because conflating them is what produces a confident wrong answer.

**A failed READ never reaches reconcile, by type.** `read` returns
`Result<FileFacts, ReadError>`, so an IO failure, an unloadable grammar
(`GrammarUnavailable`), a tree-sitter refusal (`NotParsed`) and an unmintable
file identity (`NoFileIdentity`) are all `Err` and produce no `FileFacts` at
all. Reconcile takes the facts of a SUCCESSFUL parse and there is no other way
to obtain them: `FileFacts` must have no production constructor other than a
language module's `read`, enforced the way the other v2 grammar rules are
enforced — by a guard test over the v2 sources. On `Err`, reconcile does not
run, the file keeps the graph it had, and the failure is recorded so it is
visible rather than inferred later from a hole. Nothing may turn that `Err` into
an empty fact set; `.unwrap_or_default()` here is the exact shape R4 forbids.

**A DAMAGED parse is not detectable, and this was measured rather than
assumed.** tree-sitter returns a tree for anything.

These were TWO experiments over two populations, and an earlier draft reported
them as one, which made the numbers look inconsistent. Stated separately:

- **Experiment 1, the 294 files declaring ≥10 symbols.** Truncating each to half
  its bytes produced **0 `ReadError`s**. Symbols fell 12,167 → 6,973 (57%
  kept), references 109,328 → 53,524. Per-file retention: min 0%, **median
  52%**, max 100%. Worst single file: 354 symbols → 187.
- **Experiment 2, the whole corpus.** `tree.root_node().has_error()` is true for
  **6 of 372 intact, valid** files, and false for **53 of 366 truncated** ones.
  The two populations differ by exactly the 6, but the run's notes do not record
  whether that is why, so do not cite a reason for it. The conclusion does not
  rest on the difference: the gate fails in BOTH directions, refusing to ever
  reconcile six healthy files and waving through fifty-three damaged ones.
- `""`, `"fn main() {"` and `"}}} not rust @@@"` all return `Ok` with 0 symbols,
  0 references, 0 relations and 0 imports — identical to a genuinely empty file.

So reconcile does not try to decide whether a parse was good. Two rules replace
the detector it cannot have.

1. **Damage is contained by R10.7, and what escapes it is RECOVERABLE.** An
   unparseable file removes nothing at all, so the whole-file case never reaches
   a removal decision. What remains is partial damage on a file that still
   parses, and there the loss is recoverable rather than fatal: the next healthy
   index of the same file re-declares the symbol under the same fqn and every
   inbound edge, unresolved by R10.8 with its `target_name` intact, re-resolves
   to it. Recoverability — not survivability-in-place — is the property that
   makes an undetectable input acceptable.
2. **A brake on the one diff shape that is almost always damage.** If F
   previously claimed ≥1 declaration and now claims zero, reconcile does not
   apply the diff on the first read. It re-reads F from disk once and applies
   the diff only if the second read also claims zero. A truncated read is a
   transient — an editor mid-save, a partial write — and does not survive a
   second read; a file genuinely emptied to `//! moved to bar.rs` reads zero
   twice and reconciles on the second. This is R1's one exception, and it costs
   one extra parse on a shape that a healthy scan of this corpus produces zero
   times.

What the brake deliberately does NOT cover: partial damage. A file truncated to
half still claims half its declarations, the brake does not fire, and the other
half are removed by R10.8. That is the residual, it is named rather than papered
over, and it is bounded by RECOVERABILITY: the deleted nodes' inbound edges are
unresolved with `target_name` retained, so the next healthy index of the same
file restores both the declarations and the edges that pointed at them. Nothing
in another file has to be re-indexed for that to happen. A threshold
on the retained fraction was rejected: the measured distribution runs from 0% to
100% with a median of 52%, so any cut-off refuses real edits at the same rate it
catches damage, and it would silently pin a file's graph to a stale state with
nothing counting it.

**The brake guards INFERRED emptiness only.** A file that is absent from disk is
an OBSERVED fact the caller already holds, not something concluded from a parse,
so R10.5's deletion path does not re-read and does not brake.

### R10.4 Two files can claim one thing — nodes as well as edges

**Edges.** Occurrences are already keyed by file
(`props.occurrences = {"<path>": [...]}`), because two files can produce one
`(folder, source, target, kind)` row and jsonb `||` replaces a key rather than
appending. Reconcile removes F's key and only F's key:

    props = jsonb_set(props, '{occurrences}', (props->'occurrences') - $F)

and deletes the edge row when the result is `{}`. It must never use the
source-scoped form `DELETE FROM sensei.edges WHERE source_id = ANY(…)` — that is
the v1 shape and it takes every other file's occurrences with it.

Measured: **4 of 82,913 corpus edges have more than one contributing file**, all
four sourced at `rust·senseid·base_url·item`. Four is not "safe to ignore" — it
is precisely the class of loss that nothing counts — but it does mean the corpus
cannot exercise this. The test is a two-file fixture that mints one source
identity, and the corpus count is the ratchet that says whether the case grew.

**Nodes need the same treatment, and this is the new structure R10 introduces.**
Measured: **2 identities are claimed by two different files** —
`rust·senseid·base_url·item` and `rust·senseid·main·item`. Both are A7
violations, and A7 admits no exceptions and keeps no allowlist (an earlier draft
of this paragraph cited "A7's known list of 15"; no such list exists in A7 or
anywhere else, and the reference is withdrawn). Without a claim set, reconcile
cannot tell "F was the only declarer" from "F was one of two", so releasing F's
claim would remove a node another file still defines. A node therefore carries

    props.claims = { "<file path>": true, … }

merged with the same `jsonb_set` idiom the occurrences use, for the same reason
(plain `||` on `props` would replace the whole object). This lives in `props`
DELIBERATELY, not as a deferral: a claim set is a variable-length set keyed by
path, `props` is the column the schema documents as extensible, and stage 0
opening the schema (R13) does not change the choice.

The rule: reconcile removes F's key from `props.claims`, and then the two cases
diverge completely — which is why they are stated separately rather than as one
rule with a condition.

- **`props.claims` is now EMPTY.** No file declares this identity any more. The
  node is DELETED, after its inbound edges are unresolved, per R10.8.
- **A claim REMAINS, and the released one was the one in the `file_path`
  column.** The node survives — another file still defines it. Its definition
  columns now describe a file that no longer declares it, so they are cleared,
  and the row states "no definition known" until the surviving claimant is next
  indexed and re-fills them. Reconcile does not pick a winner and does not
  invent the survivor's columns, which it does not have.

The second case is the ONLY circumstance in which a node is left with cleared
definition columns. It is not demotion returning under another name: the node
still has a claimant, so it is a row awaiting refresh, not a row nobody
declares.

Two files claiming one identity is an A7 violation, not a state to support
gracefully. So reconcile REPORTS it — the same way `persist::write` returns
`Collision` rather than swallowing it — and the repair belongs to the identity
rule, not here.

### R10.5 Deletion, rename, and package rename

**A deleted file is `reconcile(F, claims = ∅)`.** The same code path, so there
can be no second removal semantics that behaves differently from an emptied
file. A folder deletion is N file reconciles, never a path-prefix `DELETE`, for
the cascade reason in R10.2.

Nothing in the walk can trigger it — there is no file to walk — so the trigger
is the caller's, and all three already exist and all three must be re-pointed at
reconcile at cutover:

- the scan_state diff's `plan.removed` loop in
  `tasks/handlers/process.rs::process_git_folder`;
- `tasks/handlers/scan.rs::prune_vanished`, the safety net for nodes that
  outlived their scan_state row;
- the `delete_file` / `delete_folder` task handlers, the fs-watcher path.

**A rename is a deletion plus an addition, and needs no case of its own.** In
Rust the module path IS the file path, so renaming `a/b.rs` to `a/c.rs` turns
`pkg·a::b·Foo·item` into `pkg·a::c·Foo·item`: nothing is "the same symbol at a
new path". Reconcile `a/b.rs` with an empty claim set, index `a/c.rs` normally.

**The case the question is really about is a symbol MOVING between two files
that share a module path** — `mod b { fn foo() }` inline in `a.rs` moved out to
`a/b.rs`, or anything moved between the two crate roots of one package. The fqn
is unchanged and `file_path` changes. Which claim wins: **neither, because a
claim is not a winner.** Both files sit in `props.claims` while both declare it,
the old file's re-index removes its key, the new file's adds its own, and
because a SET is order-independent the answer does not depend on which file the
scan reaches first (R6, A6). Release-before-claim is explicitly NOT required:
new-first means both claim it briefly, old-first means neither does and the node
is a stub in between, and both intermediate states are true. What is forbidden
is deleting on release — under deletion, "old first" destroys the node the new
file was about to promote, and which happens is decided by scan order.

**A package rename is a full-package reindex, not reconcile's problem.** Say it
plainly: it is not that reconcile handles it badly. Reconcile handles the
renamed package's own files correctly, one at a time, releasing every old
identity and claiming every new one. What it cannot reach is everything ELSE —
every reference to that package from every other package changes at the same
moment (`rust·<old>·…` and `lib·<old>·…` alike) and those references live in
files the rename did not touch, so no set of reconciles over CHANGED files ever
visits them. The trigger is a folder-level observation, "the manifest's `name`
differs from the one the graph was built with", made by the manifest reader that
already supplies `package` to the walk; the action is to re-index every file in
the scan root. Reconcile's only obligation under a rename is not to corrupt, and
it does not.

### R10.6 The invariant

> **After `reconcile(F, facts)`, the graph records F as stating exactly what
> `facts` states and nothing more; every other file's statements are unchanged;
> and every identity that only F previously stated is REMOVED, with every edge
> that pointed at it UNRESOLVED rather than deleted.**

Four clauses, each independently testable, and in this order:

- **(a) F's contribution is exactly the new facts.** The set of nodes whose
  `props.claims` contains F equals `facts.symbols`' fqns, and the set of
  `props.occurrences -> F` lists equals what the grouping of `facts` produces.
- **(b) No other file's contribution moved.** Read the folder back before and
  after with F's contribution projected out; the two must be equal. This is the
  clause that catches a cascade, a source-scoped edge delete, and a
  `file_path`-keyed node delete — all three of which pass (a).
- **(c) What F alone dropped is GONE, and its references survive it.** For every
  fqn in `previous_claims(F) \ facts.symbols`: no node exists with that fqn, and
  every edge that targeted it has `target_id IS NULL` with a non-null
  `target_name`. Neither half is sufficient alone — the node still existing is
  the demotion defect, and the edge being gone is the cascade defect.
- **(d) An unparseable F changes nothing.** `reconcile` is not reached at all
  (R10.3: a failed read is `Err` by type). The file is marked unparseable with
  its reason, F's existing nodes and edges are marked DIRTY, and the row counts
  for nodes, edges and claims are IDENTICAL before and after. This is the clause
  that separates "cannot read it" from "it declares nothing", which demotion
  conflated and R10.7 exists to keep apart.

Reconciling F twice with the same facts must leave all three clauses holding and
change no row the first pass did not already change. Idempotence is what lets
the invariant be checked by RUNNING reconcile rather than by reading it, and it
is the property a partial implementation loses first.

## 5. Deferred — and what is captured now so they stay cheap

These are not in this pass. They are NOT ruled out, and the difference matters:
a capability declared impossible gets designed around, and the design then
actively obstructs it later. For each, the walk captures the evidence a future
pass needs, so filling the gap is a new consumer of existing facts rather than a
re-parse and a schema change.

**Type inference for unannotated values.** `let x = foo()` where `foo` is
first-party is NOT inference — `foo`'s fqn is mintable at the call site and its
return type is on its node, so it is a graph lookup and R2/R3 already cover it.
What is deferred is inference proper: a value whose type no declaration states.
CAPTURED NOW: the binding's provenance in `Evidence` — that `x` is the return of
`foo` — so a later pass has the input without re-reading source.

**Trait dispatch.** `x.fmt()` where `x: impl Display` needs the impl set and
coherence rules. CAPTURED NOW: the receiver's stated bound, and every impl as a
`Relation`. A later solver enumerates candidates from the graph; it does not need
the source.

The EASY end of the same case is already measurable and is deferred with it: a
member declared in `impl Trait for Type` is minted
`<lang>·<pkg>·<mod>·<Type>·<Trait>·<member>·<reach>` (§2), and no use site can
spell the `Trait` segment — `x.default()` and `Type::default()` both mint the
plain member form. So the declaration and every reference to it are two halves of
one symbol: the failure the reach rule closed for plain paths, one level in.
Measured over this repository: **22 references over 3 identities**, all
`Default::default`, out of 443 trait-impl member declarations —
`the_references_that_name_no_declaration_are_a_measured_and_split_set` is the
ratchet.

It is NOT closed by dropping the `Trait` segment, and the reason is a
measurement rather than a preference. No type in this corpus has two traits
supplying one member name, so dropping it would create zero collisions here —
which is exactly the problem: it would delete the only thing separating
`<X as Display>::fmt` from `<X as Debug>::fmt`, a shape no test in this
repository could then catch regressing, and R4 ranks the wrong edge that
produces below the missing one it removes. The real answer is the impl-set
lookup above: which trait supplies `default` on `CopyLimits` is a query over the
whole graph, and a per-file ladder that answered it would answer differently
depending on scan order (R6).

**Blanket impls.** `impl<T: Foo> Bar for T` grants `Bar`'s members to every
conforming `T`. CAPTURED NOW: the impl's generic parameters and their bounds, so
the rule is present as data and a solver can match against it later.

**Macro-generated code.** `#[derive(Serialize)]` produces an impl in no source
file. CAPTURED NOW: the invocation site and the macro's resolved path, so a later
expansion pass knows what generated what and where. Without this, expansion would
require a full re-walk.

**Untyped JavaScript.** Nothing states a type, so there is nothing to read.
CAPTURED NOW: a distinct `Reason` for "no annotation exists here" versus "an
annotation exists and we did not read it". The histogram must separate a gap that
is CLOSEABLE from one that is not, or nobody can tell which is worth work.

**Pattern classification** runs over nodes and edges (R8), not inside the walk.

The rule behind all of these: it is legitimate to defer WORK; it is not
legitimate to discard EVIDENCE. Anything the AST states and a future pass could
need is captured now, because re-parsing 48,646 files to recover a field we
already had is the expensive version.

## 6. Acceptance

Thresholds, not comparisons.

- **A1.** Import-mediated references resolve at ≥99.9%. An import names its
  target; failing here means the grammar is misread.
- **A2.** Zero references dropped. Count of `Reference` values equals count of
  use sites in the AST, verified by a walk that counts independently.
- **A3.** Every `Unresolved` carries a reason, and the reason histogram accounts
  for 100% of them.
- **A4.** After a COMPLETED scan, every edge whose target is not a definition
  somewhere falls into one of three NAMED populations, and the count of each is
  reported and ratcheted:
  - the target is `lib·` — external, complete by R5, will never have a file;
  - the target is PARTIAL — referenced but not yet declared. **18,450 live**, and
    R10.7d makes this the ordinary case, not an error;
  - the target was DELETED by R10.8 — `target_id IS NULL`, `target_name` kept.

  Anything OUTSIDE those three fails. Measured today: **32 references name
  nothing the package declares anywhere** (§2.1), and that 32 is the ratchet —
  it may fall, it may not rise. Stated as "zero" this gate fails 18,450 times on
  the first run and gets waived, which is how an acceptance gate dies.
- **A5.** Fields, properties and enum variants exist for every type that declares
  them, and "what shape is this data" is answerable.
- **A6.** Re-indexing in a different file order produces an identical graph.
- **A7.** No two declarations mint one identity. Over the corpus, every fqn the
  walk produces is minted by exactly one declaration; a violation names both
  sides and fails, rather than letting one silently overwrite the other. This is
  the guard that lets §2.1 drop the type/value discriminator: the collision that
  discriminator existed to prevent becomes a checked property instead of an
  assumed one, and which of two colliding declarations wins is otherwise
  scan-order dependent, so this is also an A6 obligation.

  **There is no allowlist and there are no known exceptions.** The gate is a
  CORPUS-WIDE run of the walk that groups every minted fqn and fails on any
  group of size >1, printing both declaration sites. A fixture asserting that a
  field and a method mint different strings is necessary and not sufficient —
  it cannot see a collision between two files. The two multi-claimed identities
  R10.4 measures (`rust·senseid·base_url·item`, `rust·senseid·main·item`) are
  A7 FAILURES to be repaired by the identity rule, not entries in a tolerated
  set.
- **A8.** Re-indexing a file removes what it stopped claiming and nothing else.
  R10.6's four clauses hold over the corpus for: a file that lost a declaration,
  a file that lost a use site, a file emptied, a file deleted, a symbol moved
  between two files sharing a module path, and **a file that became
  unparseable**. Zero rows belonging to a file that was not re-indexed change in
  any of them, and in the unparseable case zero rows change AT ALL. This is what
  makes a stale edge a caught failure rather than a fact the graph reports.
- **A9.** An unparseable file is ACTIONABLE and REACHABLE (R10.9). For a file
  the grammar rejects: `files.skip_reason = 'parse_error'`, the detail column
  holds the parser's verbatim message with line and column, MCP can answer
  "which files in this project are unparseable, and why", and every query
  returning one of that file's nodes labels it DIRTY with the reason. A dirty
  flag nothing can query is invisible, so an implementation that sets the state
  and stops does not pass.

## 7. Build order

Rust, then js, ts, svelte. One language at a time; the trigger to move on is the
current language passing §6, not elapsed time.

Per language:
1. Build the walk and its persistence path with no caller wired up.
2. Differential harness over the real corpus: every difference from the existing
   output is either an improvement or explained. This is the gate.
3. Cut over that language only.
4. Reindex, verify §6.
5. Retire the superseded module.

The existing indexer keeps running until step 3 for a given language, so the
graph never goes stale.

## 7b. R10 CORRECTION — dirty is not deleted, and deleted is not demoted

An earlier draft of R10 answered both "this file will not parse" and "this file
no longer declares X" with one mechanism: DEMOTION, which keeps the node and
nulls its `file_path`. That is wrong, and measurement shows how wrong: it leaves
an edge whose `target_id` is set pointing at a node that names nothing, which is
the ghost-stub-edge shape this rewrite exists to remove. Consumers compute
resolved as `target_id IS NOT NULL`, so `get_callers(z)` reports "z is called by
x" after `x()` has been deleted. R4 ranks a wrong edge worse than a missing one,
so demotion trades a data-loss RISK for a wrong-data CERTAINTY. Backwards.

They are two different events and get two different mechanisms.

### R10.7 — the file entity ALREADY EXISTS: it is `scan_state`

Correcting an earlier draft that proposed putting status in the file NODE's
props. There is a per-file row already, and it is the right owner:

    sensei.scan_state (folder_id, file_path, mtime, content_hash,
                       indexed_at, modified_at, skip_reason)

`skip_reason` is an enum and it ALREADY carries the reason code this design
needs: `unsupported_format | binary_content | invalid_utf8 | parse_error |
excluded_by_config`. Live today: 48,646 indexed, 17 binary_content, 2
invalid_utf8. `parse_error` exists as a value and is effectively unused.

So the status and the reason code need nothing new. What is missing is the
DETAIL — there is no column for the parser's message and location, and R10.9
requires it, because "parse_error" tells an agent a file is broken and
"x.rs:142: expected `}`" tells it what to do.

**Add one nullable column to `scan_state` for the failure detail.** That is a
DDL change and it is additive, so it must go through the dbd workflow rather
than being written by hand. It is deliberately NOT put in a node's props: the
code lives on scan_state, and splitting the detail somewhere else recreates the
two-copies-of-one-fact problem this requirement exists to avoid.

A symbol is DIRTY if its file's `scan_state.skip_reason` is set — a JOIN on
`(folder_id, file_path)`, never a flag written onto each symbol.

### R10.7b — folders roll up through the view that already exists

`sensei.folder_completeness` already derives a folder's state from its files and
its child folders, recursing over `parent_id` and reading nothing but persisted
per-file facts. Folder status is that view, extended to carry unparseable counts
— not a second mechanism.

### R10.7c — containment is one structure sliced many ways

The graph should answer "what contains this" at whatever granularity the caller
wants, from one containment relation rather than a bespoke query per level:

    folder -> file -> type -> member
    package -> module -> item
    library -> package -> symbol

These are different SLICES of the same parent/child structure, not different
structures. Member ownership (§3.3, D5) is that relation for symbols; folders
have `parent_id`; files sit between the two. A node knowing its file makes the
whole chain walkable in one direction and sliceable at any level — which is what
makes "show me this repo by folder" and "show me this package's types" the same
query with a different cut.

The one thing currently in the way: a symbol references its file by `file_path`
TEXT, so a file is not addressable the way every other level of the chain is.
**R13 closes it** — files become `sensei.files` rows with an `id`, and
`nodes.file_id` is the foreign key. There is NO `kind = 'file'` node with an
fqn, and an earlier draft of this paragraph proposing one is withdrawn: §2 already
gives a file its identity as the MODULE it declares, so minting a second fqn for
the same file would either duplicate that identity — an A7 collision, which is
the exact thing A7 exists to fail on — or create the second node §2 forbids.

### R10.9 — the failure must be ACTIONABLE and REACHABLE

Marking a file unparseable is only worth doing if it reaches something that can
fix it. Two requirements follow, and neither is optional.

**The reason must be specific enough to act on.** Not "parse failed" but the
parser's own error and WHERE: file, line, column, and the message. An agent
handed "x.rs:142: expected `}`" can fix the file; an agent handed "unparseable"
can only shrug. Whatever the grammar reports is recorded verbatim — this is
evidence, and R1's rule against discarding evidence applies.

**It must be reachable through MCP.** A dirty flag nothing can query is invisible
and therefore useless. Required:

- a way to ask "which files in this project are unparseable, and why", returning
  the path, the reason and when it started failing
- every query that returns a dirty node labels it as dirty, with the reason,
  rather than returning it as though it were current

This makes the graph SELF-HEALING through the agent, which is the point: the
indexer cannot fix a syntax error, but the thing consuming the indexer can. The
loop is — file breaks, index reports it precisely, agent fixes it, next index
clears the flag. No human has to notice.

It is also a G2 capability. "Which files is this repository unable to parse" is
exactly the kind of thing a person wants when judging whether a codebase is in
trouble, and today it is unanswerable.

Contrast with what this replaces: demotion silently served WRONG data and gave
nobody anything to act on. Dirty serves the last known-true data, says so, and
names the fix.

### R10.7d — COMPLETENESS is a derived state with six values

A node is in exactly one of six states, and the node+file join gives all six
from columns that already exist. Verified on the live graph:

| state | condition | meaning | live count |
|---|---|---|---:|
| COMPLETE | `file_path` set, file has no `skip_reason` | the declaration was parsed | 346,506 |
| PARTIAL | first-party, `file_path` IS NULL | referenced, not yet declared | 18,450 |
| DIRTY | `file_path` set, file HAS a `skip_reason` | was complete, its file now fails | 0 |
| EXTERNAL | origin is `lib·` | a library symbol — NO file will ever exist | 21,928 |
| ORPHANED | `file_path` set, NO `scan_state` row for it | claims a file the scanner does not track | 8,147 |
| EXCLUDED | file's `skip_reason` is not a parse failure | the file is deliberately not indexed | 0 |

**ORPHANED is a PRE-R13 state.** It is enumerated because it exists today, in
the shipped graph, and must be swept as part of the R13 migration. After R13 it
is not merely undetected-but-possible, it is UNREPRESENTABLE: `nodes.file_id` is
a foreign key, so a node cannot name a file that has no row. **After R13 the
enum has five values and this row is dropped** — from the completeness type, from
the gap queue, and from R11.3. Do not write a persistent ORPHANED bucket; write
a one-time migration sweep.

ORPHANED IS THE DANGEROUS ONE AND IT EXISTS TODAY. 8,147 nodes name a
`file_path` for which there is no `scan_state` row at all, and they read as
COMPLETE to every consumer — `file_path` set, `resolved = true` — so the graph
asserts "declared in x.rs" about a file it is not tracking. That is wrong data,
not missing data. Three routes in, all live: `clear_scan_state_for_root` (which
the version-rescan path calls on purpose) drops scan_state without touching
nodes; a newly-excluded glob prunes scan_state and leaves the symbols; a file
deleted from disk leaves its declarations behind.

EXCLUDED is empty today only because excluded files never parsed and so produced
no nodes. It is reachable: index a file, then add a glob that covers it.

The six differ in TRAJECTORY, which is why they must stay separate:

| state | resolves when |
|---|---|
| PARTIAL | its file is parsed |
| DIRTY | its file is fixed |
| ORPHANED | its file is re-scanned, OR the node is pruned |
| EXCLUDED | the exclusion is lifted, OR the node is pruned |
| EXTERNAL | never — it is already complete |
| COMPLETE | — |

For ORPHANED and EXCLUDED the honest answer may be DELETE rather than wait, and
neither is detected today.

A NODE IS CONTAINED BY EITHER A FILE OR A LIBRARY, and the two are not the same
absence. A `lib·` symbol has no `file_path` and never will: R5 says we index the
USE of a dependency and never its internals, so "no file" is its COMPLETE state,
not a pending one. Treating it as PARTIAL would report 21,928 permanent gaps that
no amount of indexing can close.

The discriminator is already in the key — the fqn's first segment is `lib` for an
external symbol and the language for a first-party one — so no new column is
needed to tell the two containers apart.

    LEFT JOIN sensei.scan_state s
           ON s.folder_id = n.folder_id AND s.file_path = n.file_path

PARTIAL is the ordinary case, not an error: `x()` calls `y()` before `y.rs` is
parsed, so a node carrying only the fqn and the name is persisted, and the
declaration fills it in later by upserting on the same key. That is the
stub-and-merge model this design already rests on, and it is what makes indexing
order-independent (R6).

THIS IS EXACTLY WHAT DEMOTION GOT WRONG. It set `file_path = NULL` on a deleted
declaration, making it indistinguishable from PARTIAL — so the graph claimed
"we have not found this yet" about something that definitively no longer exists,
and every consumer treated a dead symbol as a pending one. The distinction it
destroyed is a trajectory one: PARTIAL is waiting for a parse that will arrive,
DIRTY is waiting for a fix, and a DELETED declaration is waiting for nothing.
**DELETED is not a seventh state** — under R10.8 it is the ABSENCE of a row, so
the six stay distinct by construction rather than by a flag someone has to set.

Every query that returns a node should return its completeness with it. A caller
that receives a symbol has to be able to tell "this is current", "this is not
declared anywhere yet" and "this is stale because its file is broken" apart —
they lead to different actions, and today they are indistinguishable.

### R10.7e — a resolved reference lands in a FILE or a LIBRARY, and that is DERIVED

Where a reference lands is a property of its target, so it is a join, not a
column. Measured on the live graph for `calls`: 83,820 land in a file, 41,194 in
a library, 36,556 in neither because they are still partial.

    target_id -> node -> file_path      (first-party: which file)
    target_id -> node -> fqn origin     (external: which library)

DO NOT copy the file id or library id onto the edge. It is derivable, and storing
it creates two representations of one fact that drift the moment a symbol moves
file or a node is re-resolved. That failure has already occurred three times in
this build — `occurrences` clobbered by a shallow jsonb merge, column values
verified against props the same writer wrote, and demotion making a deleted node
indistinguishable from a partial one. Each was one fact held twice.

The need behind the question is real and belongs in the VIEW (R10.7b/7c): "which
files does this file depend on" and "which libraries does it use" are answered
once, in one place, and no caller writes the join.

ONE EXCEPTION, where it is not derivable and therefore must be recorded: an
UNRESOLVED reference that is nonetheless known to land in a package — a call
chained onto an external call, where the crate is known and the member is not.
There is no target node to join through, so the package is genuinely additional
information and belongs on the edge. `edges.target_file` already exists and is
used for the analogous first-party hint.

### R10.7f — PACKAGE is observable, LIBRARY is enrichment

An external reference resolves to the PACKAGE or CRATE the import names, because
that is the only thing the call site can see. `import { X } from '@rokkit/ui'`
names `@rokkit/ui`; it says nothing about `rokkit`. Keying on the library would
derive identity from information the reference side does not have — the failure
that killed namespace-as-key and the kotlin source-root attempt.

    scan time   -> package / crate    @rokkit/ui, gateway-embedded, serde_json
    enrichment  -> library            rokkit, gateway        (optional, later)

The node references the package. A library groups packages and is attached
later, if the information ever arrives. Absent it, the package stands alone and
nothing is missing — the graph is not wrong, it is simply not yet grouped.

MEASURED STATE — CORRECTION. The level DOES exist in the schema; it is simply
unpopulated. `sensei.library_packages` is the library->package grouping and has
ZERO rows. Alongside it: `library_pages` holds 130 pages across 128 distinct
COMPONENTS, `library_skills` 10, `library_agents` 6. So the model is already
right and the data is not there — a very different problem from a missing design.

`sensei.libraries` currently holds packages (`@rokkit/actions`, `@rokkit/app`,
`@rokkit/core` as separate rows, all `kind = 'detected'`), because with
`library_packages` empty there is nothing to group them under. `library_kind` is
`detected | imported`, which records PROVENANCE, not level.

### R10.7g — the reference chain that makes docs, skills and agents reachable

This is why the package/library split earns its place. A node referencing an
external symbol should reach everything known about it:

    node -> lib_package (@rokkit/ui)        observable at scan
         -> library_packages                the grouping, currently empty
         -> library (rokkit)                declared, from llms.txt or add_library
         -> library_pages.component         128 components indexed
         -> library_skills / library_agents 10 and 6

Every link exists but two: `library_packages` is unpopulated, and a `lib_package`
NODE joins to a `libraries` ROW only by matching name strings, with no key
between them.

The payoff is concrete and serves G1 directly. An agent editing code that calls
`@rokkit/ui`'s `List` can be handed rokkit's `List` page and the
`rokkit-components` skill, instead of inferring the API from call sites. It also
answers the inverse for G2 — "which of this library's components do we actually
use" — which is a dependency-surface question nobody can ask today.

THE GROUPING MUST BE DECLARED, NEVER INFERRED. `@rokkit/*` -> `rokkit` is a
tempting prefix rule and it is a guess: `@types/node` belongs to no "types"
library, and the crates of `gateway` share no prefix at all. Grouping comes from
declared metadata (llms.txt, registry, an explicit `add_library`) or it does not
happen. An invented grouping is a fabricated fact about a dependency, and it
would be believed.

This is the `library -> package -> symbol` slice of R10.7c, with the top level
optional — the same shape as `folder -> file -> type -> member`, where a folder
may or may not be a project.

### R10.7h — `sensei.library.json` is the declared-grouping source, WHEN AVAILABLE

A library manifest at the dependency's root is how the grouping is declared
rather than guessed. Real example, `rokkit/sensei.library.json`:

    library:  "rokkit"                     the grouping name
    version, repo, branch, site
    skills[]  name, focus, path, url       5 entries
    agents[]  name, focus, path, url       3 entries
    llms      docs corpus path + index
    install   how to add a skill or agent

That populates `libraries`, `library_skills`, `library_agents` and locates the
pages corpus — four of the five tables in the R10.7g chain.

WHEN AVAILABLE is the operative clause. Most dependencies will never ship one.
Absent it the package stands alone: `lib·@rokkit/ui·…` is unchanged and complete,
and the graph is ungrouped rather than wrong. Grouping is enrichment (R10.7f), so
its absence is not a gap to be filled by inference.

THE ONE THING THE MANIFEST DOES NOT CARRY: the package list. Nothing in it states
that `@rokkit/ui` belongs to `rokkit`, so `library_packages` — the table with
zero rows — still cannot be populated from this file alone. The options are to
add `packages: [...]` to the manifest, or to read the repo's workspace members
(`package.json` workspaces, `Cargo.toml` members), which is equally declared, just
stated elsewhere. What must NOT happen is inferring it from the `@rokkit/*`
prefix: `@types/node` belongs to no "types" library and gateway's crates share no
prefix, so a prefix rule would fabricate groupings and they would be believed.

### R10.8 — a file that parsed is authoritative, and removal is REMOVAL

When `read` returns `Ok`, the claim set is the truth. A declaration the file no
longer makes is not demoted, it is DELETED, because ownership differs by fact
type:

- A DECLARATION is owned by the file that declares it. No claimant, no node.
- A REFERENCE is owned by the file that wrote it. `x.rs` saying "I call z"
  stays true after `z()` is deleted; only `x.rs` can withdraw it.

So:

| fact | on removal |
|---|---|
| node `z`, no remaining claimant | DELETED |
| edge `x -> z`, target deleted | UNRESOLVED — `target_id` NULL, `target_name` kept |
| edge `x -> z`, source `x()` deleted | DELETED — its only claimant withdrew |
| edge with other files' occurrences | only the withdrawing file's key is removed |

"x calls something named z that we cannot place" is true and is kept. "x calls z"
when neither exists is not, and goes.

**ORDERING IS LOAD-BEARING. A plain `DELETE FROM nodes` violates the table
above.** R10.2 records the DDL: `edges.source_id`, `edges.target_id` and
`nodes.parent_id` are all `on delete cascade`. So deleting node `z` DELETES
every inbound edge — the exact opposite of row 2 — and takes `z`'s child nodes
with it. Two clauses close it, and neither is optional:

- **(i) Unresolve before delete, same transaction.** `UPDATE sensei.edges SET
  target_id = NULL WHERE target_id = $z` — with `target_name` already
  populated — runs BEFORE `DELETE FROM sensei.nodes WHERE id = $z`. After the
  update no edge references `z`, so the `target_id` cascade has nothing to take.
  `target_name` must be verified non-null before the update, not after: once the
  node is gone there is nothing left to recover the name from.
- **(ii) Children are released by their own claims, never by the cascade.** A
  member of a deleted type is declared by the same file, so the same reconcile
  pass releases its claim and it is deleted on its own terms — with its own
  inbound edges unresolved first. The `parent_id` cascade must therefore find
  NOTHING left to delete. Assert that: count children of `z` immediately before
  the delete and require zero. If the cascade ever fires, it has silently taken
  a member's inbound edges with it, which is (i)'s failure one level down.

Deleting a node whose inbound edges were not unresolved first is the single most
likely way this corrected design still destroys data, and it is the same class
R10.2's blast-radius measurement sized: 1,485 declarations, 3,070 pairs, worst
case 67 files behind one identity.

### What remains genuinely undetectable, and why that is now acceptable

One residual, stated once here; the measurements behind it are in R10.3 and are
not repeated. A file that is DAMAGED but still PARSES is indistinguishable from
one that was legitimately edited — no gate separates them in either direction.

Under R10.8 that is RECOVERABLE rather than fatal, and recoverability is the
whole argument: the next good index of the SAME FILE re-declares the symbol
under the same fqn, and the inbound edges — unresolved by clause (i) with their
`target_name` intact — re-resolve to it. No other file has to be re-indexed.
Demotion had no equivalent property, because the node it left behind still
satisfied `target_id IS NOT NULL` and so was never revisited by anything.

The brake still applies on top: a file going from ≥1 claims to zero triggers one
confirming re-read before any removal.

## 7c. R11 — a gap is DATA, and every gap has a fill path

The pipeline's output is not "the graph, plus some things we could not work out".
It is one dataset in which what is UNKNOWN is recorded as explicitly as what is
known, with a route to closing it. Every mechanism in this spec already works
that way and R11 states it once:

| unknown | recorded as | closed by |
|---|---|---|
| a reference we cannot place | `Unresolved { reason, evidence }` | a later parse, or a resolver rung |
| a symbol not yet declared | completeness = PARTIAL | its file being parsed |
| a file that will not parse | `skip_reason` + detail | the file being fixed |
| a node whose file is untracked | completeness = ORPHANED | re-scan, or pruning |
| packages with no library | `library_packages` empty for them | R11.1 below |

### R11.1 — three ways a grouping arrives, and provenance distinguishes them

1. **Declared by the dependency** — `sensei.library.json` at its root (R10.7h).
   Authoritative, and it also brings skills, agents and the docs corpus.
2. **Declared by the user** — "these crates are the `gateway` library, this is
   its root url". Equally declared, just locally. A UI surfaces the gap and takes
   the answer; it is never asked to confirm a guess the system made.
3. **Absent** — the packages stand alone. Complete, ungrouped, not wrong.

PROVENANCE IS RECORDED ON THE FACT, not implied by which table it landed in. A
user-supplied grouping and a manifest-supplied one must be distinguishable
forever, because they differ in authority: the manifest is the dependency
speaking about itself, the user is a local judgement that may be wrong or may be
right in a way upstream has not yet said. `library_kind` already carries
`detected | imported` and is the natural home.

This does NOT weaken the no-inference rule (R10.7f). A user stating a grouping is
a declaration. The system inferring one from `@rokkit/*` is a guess. The
difference is not confidence, it is who is answerable for it.

### R11.2 — closing the loop upstream: DEFERRED, and the measurement says why

The idea: when a user fills a grouping, emit the `sensei.library.json` the
dependency would have shipped, and offer to open an issue or PR on its
repository. The gap stops being a local workaround and becomes a contribution.

**MEASURED, and it does not hold up yet.** All 1,121 rows in `sensei.libraries`
are `kind = 'detected'`, and **exactly 2 of them carry any URL at all** —
`homepage_url`, `docs_url` and `base_url` are populated on 2 rows each. There is
no repository to file against for 1,119 of the 1,121. The upstream half of this
loop has no input.

So it splits, and only the first half is in scope:

- **IN — emit the manifest locally.** A user-filled grouping is written as a
  `sensei.library.json` the user owns. Costs nothing beyond serialising what
  R11.1 already records, and it is what makes the grouping portable.
- **DEFERRED — contact the dependency's repository.** Needs a repo URL we do
  not have for 99.8% of packages, and it is the only action in this spec that
  reaches a THIRD PARTY. That is a materially different class from everything
  else here and does not get carried in on the back of a metadata feature.
  CAPTURED NOW: provenance on every grouping (R11.1), which is the input a
  later upstreaming pass would need.

Populating `homepage_url` from the manifests stage 2 already reads is the
prerequisite, and it is a `parse_dependencies` field, not a new subsystem.

### R11.3 — the gap queue is a first-class view

"What does this graph not know, and what would fix it" must be answerable in one
query, per gap kind, with counts. Measured examples that exist right now:

- 18,450 PARTIAL nodes — closed by parsing their files
- 8,147 ORPHANED nodes — a PRE-R13 population, closed once by the migration
  sweep, after which the state is unrepresentable and this row is dropped
  (R10.7d)
- 1,121 packages with no library — closed by R11.1
- 431 references naming an fqn no declaration mints, of which 100 are closed by
  the reach rule (§2.1), leaving **331** as the separate defect
- unresolved references by reason code

That view is the work queue, for a person and for an agent. It is the same data
G2 needs to show where a repository is weak, and the same data an agent needs to
decide whether an answer it just received can be trusted.

## 7d. R12 — the containment model, drawn

    folders ──< scan_state ─────────────── THE FILE ENTITY
       │        (folder_id, file_path, mtime, content_hash,
       │         indexed_at, skip_reason)
       │
       └──< nodes ──┬── parent_id (self: type owns member)
                    │
                    ├── file_path TEXT ······> scan_state      [1] string, not a key
                    │
                    └── fqn segment 2 ·······> library_packages.package_name
                                                     │          [2] the join, unpopulated
              edges (source_id, target_id) ──> nodes

    libraries (id, kind, name, ecosystem, version, base_url, docs_url, …)
       ├──< library_packages     (package_name, library_id, source)      0 rows
       ├──< library_skills       (library_id, name, focus, body, origin, scope, …)   10
       ├──< library_agents       (library_id, name, focus, body, origin, scope, …)    6
       ├──< library_pages        (library_id, title, url, content, component, …)    130
       ├──< referenced_libraries (folder_id, library_id, version_used)
       └──< project_libraries    (library_id, project_id, enabled)

A node's container is a FILE or a LIBRARY CONTENT ITEM, and both chains already
exist:

    node -> file -> folder                    first-party
    node -> package -> library -> content     external

### What is already right

- `library_packages` is EXACTLY the grouping: `package_name` TEXT keyed to a
  `library_id`, joinable to the fqn's package segment with no new column. It even
  carries `source` — provenance, which R11.1 requires.
- `library_skills` and `library_agents` are already near-identical:
  `library_id, name, focus, body, source, source_path, version_range, origin,
  scope`. They differ only in that skills add `tokens`/`generated_at`. Two tables
  for one shape.
- `referenced_libraries (folder_id, library_id)` already answers "which libraries
  does this repo use".

### The two defects

**[1] `nodes.file_path` is TEXT, not a key.** Every join to the file entity is a
string match on `(folder_id, file_path)`. It works, but nothing enforces that the
file exists — which is precisely how 8,147 ORPHANED nodes came to name files the
scanner does not track. A key would have made that state unrepresentable.

**[2] `library_packages` is empty**, so the external chain terminates at the
package and never reaches the library, its pages, its skills or its agents.

### CONTENT TYPE — the generalisation worth making

`skill | agent | page | package` are all things a library CONTAINS, and three of
them are already the same shape. A single `library_content` with a type
discriminator would mean a new kind — an mcp server, a plugin, an example — costs
a row rather than a table, and the R11.3 gap queue stays uniform instead of
growing a case per kind.

Not urgent, and it is a migration of live data (146 rows). Recorded because the
shape should be decided BEFORE `library_packages` is populated and a fifth table
joins the pattern.

### Why this matters beyond tidiness

With [1] and [2] closed, one query answers: given this call site, what library
does it reach, which of that library's components document it, and which skills
and agents exist for it. That is R10.7g, and it is two links away — not a new
subsystem.

## 7e. R13 — rename `scan_state` to `files`, and give it a key

The name is wrong and it caused a design error. `scan_state` reads as transient
bookkeeping, so the question "do we have a file entity?" stayed open for several
rounds of this design while the answer sat in that table. A table called `files`
would have made the entity obvious, and would have made the missing surrogate key
obvious with it.

It is the file entity: `(folder_id, file_path, mtime, content_hash, indexed_at,
modified_at, skip_reason)`, one row per file.

**The three file populations, each measured and labelled** — this document
previously used 48,665, 48,654 and 47,904 interchangeably, which is one
experiment reported at three sizes:

| population | count | what it is |
|---|---:|---|
| file rows | **48,665** | every `scan_state` row — the walk's output |
| indexed | **48,646** | `skip_reason IS NULL` — the set that gets parsed |
| skipped | **19** | 17 `binary_content`, 2 `invalid_utf8` |

Cite the one that matches the claim. Parse cost is over the 48,646; the
per-file id lookup is over all 48,665, because the walk creates every row.

### Do the rename and the key in ONE migration

They are the same change and splitting them means migrating the same table twice.

- **Rename** `scan_state` -> `files`.
- **Add `id uuid`** as a surrogate key. Today the PK is `(folder_id, file_path)`
  and there is no id, which is why `nodes` references a file by TEXT.
- **Point `nodes.file_id` at it**, replacing `file_path` on the node. Measured,
  this wins on every axis:

  | | `file_path` (today) | `file_id` |
  |---|---|---|
  | file rename | rewrite EVERY node in it — 7.4 avg, 1,150 worst | 1 row |
  | ORPHANED state | representable — 8,147 exist | impossible, it is a FK |
  | storage | 47 bytes x 354,653 = 16 MB of duplicated text | 16 bytes, ~5.7 MB |
  | `nodes_unique_identity` | carries a 47-byte column | 16 bytes |

  **354,653 is every node carrying a path** — COMPLETE (346,506) plus ORPHANED
  (8,147). Both store the text, so both count toward the duplication, which is
  why this figure is larger than R10.7d's COMPLETE row.

  The rename figure decides it. Today renaming a file rewrites the path on every
  node it contains. R10.5 rules that a rename is a deletion plus an addition and
  needs no case of its own, which is correct for Rust — the module path IS the
  file path — but it means every node in the file is destroyed and recreated.
  With `file_id` a rename is ONE update to the file row and every node follows,
  because none of them stored the path.

  The write-side cost is one path->id lookup per FILE (48,665 — one per file
  row, since the walk creates them all), not per node (354,653), and the file
  row is being upserted at that moment anyway so the id is already in hand.
  Readability is not an objection: the view (R12) exposes the path.

  It also makes the 8,147 ORPHANED nodes UNREPRESENTABLE — a node cannot name a
  file that does not exist when the reference is a foreign key. The state stops
  needing detection because it stops being possible.

### The file row is created by the WALK, never by node persistence

Persisting a node must LOOK UP `file_id` and FAIL CLOSED if it is absent. It must
NOT get-or-create.

Get-or-create is the obvious convenience and it defeats the entire purpose of the
key. A node naming an untracked file would simply create one, producing a phantom
`files` row with no `mtime`, no `content_hash` and no `indexed_at` — a file that
was never scanned. That is the 8,147 ORPHANED problem again, one table over, and
worse because the foreign key now certifies it.

The ordering falls out of who holds the facts:

    WALK          discover the file set, create ALL the file rows   <-- barrier
      |
      +-> enqueue one parse task per file
            |
            +-> PARSE   read, resolve, persist nodes with file_id already known

The file rows are created by the walk BEFORE any parse task is enqueued. Four
things follow, and three of them are problems that simply do not arise:

1. **No race.** Parse tasks run concurrently. If each did get-or-create, two
   tasks touching one file would race to insert it. Creating the rows upfront,
   single-threaded, removes the race rather than locking around it.
2. **The denominator is free.** At that barrier the walk knows the complete
   post-filter file set — which is exactly `folders.props.expected_files`, the
   denominator `sensei.folder_completeness` already divides by (R10.7b) —
   obtained without a second count.
3. **A stalled parse is visible.** A file row with no parse outcome is a task
   that never ran. Today that is indistinguishable from a file that does not
   exist.
4. **Order independence holds** (R6). Parse tasks can complete in any order
   because none of them creates shared state; they only fill in their own row
   and write their own nodes.

Only the walk opened the file, so only the walk can state its facts. By the time
nodes are written the row exists, and a lookup that misses means the pipeline is
broken — which it should say, not paper over (R4).

A FILE THEREFORE HAS ITS OWN LIFECYCLE, distinct from the node completeness
states of R10.7d (which describe symbols):

    discovered  -> the walk created the row; no parse outcome yet
                -> parsed        the declarations are current
                -> unparseable   skip_reason + detail (R10.7, R10.9)
                -> skipped       binary, non-UTF8, excluded by config

`discovered` with no successor is a stalled or lost parse task, and it is
actionable in the same way an unparseable file is.

There is no legitimate exception. `file_id` is set only on DECLARATIONS, and a
declaration exists only because its file was walked. PARTIAL and EXTERNAL nodes
carry `file_id` NULL by definition (R10.7d), which is a different thing from a
missing lookup.

Keep `(folder_id, file_path)` as a UNIQUE constraint. The table then carries two
keys serving two different callers, and neither is redundant:

| key | who uses it |
|---|---|
| `id` (surrogate) | `nodes.file_id` — 16 bytes, stable across a rename |
| `(folder_id, file_path)` | the walk, which only ever has a path |

The path lookup is already an index scan today — verified, `EXPLAIN` on
`WHERE folder_id = $1 AND file_path = $2` gives
`Index Scan using scan_state_pkey`. Preserving that constraint through the rename
keeps the walk's once-per-file lookup (48,665 per full scan) at O(log n) with no
additional index.

### Cost, measured

134 references across 10 Rust files, plus `design.dbml` and the
`folder_completeness` view. Real but bounded, and mechanical.

It is DDL on a table the SHIPPED indexer writes on every file of every scan, so it
goes through the dbd workflow, not by hand, and it is the largest schema change
this design asks for. Sequence it with everything else the schema needs so the
tables are opened ONCE — see §7h.

**EVERY `scan_state` AND `file_path` REFERENCE EARLIER IN THIS DOCUMENT IS
PRE-R13 AND CHANGES HERE.** Those sections describe the state as MEASURED
today, which is why they use the old names; they are not stale, they are
historical. At this migration:

| earlier text says | after R13 |
|---|---|
| the `scan_state` table (R10.7, R10.7d's join, R12's diagram) | `sensei.files` |
| `nodes.file_path` (§2.1, R10.1, R10.2, R10.6(c), R10.7d, R10.7e) | `nodes.file_id`, an FK |
| `file_path IS NULL` as the PARTIAL/stub predicate | `file_id IS NULL` |
| "no `scan_state` row for it" (ORPHANED) | unrepresentable — the FK forbids it |

The path is still readable through the view (R12) or from the `files` row —
never off a node. Note the ORDER within stage 0: the ORPHANED sweep runs
AFTER the rename and BEFORE the foreign key, so it targets `files` even
though the 8,147 were counted against `scan_state`. Same population, new
name. See `docs/spec/indexer/00-files-entity.md`, "Execution order".

### Why bother, when the join works today

Because the string join is what allows the broken state. 8,147 nodes currently
assert a declaration in a file the scanner does not track, and they read as
COMPLETE. No amount of care in the writer prevents that; a foreign key does. This
is the same argument as R9 — a failure that the type system makes impossible
beats one that a reviewer has to notice.

## 7f. R14 — the pipeline has two modes, and both create structure before work

### Full — process repo

    glob all folders and files (post-filter)
      -> create the folder rows and the file rows      <-- structure barrier
      -> enqueue one parse task per file

Structure first, work second. The barrier is what gives R14 its properties: no
parse task creates shared state, so none can race another, and the complete file
set is known at that instant, which IS `folders.props.expected_files` (R10.7b)
with no second count.

### Incremental — a change arrives

A change is detected as OLD plus NEW, and that pairing is what makes it
classifiable rather than guessable:

| detected | file/folder rows | reparse? |
|---|---|---|
| content changed, path same | touch `mtime` + `content_hash` | YES |
| path changed, content same | UPDATE the path, `id` unchanged | see below |
| path + content changed | update both | YES |
| added | create the row | YES |
| removed | delete the row, cascade | no — R10.8 reconcile |
| touched, hash identical | refresh `mtime` only | NO |

Content decides reparse. `content_hash` already exists on the file row and
already gates this today (`plan_reindex`).

### What a rename actually costs, and what it does not

`file_id` removes the node churn that a path change used to cause: nodes never
stored the path, so renaming a file rewrites ONE row instead of every node in it
— 7.4 on average here, 1,150 in the worst case.

But it does NOT always make a rename free of reparsing, and R10.5's ruling —
that a rename is a deletion plus an addition — is why. In Rust and in TS/JS the MODULE
PATH is derived from the file path, and the module is a segment of every fqn the
file declares. So:

- rename that CHANGES the module path -> every declaration's identity changes ->
  the file must be re-minted even though its bytes are identical
- rename that does NOT (a case-only change, a move that preserves the module) ->
  update the file row and stop

So the honest saving is: the file and folder rows update trivially and the node
rows stop carrying a redundant path, but identity still depends on location, so a
module-changing rename is a re-mint. That is a property of the FQN grammar, not
of the storage choice, and it is the price of an identity a reference can
independently compute (R2).

## 7g. R15 — the two-stage scan, and what git can and cannot tell us

    SCAN ROOT   glob for .git entries, apply root exclusions  -> repo roots
                  |
                  +-> per root:
    SCAN REPO   read .gitmodules       -> submodules (declared)
                  |                       each enqueues its OWN scan_repo
                read manifests        -> commands, deps, workspace members
                glob files and folders, apply filters + .gitignore
                create the folder rows and the file rows    <-- structure barrier
                  |
                  +-> enqueue one parse task per file

Two stages, each doing one thing: find the repos, then — per repo — find
everything inside it. Structure is complete before any parse task exists (R14).

### Submodule: yes, from git — and it is SCAN REPO's job

Declared in `.gitmodules` at the repo root, and the submodule's directory holds a
`.git` FILE (not a directory) pointing into the parent's `.git/modules/`. Both
are working-tree facts and DECLARED rather than inferred.

**It belongs to `scan_repo`, not `scan_root`**, and an earlier draft of this
section placing it at scan root is withdrawn. `.gitmodules` is a file INSIDE a
repo; reading it at scan root would mean scan root opening repo contents, which
is the one thing the two-stage split exists to avoid. A submodule IS a repo
root, so `scan_repo` enqueues a child `scan_repo` for each one it declares —
the root set grows during the scan-repo wave rather than being fixed by scan
root.

That has one consequence worth stating, because the incremental path depends on
it: **the complete root set is known only after the scan_repo wave drains, and
`match_repos` (R14, incremental) reads it from the DATABASE, never from scan
root's output.** Longest-prefix matching a changed path against known roots is
therefore correct for submodules — `repo/submodule/x.rs` belongs to the
submodule — because the submodule has its own persisted root row by then. A path
matching no known root is what escalates back to `scan_root`.

### Subtree: NO, and a folder-level scan would not help

A subtree is ordinary files in the parent repo. There is no `.gitmodules` entry,
no nested `.git`, no marker of any kind in the working tree. Verified here:
`homebrew/` and `marketplace/` are subtrees and are indistinguishable from any
other directory.

GIT CANNOT BE ASKED. `git subtree` exposes only `add | merge | split | pull |
push` — there is no list or query subcommand. The only trace is in COMMIT
HISTORY, and every way of reading that history is wrong in a different
direction. Measured, all three against this repo, whose real subtrees are
`homebrew/` and `marketplace/`:

| method | returns | how it fails |
|---|---|---|
| `git log --grep='git-subtree-dir'` | 3 | FALSE POSITIVE — matched a doc commit that merely writes about subtree detection |
| `%(trailers:key=git-subtree-dir)` | 5 | STALE — `daemon`, `docs`, `gateway` are historical; `git subtree split` writes the same trailer, and `gateway` has since moved out to its own repo |
| `Squashed '<dir>/'` subject | 2 (correct) | only catches `--squash` adds; a non-squashed subtree never produces it |

No combination repairs this. A directory that still exists and once carried a
trailer is indistinguishable from a live subtree, BECAUSE THERE IS NO DIFFERENCE
IN THE WORKING TREE. History says a subtree operation happened once; it cannot
say the arrangement still holds.

The raw evidence:

    5d998336 Squashed 'marketplace/' content from commit 10ea796
    4a8e7ed4 Squashed 'homebrew/' content from commit 4f2e246

So adding a folder-level scan does not solve it — there is nothing in the folder
to find. History archaeology (`git log --grep='git-subtree-dir'`) is the only
route and it is LOSSY: a squash or a rebase drops the trailer, and a subtree
added by hand never had one.

Treat subtree detection as best-effort enrichment with a provenance of
"inferred from history", never as a structural fact the graph depends on. If it
matters, it belongs in the same class as library grouping (R11.1): declared by
the user when the heuristic misses.

### What a folder-level pass IS worth doing for

Not subtrees — NESTED MANIFESTS. A `Cargo.toml` with `[workspace] members`, a
`package.json` with `workspaces`, a `pyproject.toml`. These are declared,
reliable, and they are what actually define package boundaries inside one repo
— which is the `package -> module -> item` slice of R10.7c and the input to
`library_packages` (R10.7h).

## 7h. ALL DDL happens ONCE, in stage 0, before any v2 code

Two plans disagreed about this and the disagreement was not cosmetic.
`docs/plans/indexer-v2-rust.md` deferred every schema change to its step 9, at
the END. `docs/plans/indexer-v2-sequence.md` — the master plan — puts the files
migration at stage 0, at the START. **Stage 0 wins, and it takes ALL the DDL
with it.**

The deferral was not free. Three mechanisms in this document exist ONLY because
the schema was going to change later, and each is a workaround for a column that
stage 0 now creates before a line of v2 code is written:

| mechanism | exists because | under stage 0 |
|---|---|---|
| the `parameter` kind placeholder (§2.1) | `node_kind` has no `field` / `variant` | **DELETED** — write the real kind |
| R10.1's two-disjunct edge predicate | no gin index on `props->'occurrences'` | **DELETED** — one clean occurrence test |
| `nodes.props.claims` in jsonb (R10.4) | — | **KEPT, deliberately.** A claim set is a variable-length set keyed by path; `props` is the right home whether or not the schema is open. This one was never a deferral. |

Writing a workaround for a constraint you are about to remove is pure waste, and
worse, the workaround outlives the constraint — the `parameter` placeholder is
already in committed code at `indexer/persist.rs`, and twelve comments across
the v2 sources cite "step 9" as the moment it goes away. A step that no longer
exists.

**What stage 0 owns, in one dbd pass:**

1. `scan_state` -> `files`; add `id uuid`; keep `(folder_id, file_path)` UNIQUE
   (R13).
2. `nodes.file_path` -> `nodes.file_id`, FK to `files.id` (R13). Sweep the 8,147
   ORPHANED rows as part of the migration — after the FK they are
   unrepresentable (R10.7d).
3. Add the parse-failure DETAIL column to `files` — the parser's verbatim
   message with line and column (R10.7, R10.9, A9).
4. Widen `sensei.node_kind` for the kinds the walk actually produces (field,
   variant, …), retiring the `parameter` placeholder.
5. Widen `sensei.edge_kind` for the relation kinds R8 needs.
6. `create index … using gin ((props->'occurrences'))` on `edges` (R10.1).
7. Rename `project_commands` -> `folder_commands` (D11).

All seven touch tables the SHIPPED indexer writes, so all seven go through the
dbd workflow rather than by hand, and doing them together means those tables are
opened once instead of seven times.

## 8. Decisions

**D1.** Scope is the walk AND the persistence path (see R3).
**D2.** Parameters are typed props on their function, not nodes.
**D3.** Externals are indexed by USE, never by internals (see R5).
**D4.** Language order: rust, js, ts, svelte.
**D5.** OO structure comes from the same walk; patterns are derived from nodes
and edges.
**D6.** The fact set is derived from the patterns and fixed before the walk is
considered complete (see R8).
**D7.** The trailing segment of an fqn is the REACH, not the namespace:
`item`, `field`, `macro`, `mod` (see §2.1). Rust's type and value namespaces
collapse into `item` because a path leaf does not state which it is, and a
declaration may occupy both. A7 is what makes that safe.
**D8.** Reconcile DELETES a node that has no remaining claimant, after
unresolving its inbound edges in the same transaction (R10.8). An UNPARSEABLE
file removes nothing at all and marks its contents DIRTY (R10.7). **Demotion was
rejected** — it left an edge whose `target_id` was set pointing at a node naming
nothing, which every consumer reads as resolved (67,839 such edges measured),
and it could never fire on the deletion it was designed for. See §7b. The
cascade risk that argued for demotion is real and is handled by ordering, not by
avoidance: R10.8 clause (i).
**D9.** A file's claim on a declaration is a KEY in `nodes.props.claims`, exactly
as its contribution to an edge is a key in `edges.props.occurrences` (R10.4).
Both because two files can state one thing, and `file_path` is one column.
**D10.** ALL DDL is done ONCE, in stage 0, before any v2 code is written — see
§7h. The alternative (defer schema changes to the end) was tried in the first
plan and cost three interim mechanisms that exist only to work around a schema
that was going to change anyway.
**D11.** **A manifest sits AT a folder, so every fact read out of one is
FOLDER-grained at the base, and repository- and project-level answers are
VIEWS over it.** Storing an aggregate is how the aggregate comes to disagree
with what a manifest actually said. Applied table by table in stage 0:

| table | today | ruling |
|---|---|---|
| `project_commands` | keyed `folder_id`, no project ref | **rename `folder_commands`.** 572 commands span **58 folders but only 36 repositories** — a repo key collapses 22 folders' sets and loses which directory each `build` runs in. |
| `project_dependencies` | PK `(from_project_id, to_project_id, from_folder_id, source_protocol, source_manifest)`, **0 rows** | **regrain to `folder_dependencies`**, keyed on the folder pair. `from_folder_id` is already present; the project ids are pre-aggregation in the key and are DERIVABLE from the folders. Zero rows, so this is free. |
| `referenced_libraries` | keyed `(folder_id, library_id)`, `version_used`, **1,016 rows / 49 folders** | **already correct — this IS the folder-grained library fact.** Its comment already says "the pinned version observed in the folder (from package.json, Cargo.toml, …)". Do not create a `folder_libraries`; it exists under this name. |
| `project_libraries` | `(library_id, project_id, enabled, props)`, **1,299 rows**, `project_id NULL` = GLOBAL | **NOT regrained.** It is a user-curated ENABLEMENT layer derived from `referenced_libraries` ("editable by user"), and a library enabled globally or per project is inherently not folder-grained. Its name is still poor — it reads as an observation and is a toggle. Rename to `library_enablement`; `project_id` stays as the scope column. |
| `project_metrics` | — | already renamed `repository_metrics`. Precedent for this class of rename. |

Repository- and project-level dependency and command answers are VIEWS over
the folder-grained tables, following `sensei.folder_completeness`, which
already recurses `parent_id` to roll per-file facts up. Derived beats stored:
a view cannot drift from what it derives from.

**D12.** **`node_kind` loses `lib_symbol` and `lib_package`.** A node's KIND
says what it is; its FQN PREFIX says where it comes from. Those are orthogonal
axes and the `lib_` values conflate them.

Measured, and the redundancy is total: **21,928 nodes carry a `lib_` kind and
all 21,928 have a `lib·` fqn; ZERO nodes of any other kind have one.** The
prefix is a perfect discriminator in both directions, so the kind carries no
information the fqn does not — and R10.7d already rules that "the
discriminator is already in the key … so no new column is needed to tell the
two containers apart."

It is not merely redundant, it DESTROYS information. **18,240 `lib_symbol`
rows** say only "external", not whether the symbol is a function, a type or a
const. First-party symbols keep that distinction; external ones lose it, for
no reason.

And both discriminators are ALREADY IN USE — `kind IN ('lib_symbol',
'lib_package')` in `db/pg_store/graph.rs`, `fqn.starts_with("lib·")` in the
language modules and their tests. That is two representations of one fact,
which R10.7e forbids in the paragraph next door.

The replacement:

| today | becomes |
|---|---|
| `lib_package` | `package` — an existing value, and unambiguous |
| `lib_symbol` | the REAL kind where the use site reveals it (a call site implies `function`, a type position implies `type`), else `unknown` — the value stage 0 adds anyway |
| "is it external?" | `fqn LIKE 'lib·%'`, the single discriminator |

`unknown` is the honest answer when the use site does not reveal a kind: R5
says we index the USE of a dependency and never its internals, so sometimes we
genuinely cannot tell. That is a gap with a reason (R11), not a category.

NO new `is_external` column. `nodes` has none today, the fqn already answers
it, and adding one would store what is derivable — the failure R10.7e names.
`nodes_unique_fqn` is `btree (folder_id, fqn)`, so a folder-scoped prefix test
still uses the index for the folder equality; the lookup does not regress.

**Cost, measured and NOT small: 111 references across the tree** — heaviest in
`db/pg_store/{tests,graph}.rs`, `tasks/handlers/process.rs`, `graph_facts.rs`,
`languages/import_target.rs` — plus `nodes.ddl`, the `graph_nodes` and
`edge_resolution_class` views, and a 21,928-row backfill. Larger than the rest
of stage 0's enum work put together, and it is a code migration rather than
pure DDL. It is sequenced there anyway because the alternative is letting the
v2 walk write `lib_symbol` and migrating twice.
