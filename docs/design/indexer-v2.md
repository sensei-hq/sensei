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

**How it says so, given `sensei.node_kind` has no value for it.** The column is
`not null` and widening the enum is a DDL change to a table the shipped indexer
writes, so it is step 9's (§7). Until then an `item` stub carries ONE
placeholder, chosen here rather than per use site: `parameter`. Not a plausible
`type` or `function`, because a plausible value is one no caller can tell from a
real reading, and that is the one thing a failure path may not produce. This one
is distinguishable twice over — v2 never mints `parameter` for a declaration (a
parameter is a prop, D2) and no query in the tree selects it, so nothing counts
a stub as a classification it does not have. The row is additionally marked as a
stub by the schema's own means (`resolved = false`, null `file_path`), and the
kind is replaced wholesale when the declaration lands.

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
re-indexed. Reconcile must not use that predicate, and step 9 must not keep it.

Finding F's edges without a jsonb scan. Every edge F contributes to is sourced
at a symbol F declares or at F's own module identity — with **26 exceptions out
of 119,248 edge sources** (0.02%), all of them split-impl anchors
(`impl PgStore` in `commands.rs` anchors at
`rust·senseid·db::pg_store::commands·PgStore·item`, which no file declares —
the gap §2.1 records). So the affordable predicate is

    source.fqn = ANY(claims(F) ∪ {file_fqn(F)})
      OR (source.resolved = false AND props->'occurrences' ? F)

The second disjunct is the correctness clause and it scans only stub-sourced
edges. `create index … using gin ((props->'occurrences'))` would collapse both
into one clean test; it is DDL on a table the shipped indexer writes, so it is
step 9's (§7). The interim predicate must be proven a SUPERSET of the occurrence
key over the corpus, never a convenient narrowing of it.

### R10.2 Removal is DEMOTION, never deletion

A definition F no longer declares does not lose its node. The node is demoted to
a stub: `resolved = false`, `file_path = NULL`, `kind` back to the placeholder
§2.1 names, and every definition-only column and prop CLEARED — `line_start`,
`line_end`, `docstring`, `signature`, `is_exported`, and the props
`symbol_kind`, `visibility`, `declared_type`, `params`, `span_columns`. Clearing
them is not tidiness: a row that says `resolved = false` while still carrying a
`line_start` answers "where is X defined" with a line in a file that no longer
defines it, which is a fabricated reading (R4).

**Inbound edges are left exactly as they are.** They keep pointing at the same
node id, which now states that nothing declares it. That reading is true, and it
is the same shape as an edge to a target that has not been indexed yet — which
is deliberate, because R6 forbids a graph that can tell those two apart by scan
order, and A4 already admits the shape.

Three reasons demotion and not deletion, in order of weight.

1. **Deleting is not reversible and the input is not trustworthy.** R10.3 shows
   that a damaged parse cannot be detected. A demotion taken on a bad parse
   costs a few minutes of `resolved = false` and is undone by the next healthy
   index of the same file, which re-promotes the same node by fqn. A deletion
   taken on a bad parse destroys every inbound edge from every file that is not
   being re-indexed, and nothing re-creates them until each of those files is
   itself re-indexed.
2. **The blast radius is measured and large.** 1,485 of 12,519 declarations have
   inbound edges from a file other than their own — 3,070 (declaration,
   referring file) pairs — and the worst is
   `rust·senseid·db::pg_store·PgStore·item`, referenced from **67 files**. A bad
   parse of one `mod.rs` would take all 67 files' inbound edges with it.
3. **A node deletion cascades in BOTH directions and counts nothing.** The DDL
   is explicit: `source_id … on delete cascade`, `target_id … on delete
   cascade`, and `nodes.parent_id … on delete cascade`. There is no version of
   "remove the node, keep the edges".

Reconcile therefore performs exactly one kind of deletion: an EDGE ROW whose
occurrence object became empty. It never deletes a node, so it never cascades.

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

- Truncating each of the 294 corpus files that declare ≥10 symbols to half its
  bytes produced **0 `ReadError`s**. Symbols fell 12,167 → 6,973 (57% kept),
  references 109,328 → 53,524. Per-file retention: min 0%, **median 52%**, max
  100%. Worst single file: 354 symbols → 187.
