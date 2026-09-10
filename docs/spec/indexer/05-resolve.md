# Stage 5 — the resolution ladder

Whole-system spec: `docs/design/indexer-v2.md` §2 (identity), §2.1 (reach),
R2, R6, R7, A1, A3, A6, D7. Depends on stage 4.

## 1. Purpose

Turn each `Reference`'s minted candidate identity into a `Resolution` — either
a resolved fqn or an `Unresolved` carrying a reason specific enough to act on.
Shared across all languages (R7), because two ladders drift and the drift is
invisible.

Serves G1: `get_callees` is 44% blind today, and the reason histogram is what
turns "blind" into a ranked list of fixable causes.

## 2. Inputs and outputs

    resolve(reference: &Reference, scope: &FileScope, imports: &[Import]) -> Resolution   PURE

Pure, and takes a FILE-LOCAL scope. It must NOT take "everything scanned so
far" — see S3.

## 3. Requirements

- **S1** (§2). Mint from what the CALL SITE can see. If the two sides mint
  different strings they never merge. This broke twice before it was believed,
  and it is the single rule the whole design rests on.
- **S2** (R2). Every reference gets a `Resolution`. `Unresolved` carries a
  `Reason` ENUM — never a `String`, which cannot be exhaustively matched and
  lets the histogram grow junk categories silently.
- **S3** (R6, A6). **Never consult what has been scanned.** A symbol missing
  from the current set may simply not be parsed yet; resolving against
  scan progress makes the graph depend on file order, which is the defect that
  produced the ghost nodes. Externality comes from the IMPORT, not from
  absence. Absence is not evidence.
- **S4** (§2.1, D7). The reach is a function of the USE SYNTAX alone, so a use
  site mints exactly ONE candidate. There is no candidate set, no uniqueness
  gate, and nothing for the resolver to adjudicate — which is also why no rung
  needs scan state.
- **S5** (A3). The reason histogram accounts for 100% of unresolved references.
- **S6.** The plumbing denylist (`clone`, `unwrap`, and friends) gets its own
  reason. It is FILTERING, not failure, and mixing it into a genuine-miss
  bucket hides real gaps behind noise.
- **S7** (R7). One ladder, in one module, shared by every language. A
  language contributes its scope and its imports, not its own resolver.

## 4. Failure modes

| input | this stage does |
|---|---|
| the name matches nothing anywhere | `Unresolved { NotFound }`. Not a guess, not a lib node. |
| the receiver has no type annotation | `Unresolved { NoAnnotation }` — and this is distinct from `AnnotationNotRead`, because one gap is closeable and the other is not. A histogram that cannot tell them apart cannot rank work. |
| an import names a package not in any manifest | resolve to `lib·<package>·…` anyway. The import is the declaration; the manifest's silence does not overrule it. |
| a re-exported item | `Unresolved` with the re-export reason. Re-exports are NOT followed — a recorded gap, and it is the honest answer rather than a wrong target. |
| a trait-impl member reached without its trait (`x.default()`) | mints the plain member form and does not merge with the `<Type>·<Trait>·<member>` declaration. **22 references over 3 identities**, all `Default::default`. Deferred with its evidence captured, never closed by dropping the `Trait` segment — that would delete the only thing separating `<X as Display>::fmt` from `<X as Debug>::fmt`. |
| an ambiguous path split (`a::b::c`) | decided by the grammar's `names_a_type` predicate. A misread produces a DANGLING edge, not a wrong one, because the two splits put the boundary in different places and the loser names no declaration at all. |

## 5. Stage report — what you SHOW when the stage is done

    {"stage":"05-resolve","at":"<iso8601>","repo":"…",
     "references":109328,"resolved":<n>,"unresolved":<n>,
     "resolved_first_party":<n>,"resolved_lib":<n>,
     "import_mediated_resolved_pct":99.94,
     "reasons":{"NotFound":<n>,"NoAnnotation":<n>,"AnnotationNotRead":<n>,
                "Denylisted":<n>,"ReExport":<n>,"TraitMember":22,
                "UnhandledForm":<n>},
     "reasons_sum_equals_unresolved":true,
     "sample_unresolved":{"site":"pg_store/graph.rs:812:14","wrote":"pool.bind()",
                          "minted":"rust·senseid·db::pg_store·?·bind·item",
                          "reason":"NoAnnotation",
                          "evidence":{"receiver":"pool","binding":"ReturnOf(new_pool)"}}}

`reasons_sum_equals_unresolved` is asserted, not reported — A3 requires 100%
coverage and a histogram that quietly loses rows is worse than none.

The sample must render the EVIDENCE, not just the reason. "NoAnnotation" tells
you the class; the receiver and its binding provenance tell you whether a
future pass could close it.

## 6. Verification

| test | mutation that must break it |
|---|---|
| one test per `Reason` variant, asserting that exact variant is produced | merge two reasons into one |
| the histogram sums to 100% of unresolved over this repo's real Rust | drop any reason emission |
| **order independence** — resolve the same file with an EMPTY "already scanned" set and with a FULL one; output identical | make any rung consult scan state |
| a nonexistent symbol resolves to `Unresolved`, never to a plausible `lib·` node | add an absence-implies-external fallback |
| import-mediated references resolve at ≥99.9% (A1) | break the import splicing |
| a denylisted `clone` gets `Denylisted`, not `NotFound` | remove S6's separate reason |
| `Reason` is an enum and no `String` reason exists in the tree | change one to a `String` |
| a use site mints exactly one candidate — no candidate set type exists | introduce a `Vec<Fqn>` candidate list |

The order-independence test is the one that catches the whole class. It is
cheap, it runs on one file, and it would have caught the ghost-node defect
before it reached the corpus.

## 7. Watch out

**"External because we did not find it" is the defect, not the fallback.** It
is the most natural thing to write and it makes the graph scan-order dependent.
Externality comes from the import. If there is no import, the answer is
`Unresolved`, and that is a better answer than a fabricated `lib·` node a
consumer cannot distinguish from a real one.

**Do not tighten the semantic-search threshold reasoning into this stage.**
Resolution is exact, not ranked. There is no "close enough" fqn.

**A wrong edge is worse than a missing one (R4).** Every rung that is tempted
to guess should return `Unresolved` with a reason instead — the gap is then
DATA with a fill path (R11), which is the whole disposition of this design.

## 8. Definition of done

- One shared ladder, database-free, used by every language module.
- A test per reason variant; the histogram sums to 100% over the real corpus.
- Order independence proven by the empty-vs-full scope test.
- A1 met: import-mediated resolution ≥99.9%.
- No absence-implies-external path exists anywhere, verified by test.
