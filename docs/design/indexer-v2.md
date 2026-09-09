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

**R1. One parse per file.** No stage may re-read source bytes.

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
