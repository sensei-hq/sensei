# Indexer v2 — design before implementation

Status: DESIGN ONLY. No code. Nothing below is implemented.

## Why a rewrite rather than more slices

Every defect found this week is a symptom of three structural properties of the
current design, not of individual mistakes. Slicing cannot remove them because
they are the shape of the thing.

**S1. A file is parsed three times and no pass sees the others' work.**
`crates/senseid/src/tasks/processors/code.rs` calls `adapter.parse`,
`adapter.parse_to_ir`, and `adapter.fqn_output` — three tree-sitter/oxc passes
over the same bytes. The pass that KNOWS a type is not the pass that MINTS the
node. This is the direct cause of "we parse it and throw it away":
- rust `extract_return_type` ran on every function; the value was dropped.
- TS `collect_binding` reads one form (`new Foo()`); every `TSTypeAnnotation` is
  discarded.
- 1,801 rust structs -> 0 field nodes; 1,245 TS classes -> 0 property nodes.

**S2. Resolution returns `Option`, so failure is indistinguishable from absence.**
`resolve_call(..) -> Option<CallTarget>`, and its caller does
`&& let Some(target) = resolve_call(..)`. A `None` emits NO EDGE. Four of seven
bail sites do this, so the call disappears and never enters the unresolved count.
Every coverage number ever quoted for this graph understates the gap by an
unknown amount. A dropped edge produces no failure anywhere, so no test can
catch it — which is why a 3,189-test suite did not.

**S3. There is no spec, so every fix is a local judgement call.**
Bails were added one at a time for good local reasons, each accounting for
nothing it silently cost. Three HIGH wrong-merge defects reached a green suite
this week. Two tests were found actively GUARDING a bug.

## The design

### One parse, one output, total functions

    source bytes
        │
        ├─ parse once  (tree-sitter for rust/java/kotlin/python, oxc for TS/JS)
        │
        └─ ONE walk ─> FileFacts

    struct FileFacts {
        language:   Language,
        package:    Package,
        module:     Module,
        symbols:    Vec<Symbol>,      // EVERY declaration
        references: Vec<Reference>,   // EVERY reference, resolved or not
        relations:  Vec<Relation>,    // extends / implements
        imports:    Vec<Import>,
    }

### The two type decisions that prevent S1 and S2

**No `Option` for resolution.** Resolution is a total enum, so "could not place
it" is a VALUE that must be constructed and carried, never an early return:

    enum Resolution {
        Resolved(Fqn),
        Unresolved { reason: UnresolvedReason, evidence: Evidence },
    }

`UnresolvedReason` is a closed enum — one variant per distinct cause, never
collapsed. `Evidence` carries what the walk SAW (receiver expression, the import
in scope, the declared type it could not place) so a later pass has something to
work with instead of a bare name. A reference cannot be dropped because there is
no code path that produces "nothing".

**Declared types are captured once, at the point of parse.** `Symbol` carries
`declared_type: Option<TypeRef>` — for a function's return, a parameter, a
struct field, a class property, an enum variant payload. Captured by the single
walk, never re-derived, never dropped at a seam. S1 becomes structurally
impossible: there is only one seam.

### What `Symbol` must cover (currently missing entirely)

Fields, properties, enum variants, and parameters are symbols. "What shape is
this data" is a grade-A capability for both goals and is impossible today
because none of these exist as nodes in any language.

### Modularity

    indexer/facts.rs        the types above — shared, language-agnostic, no logic
    indexer/fqn.rs          the key grammar (UNCHANGED — see below)
    indexer/resolve.rs      the shared resolution ladder + reason codes
    indexer/lang/rust.rs    one walk, produces FileFacts
    indexer/lang/<next>.rs  added ONE LANGUAGE AT A TIME

A language module owns only "how do I read this grammar". The output type, the
resolution ladder and the reason codes are shared, so a language cannot invent
its own idea of what a miss means.

## What is kept, and why

Not everything is broken. These are measured working and must be carried over
unchanged — re-deriving them is how regressions get reintroduced:

- **The fqn grammar** `<lang>·<package>·<module>·<Type>·<member>`. It is a lookup
  key minted independently by caller and definition. Changing it has been tried
  twice and failed twice.
- **Import classification.** 141,980 resolved / 4 unresolved. This is the proof
  the approach works when the AST names a target.
- **External mapping** to `lib·<package>·<member>` with no library internals:
  218,310 edges onto 18,240 nodes. Library sources are never needed.
