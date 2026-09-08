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

Anything that serves neither is out of scope.

## 2. Definitions

**Indexing** is scanning SOURCE FILES to extract nodes and edges. An edge may
point outside the local code — that is where libraries and language internals
live. Dependency sources are never parsed.

**FQN** — the identity of a symbol,
`<lang>·<package>·<module>·<Type>·<member>·<ns>`. It is a LOOKUP KEY, minted
independently by a reference and by a definition. If the two sides mint
different strings for the same symbol they never merge, so every rule that
builds one is shared, never per-language.

The trailing `<ns>` is the NAMESPACE the name is minted in — for Rust, one of
`ty`, `val`, `field`, `macro`. It is required because a language may let two
declarations share one name in one scope when they sit in different namespaces:
this repo has a `WatcherHealth::healthy` field alongside a `healthy()` getter,
and `sensei-bootstrap` has `pub mod config;` alongside `pub fn config()`.
Without it those mint one string, one declaration overwrites the other, and
references to either land on the wrong one — a wrong edge (R4). The
discriminator is the namespace and not the declaration KIND, because a use site
can tell namespaces apart from syntax alone (`x.foo` vs `x.foo()`) but cannot
tell a `const` from a `static`. An external (`lib·<package>·<member>`) carries
no namespace: we index no declarations for it, so there is nothing to collide.

**Local** — declared in a scanned source file. **External** — everything else.
Externality is decided by the import that brought a name into scope, never by
whether a symbol happens to be absent (absence is scan-order dependent).

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

## 5. Non-goals

- Type inference. The walk reads types the language STATES. It does not infer
  the type of an unannotated value. Where a language does not state a type, the
  reference is `Unresolved` with that reason.
- Parsing dependency sources.
- Pattern classification inside the walk.
- Any language beyond the one currently being built.

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