- `""`, `"fn main() {"` and `"}}} not rust @@@"` all return `Ok` with 0 symbols,
  0 references, 0 relations and 0 imports — identical to a genuinely empty file.
- The obvious gate does not work in EITHER direction. `tree.root_node()
  .has_error()` is true for **6 of 372 intact, valid corpus files** and false for
  **53 of 366 truncated ones**. Gating on it would refuse to ever reconcile six
  healthy files and would wave through fifty-three damaged ones.

So reconcile does not try to decide whether a parse was good. Two rules replace
the detector it cannot have.

1. **Removal is demotion (R10.2).** Being wrong is survivable, which is the
   property that makes an undetectable input acceptable at all.
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
half demote to stubs. That is the residual, it is named rather than papered
over, and it is bounded by R10.2 — the graph says "no definition known", which
is true of what it can see, and the next healthy index restores it. A threshold
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
`rust·senseid·base_url·item` and `rust·senseid·main·item`, both already on A7's
known list of 15. Without a claim set, reconcile cannot tell "F was the only
declarer" from "F was one of two", so releasing F's claim would demote a node
another file still defines. A node therefore carries

    props.claims = { "<file path>": true, … }

merged with the same `jsonb_set` idiom the occurrences use, for the same reason
(plain `||` on `props` would replace the whole object). No DDL: `props` is the
column the schema documents as extensible and where every other v2 fact that has
no column already lives.

The rule: reconcile removes F's key from `props.claims`, and the node demotes
only when `props.claims` is empty. When a claim remains but the released one was
the one in the `file_path` column, the node's definition columns are now stale
and are cleared anyway — the row honestly states "no definition known" until the
surviving claimant is next indexed and re-promotes it. Reconcile does not pick a
winner and does not invent the survivor's columns, which it does not have.

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
> and every identity that only F previously stated survives as a stub that
> states nothing.**

Three clauses, each independently testable, and in this order:

- **(a) F's contribution is exactly the new facts.** The set of nodes whose
  `props.claims` contains F equals `facts.symbols`' fqns, and the set of
  `props.occurrences -> F` lists equals what the grouping of `facts` produces.
- **(b) No other file's contribution moved.** Read the folder back before and
  after with F's contribution projected out; the two must be equal. This is the
  clause that catches a cascade, a source-scoped edge delete, and a
  `file_path`-keyed node delete — all three of which pass (a).
- **(c) What F alone dropped is a stub, not a hole.** Every fqn in
  `previous_claims(F) \ facts.symbols` still has a node, with `resolved = false`,
  `file_path = NULL`, no definition column and no definition prop set.

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
need is captured now, because re-parsing 48,654 files to recover a field we
already had is the expensive version.

## 6. Acceptance

Thresholds, not comparisons.

- **A1.** Import-mediated references resolve at ≥99.9%. An import names its
  target; failing here means the grammar is misread.
- **A2.** Zero references dropped. Count of `Reference` values equals count of
  use sites in the AST, verified by a walk that counts independently.
- **A3.** Every `Unresolved` carries a reason, and the reason histogram accounts
  for 100% of them.
- **A4.** Zero edges point at a symbol that does not exist as a definition
  somewhere, unless the target is `lib·`.
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
- **A8.** Re-indexing a file removes what it stopped claiming and nothing else.
  R10.6's three clauses hold over the corpus for: a file that lost a
  declaration, a file that lost a use site, a file emptied, a file deleted, and
  a symbol moved between two files sharing a module path. Zero rows belonging to
  a file that was not re-indexed change in any of them. This is what makes a
  stale edge a caught failure rather than a fact the graph reports.

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
structures. `Owns` (R10, D5) is that relation for symbols; folders have
`parent_id`; files sit between the two. A node knowing its file makes the whole
chain walkable in one direction and sliceable at any level — which is what makes
"show me this repo by folder" and "show me this package's types" the same query
with a different cut.

The one thing currently in the way: a symbol references its file by `file_path`
TEXT, and every `kind = 'file'` node has an EMPTY fqn, so a file is not
addressable as a merge target the way every other node is. Give the file node an
fqn and claim it like any other declaration, and the chain closes.

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

