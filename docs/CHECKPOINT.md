# Checkpoint

**Slice** — indexer-v2. Receiver typing; the grading decision that had blocked
three slices is made.

**Done** — 22 commits. `binding_of` segment fix (582ce311, 3 tests, one per arm
since the arms need different segment rules). Container labelling (e7bf8ae6):
`reached_by` supplies the vocabulary half of `can_be_named`, so containers
table apart under `imports | test imports | not imported` with no evidence
columns, and leave the callable TOTAL (rust 8573→8256 nodes, 3525→3203 `not
called`; container total 325/325 printed, so the old figure still adds up).
Gate: fmt clean, clippy `-D warnings` 0, workspace 3570 / 0.

**DECISION (user) — grade on unresolved SITES by `Reason`, not `lost`.** `lost`
stays printed as an upper bound only. `lost_exact` is 0 on every row and cannot
be otherwise: barrier.rs:357-374 makes the exact and by-name channels mutually
exclusive, and an untyped receiver — the whole remaining defect class — cannot
mint a candidate. `lost_by_name` is set-membership on the bare NAME
(`verdict()`, barrier.rs:401-436): one unresolved `.id` blames all 126 unreached
declarations spelled `id`. Barrier verified deterministic across two runs.

**My sizing was wrong.** I predicted ~60-70 first-party receivers from 644
matching `let` bindings; actual 3, with 0 new edges. I sized the SHAPE without
checking which later rung covers it — `bound_to_a_call` (04289f5f) already
types the first-party half from the callee's declared return type via the
`returns` map (barrier.rs:879). Check the later rung before sizing a walk fix.

**Next — the file-module node.** Design user-approved, unchanged.

1. One `SymbolKind::Module` per FILE, at `Reach::Mod`, from `FileFacts.module`.
2. `RelationKind::Contains` — feeds `parent_id` alongside `Owns`. MUST NOT be
   read by `members_declared_by` (resolve.rs:285-292) or a member call resolves
   onto a module (R4).
3. `RefKind::Imports` — import→module edges; this is what makes the container
   table's `imports` column non-zero (0/325 today).

Present: `SymbolKind::Module` facts.rs:154, `Reach::Mod` fqn.rs:116 (minted at
:400, :831), `FileFacts.module` facts.rs:782, module_path per file
acceptance.rs:89. **Missing:** no `Contains` variant (facts.rs:666-682);
`parent_id` fed only by `Owns` (reconcile.rs:1129). Cause: walk.rs:599 fires
only for `"mod_item"`, so the 325 Module nodes are inline `mod` blocks. The
+1262 file-modules now land in `not imported` — no longer a confusing number.

    cargo test -p senseid --bin senseid -- --ignored --nocapture \
      indexer::acceptance::every_source_node_is_reached_by_a_test_and_then_by_other_source

**Open questions — neither blocking.** (2) is a bare identifier READ a
reference? Decides ts Const 2707 / rust Const 436. (3) `Arc<T>` in
`simple_type_name`.

**Known broken** — `real_dbd_single_file_has_deploy_component` (`--ignored`)
reads a hardcoded `~/Developer/dbd-rs`, verified absent. The barrier counts
acceptance.rs's own 14 `#[ignore]` tests as source: +14 rust Function `not
called`, no `lost`.
