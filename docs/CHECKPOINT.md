# Checkpoint

**Slice** — indexer-v2. Receiver typing, and the grading decision that had
blocked three slices is now made.

**Done** — 21 commits. `binding_of` segment fix (582ce311), red-first, 3 tests
— one per arm, because the arms need DIFFERENT segment rules and a one-arm fix
leaves one red. Gate at HEAD: fmt clean, clippy `-D warnings` 0, workspace 3569
passed / 0 failed.

**DECISION (user, this session) — grade on unresolved SITES by `Reason`, not on
`lost`.** `lost` stays printed as an upper bound for comparability only.
Grounds: `lost_exact` is 0 on every row of both languages and cannot be
anything else — barrier.rs:357-374 makes the exact and by-name channels
mutually exclusive, and an untyped receiver (the whole remaining defect class)
cannot mint a candidate. `lost_by_name` is set-membership on the bare NAME
(`verdict()`, barrier.rs:401-436): one unresolved `.id` blames all 126 unreached
declarations spelled `id`. Inflated on one side, blind on the other. Barrier
verified deterministic across two runs.

**My sizing was wrong — recorded so it is not repeated.** I estimated the fix
at ~60-70 first-party receivers from 644 matching `let` bindings. Actual: 3
receivers reclassified (596→593 receiver-untyped), 0 new first-party edges,
`lost` unchanged. I sized the SHAPE without checking which later rung covers
it: `bound_to_a_call` (04289f5f) already types the first-party half from the
callee's declared return type via the `returns` map (barrier.rs:879). Before
sizing a walk fix, check which later rung already covers it.

**Next — the file-module node.** Design user-approved, unchanged.

1. One `SymbolKind::Module` per FILE, at `Reach::Mod`, from `FileFacts.module`.
2. `RelationKind::Contains` — feeds `parent_id` alongside `Owns`. MUST NOT be
   read by `members_declared_by` (resolve.rs:285-292) or a member call resolves
   onto a module (R4).
3. `RefKind::Imports` — import→module edges, now that a target exists.

Present: `SymbolKind::Module` facts.rs:154, `Reach::Mod` fqn.rs:116 (minted at
:400, :831), `FileFacts.module` facts.rs:782, module_path per file
acceptance.rs:89. **Missing:** `RelationKind` has no `Contains`
(facts.rs:666-682); `parent_id` fed only by `Owns` (reconcile.rs:1129). Cause
confirmed: walk.rs:599 fires only for `"mod_item"`, so the 325 Module nodes are
inline `mod` blocks.

**Expect `not called` +~1262 — NOT a regression.** `Module` is `can_be_named`
false (facts.rs:194), so a file-module reads `lost` 0 by construction, and the
corpus has 389 rust + 873 web files. Pin it before the number moves.

    cargo test -p senseid --bin senseid -- --ignored --nocapture \
      indexer::acceptance::every_source_node_is_reached_by_a_test_and_then_by_other_source

**Open questions — neither blocking now.** (2) should a bare identifier READ be
a reference? Decides ts Const 2707 / rust Const 436 `not called`; both walks
chose "no". (3) `Arc<T>` unwrapping in `simple_type_name`.

**Known broken** — `lib_indexer::tests::real_dbd_single_file_has_deploy_
component` (`--ignored`) reads a hardcoded `~/Developer/dbd-rs`, verified absent
here. The barrier counts acceptance.rs's own 14 `#[ignore]` tests as source:
+14 rust Function `not called`, no `lost`.