### R10.7d — COMPLETENESS is a derived state with three values

A node is in exactly one of three states, and the node+file join gives all three
from columns that already exist. Verified on the live graph:

| state | condition | meaning | live count |
|---|---|---|---:|
| COMPLETE | `file_path` set, file has no `skip_reason` | the declaration was parsed | 354,653 |
| PARTIAL | first-party, `file_path` IS NULL | referenced, not yet declared | 18,450 |
| DIRTY | `file_path` set, file HAS a `skip_reason` | was complete, its file now fails | 0 |
| EXTERNAL | origin is `lib·` | a library symbol — NO file will ever exist | 21,928 |
| ORPHANED | `file_path` set, NO `scan_state` row for it | claims a file the scanner does not track | 8,147 |
| EXCLUDED | file's `skip_reason` is not a parse failure | the file is deliberately not indexed | 0 |

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

The three states differ in TRAJECTORY, which is why they must not be conflated:

- PARTIAL becomes complete when its file is parsed. Nothing is wrong.
- DIRTY was complete and will be again. What we hold is the last known-true
  answer, and we say so.
- DELETED is neither. It is gone, and no future parse restores it.

THIS IS EXACTLY WHAT DEMOTION GOT WRONG. It set `file_path = NULL` on a deleted
declaration, making it indistinguishable from PARTIAL — so the graph claimed
"we have not found this yet" about something that definitively no longer exists,
and every consumer treated a dead symbol as a pending one. Under R10.8 a deleted
node is deleted, so the three states stay distinct by construction.

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

MEASURED STATE: this level does not exist. `sensei.libraries` holds PACKAGES —
`@rokkit/actions`, `@rokkit/app`, `@rokkit/blocks`, `@rokkit/core` are separate
rows, all `kind = 'detected'`, `ecosystem = 'npm'`, and there is no `rokkit` row
grouping them. The `library_kind` enum is `detected | imported`, which records
PROVENANCE (found by scan vs added deliberately), not LEVEL. 1,121 rows across
npm/cargo/pypi/nuget/maven, every one a package.

So two things are needed, both additive:
- a level discriminator, so a row can be a library rather than a package
- a parent link from package to library

Neither changes any fqn. `lib·@rokkit/ui·…` stays exactly as it is; the grouping
hangs above it.

THE GROUPING MUST BE DECLARED, NEVER INFERRED. `@rokkit/*` -> `rokkit` is a
tempting prefix rule and it is a guess: `@types/node` belongs to no "types"
library, and the crates of `gateway` share no prefix at all. Grouping comes from
declared metadata (llms.txt, registry, an explicit `add_library`) or it does not
happen. An invented grouping is a fabricated fact about a dependency, and it
would be believed.

This is the `library -> package -> symbol` slice of R10.7c, with the top level
optional — the same shape as `folder -> file -> type -> member`, where a folder
may or may not be a project.

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

### What remains genuinely undetectable, and why that is now acceptable

A file that is DAMAGED but still parses is indistinguishable from one that was
legitimately edited: truncating 294 corpus files to half produced 0 `ReadError`s
and retained 57% of symbols, and `has_error()` is wrong in both directions (true
for 6 of 372 intact files, false for 53 of 366 truncated). No gate separates them.

Under R10.8 that is survivable rather than fatal: the loss is RECOVERABLE — the
next good index restores the declaration and inbound edges re-resolve. Demotion's
failure was not. The brake still applies: a file going from >=1 claims to zero
triggers one confirming re-read before any removal.

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
**D8.** Reconcile DEMOTES a node it no longer has a claim for; it never deletes
one (R10.2). Deletion cascades both ways with no count, and the input it would
act on — a parse that succeeded but read half a file — is measurably
undetectable (R10.3). The only row reconcile deletes is an edge whose occurrence
object is empty.
**D9.** A file's claim on a declaration is a KEY in `nodes.props.claims`, exactly
as its contribution to an edge is a key in `edges.props.occurrences` (R10.4).
Both because two files can state one thing, and `file_path` is one column.