- **Stub-and-merge ordering** (`OnMiss::CreateStub` + `upsert_node_by_fqn`), which
  already makes indexing order-independent.

## Migration — why the old code cannot be parked first

The daemon must keep indexing while v2 is built; parking `languages/` first
leaves 48,654 files un-indexable and the graph going stale. So:

1. Build `indexer/` alongside, rust only, with no caller.
2. Differential harness: run v1 and v2 over the same corpus, diff the facts.
   v2 must be a strict superset on resolved edges and must explain every
   difference. This is the acceptance gate, not a test count.
3. Cut `process.rs` over to v2 for rust only.
4. Reindex, compare against the recorded v1 baseline.
5. THEN move `languages/rust_lang.rs` to `to_be_discarded/`.
6. Repeat per language. One at a time. No exceptions.

## Acceptance — these, not a percentage

G1. An agent can ask "where is X", "who calls X", "what does X depend on",
    "what shape is this data" and get an answer that is either correct or
    explicitly incomplete with a reason.
G2. A human can see a repo's structure and where its risk is concentrated.

A change that moves a coverage number but serves neither is not progress.

## Decisions — SETTLED, do not re-open

**D1. Scope is the walk AND the persistence path.** If data is lost at the
boundary, a clean walk alone achieves nothing. `upsert_node` takes 8 positional
args with no slot for a declared type; v2 owns the emit path too, and the wire
format between producer and persistence must carry everything the walk captured.

**D2. Parameters are PROPS on the function symbol, not nodes.** They carry the
declared type (which receiver typing needs) without doubling the node count. A
parameter is not a thing you navigate to.

**D3. Externals: index the USE, never the internals.** The definition that
governs this:

> Indexing is the process of scanning SOURCE FILES to extract nodes and edges.
> An edge may legitimately point outside the bubble of local code — that is where
> libraries and language internals live.

So the graph must answer "which libraries do we use most" and "which methods do
we use from which library" — that requires member-level `lib_symbol`, which is
what exists today and works (218,310 edges onto 18,240 nodes). It must NOT read
the internals of an external function; that is what library docs are for. No
dependency sources are ever parsed.

Consequence for `lib_package`: 3,688 such nodes carry ZERO inbound edges. The
"which libraries most" question is a GROUP BY over the symbols' package, not a
node per package. Decide whether the container survives as structure or is
replaced by aggregation — but it is not carrying graph signal today.

**D4. Language order: rust, then js, ts, svelte — in that order.** The trigger to
move on is rust passing its acceptance gate, not elapsed time. One at a time; no
mixing, ever.

**D5. OO structure comes from the SAME walk, and patterns are DERIVED from
nodes and edges — never from a second pass over code.** The walk emits the facts:
`extends`, `implements`, `impl` blocks, trait impls, mixins, decorators, member
ownership, and the declared types of fields, parameters and returns. Pattern
naming (adapter, factory, strategy) is a query over those facts. Source bytes are
read exactly once, ever.

**D6. The walk therefore carries a COMPLETENESS OBLIGATION.** A classifier that
cannot go back to the source can only see what the walk recorded. So the fact set
is not "whatever the walk happens to emit" — it is derived from what the patterns
need, and checked BEFORE the walk is considered done. Deriving it now, from the
patterns worth detecting:

| pattern | recognised by | facts required |
|---|---|---|
| Adapter | implements an interface, holds a field of another type, methods delegate to that field | `implements`, FIELD TYPE, call edge method -> field's member |
| Decorator | implements X **and** holds a field typed X | `implements`, FIELD TYPE |
| Strategy | a field or parameter typed as an interface having ≥2 implementors | FIELD/PARAM TYPE, `implements` |
| Factory | returns an interface/trait type, constructs one of its implementors | RETURN TYPE, construction edge, `implements` |
| Singleton | a static/lazy field of its own type, private constructor | FIELD TYPE, `static`, visibility |
| Observer | registration method taking a callback/interface parameter | PARAM TYPE, `implements` |
| Facade | a type whose members mostly delegate outward, low inbound fan-in | call edges, member ownership |

The union of the right-hand column is exactly: **field types, parameter types,
return types, `implements`/`extends`, construction edges, member ownership, and
call edges from a member to a field's member.** Every one of those is already
committed by D1–D3 (fields as symbols with declared types; parameters as typed
props; return types carried; relations from the walk). The fact set is therefore
sufficient — and that is a consistency check on the design, not a coincidence.

