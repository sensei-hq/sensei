# Checkpoint

**Slice** — indexer-v2, receiver typing. rust `lost` **2048 → 1855** this
session. Rationale: `/sensei:session status` + commit bodies.

**Done** — 24 commits. `binding_of` segment fix (582ce311). Container
reporting (e7bf8ae6, a9d5c427): a container's row is its COUNT only — `not
called` was the wrong word, `not imported` a false claim. One-binding
destructured params (e256f572): Field calls 758→936, lost 1441→1248.
Gate: fmt clean, clippy `-D warnings` 0, workspace 3575 / 0.

**FQN scheme is not the gap — don't re-open.** Field FQNs are already
type-qualified (`Form::Member.ty`, `Required::Yes`, fqn.rs:237-242) and
`Reach::Field` already splits a field from a same-named method. The qualifier
is unfillable at the USE site and the walk won't invent one, so it records a
bare name. Hence `lost_exact = 0` is guaranteed: exact evidence needs a
candidate, a candidate needs the type.

**5347 untyped field reads** — 2290 closure/match · **632 destructured param,
FIXED** · 616 path call · 595 plain call · 446 method chain (external, rightly
refused) · 378 chained/indexed · 150 call receiver.

**Grading** — unresolved SITES by `Reason`; `lost` is the alarm only
(reliably 0 good, high = investigate).

**Next — the file-module node.** User-approved, unchanged.

1. One `SymbolKind::Module` per FILE, at `Reach::Mod`, from `FileFacts.module`.
2. `RelationKind::Contains` — feeds `parent_id` alongside `Owns`. MUST NOT be
   read by `members_declared_by` (resolve.rs:285-292), or a member call
   resolves onto a module (R4).
3. `RefKind::Imports` — import→module edges.

Missing: no `Contains` variant (facts.rs:666-682); `parent_id` fed only by
`Owns` (reconcile.rs:1129). The 325 Module nodes are inline `mod` blocks; the
+1262 file-modules land in the container count, harmless now.

    cargo test -p senseid --bin senseid -- --ignored --nocapture \
      indexer::acceptance::every_source_node_is_reached_by_a_test_and_then_by_other_source

**Open** — bare identifier READ a reference? (ts Const 2707 / rust Const 436).
`Arc<T>` in `simple_type_name`. **Known broken** —
`real_dbd_single_file_has_deploy_component` reads an absent `~/Developer/dbd-rs`.
