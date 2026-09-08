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

## Open questions for the user — answer before implementation starts

1. **Scope of v2.** Just the fact-production walk, or also the persistence path
   (`process.rs` emit + `graph_facts`)? The emit path has its own dropped-data
   seam (`upsert_node` takes 8 positional args and has no slot for a declared
   type), so a clean walk alone will still lose data at the boundary.
2. **Parameters as nodes.** They make "what shape is this call" answerable and
   are needed for receiver typing, but they roughly double the node count.
   Worth it, or keep parameter types on the function symbol only?
3. **Do we index dependency sources?** Currently no, and externals map to a name.
   Everything measured says that is sufficient. Confirm it stays that way.
4. **Which language after rust**, and is the trigger "rust hits its acceptance
   gate" rather than a time box?