One consequence worth stating plainly, because it changes sequencing:

`self.field.method()` call edges cannot be produced BY THE CURRENT INDEXER at
all. There are zero field nodes in any language, so nothing records that `pg` is
a `PgStore`, and there is nothing to look `method` up on. This is a statement
about the code being replaced, not a limitation of the design.

v2 must produce them: four of the seven patterns above (adapter, decorator,
strategy, singleton) key on a field's declared type. D2 and D3 already commit to
fields as symbols carrying their type, so v2 satisfies this — but it means field
support is a PREREQUISITE for pattern detection, not an optional extra to add
later.

## What is kept, and why

Not everything is broken. These are measured working and must be carried over
unchanged — re-deriving them is how regressions get reintroduced:

- **The fqn grammar** `<lang>·<package>·<module>·<Type>·<member>`. It is a lookup
  key minted independently by caller and definition. Changing it has been tried
  twice and failed twice.
- **Import classification.** 141,980 resolved / 4 unresolved. This is the proof
  the approach works when the AST names a target.
- **External mapping** to `lib·<package>·<member>` with no library internals:
  218,310 edges onto 18,240 nodes. Library sources are never needed.
- **Stub-and-merge ordering** (`OnMiss::CreateStub` + `upsert_node_by_fqn`), which
  already makes indexing order-independent.

## Migration — why the old code cannot be parked first

The daemon must keep indexing while v2 is built; parking `languages/` first
leaves 48,654 files un-indexable and the graph going stale. So:

1. Build `indexer/` alongside, rust only, with no caller.
2. Differential harness: run v1 and v2 over the same corpus, diff the facts.
   v2 must be a strict superset on resolved edges and must explain every
   difference. This is the acceptance gate, not a test count.
3. Cut `process.rs` over to v2 for rust only.
4. Reindex, compare against the recorded v1 baseline.
5. THEN move `languages/rust_lang.rs` to `to_be_discarded/`.
6. Repeat per language. One at a time. No exceptions.

## Acceptance — these, not a percentage

G1. An agent can ask "where is X", "who calls X", "what does X depend on",
    "what shape is this data" and get an answer that is either correct or
    explicitly incomplete with a reason.
G2. A human can see a repo's structure and where its risk is concentrated.

A change that moves a coverage number but serves neither is not progress.

## Decisions — SETTLED, do not re-open

**D1. Scope is the walk AND the persistence path.** If data is lost at the
boundary, a clean walk alone achieves nothing. `upsert_node` takes 8 positional
args with no slot for a declared type; v2 owns the emit path too, and the wire
format between producer and persistence must carry everything the walk captured.

**D2. Parameters are PROPS on the function symbol, not nodes.** They carry the
declared type (which receiver typing needs) without doubling the node count. A
parameter is not a thing you navigate to.

**D3. Externals: index the USE, never the internals.** The definition that
governs this:

> Indexing is the process of scanning SOURCE FILES to extract nodes and edges.
> An edge may legitimately point outside the bubble of local code — that is where
> libraries and language internals live.

So the graph must answer "which libraries do we use most" and "which methods do
we use from which library" — that requires member-level `lib_symbol`, which is
what exists today and works (218,310 edges onto 18,240 nodes). It must NOT read
the internals of an external function; that is what library docs are for. No
dependency sources are ever parsed.

Consequence for `lib_package`: 3,688 such nodes carry ZERO inbound edges. The
"which libraries most" question is a GROUP BY over the symbols' package, not a
node per package. Decide whether the container survives as structure or is
replaced by aggregation — but it is not carrying graph signal today.

**D4. Language order: rust, then js, ts, svelte — in that order.** The trigger to
move on is rust passing its acceptance gate, not elapsed time. One at a time; no
mixing, ever.

**D5. OO structure comes from the SAME walk. No second scan.** The walk emits the
FACTS: `extends`, `implements`, `impl` blocks, trait impls, mixins, decorators,
and which type each member belongs to. These are single-file facts and there is
no reason to re-read source for them.

Distinction that matters for the design: naming a DESIGN PATTERN (adapter,
factory, strategy) is a query over those facts across files — it needs the graph,
not the source. "No second scan" means no second parse of source, not that
pattern classification happens inside the walk. The walk's obligation is to emit
every structural fact a classifier could need, so the classifier never has to go
back to the bytes.

