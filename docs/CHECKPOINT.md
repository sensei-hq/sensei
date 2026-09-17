# Checkpoint

**Slice** — indexer-v2. Receiver typing; the grading decision that had blocked
three slices is made. Full rationale: `/sensei:session status` + 582ce311,
e7bf8ae6 commit bodies.

**Done** — 22 commits. `binding_of` segment fix (582ce311) — each initialiser
shape says where in its path the type is; 3 tests, one per arm. Container
labelling (e7bf8ae6) — `reached_by` supplies the vocabulary half of
`can_be_named`, so containers table apart under `imports | test imports | not
imported` and leave the callable TOTAL (rust 8573→8256 nodes, 3525→3203 `not
called`; container total 325/325 printed so the old figure still adds up).
Gate: fmt clean, clippy `-D warnings` 0, workspace 3570 / 0.

**DECISION (user) — grade on unresolved SITES by `Reason`, not `lost`.**
`lost_exact` is 0 on every row and cannot be otherwise (barrier.rs:357-374 makes
the two channels exclusive; an untyped receiver can't mint a candidate).
`lost_by_name` is name set-membership (barrier.rs:401-436): one unresolved `.id`
blames all 126 declarations spelled `id`. Keep printing it as an upper bound.

**Sizing lesson** — predicted ~60-70 receivers from 644 matching bindings;
actual 3, 0 new edges. `bound_to_a_call` (04289f5f) already covered the
first-party half via the `returns` map. Check the later rung before sizing.

**Next — the file-module node.** Design user-approved, unchanged.

1. One `SymbolKind::Module` per FILE, at `Reach::Mod`, from `FileFacts.module`.
2. `RelationKind::Contains` — feeds `parent_id` alongside `Owns`. MUST NOT be
   read by `members_declared_by` (resolve.rs:285-292), or a member call
   resolves onto a module (R4).
3. `RefKind::Imports` — makes the container `imports` column non-zero (0/325).

Present: `SymbolKind::Module` facts.rs:154, `Reach::Mod` fqn.rs:116,
`FileFacts.module` facts.rs:782, module_path acceptance.rs:89. **Missing:** no
`Contains` variant (facts.rs:666-682); `parent_id` fed only by `Owns`
(reconcile.rs:1129). Cause: walk.rs:599 fires only for `"mod_item"`, so the 325
Module nodes are inline `mod` blocks. The +1262 file-modules now land in `not
imported`.

    cargo test -p senseid --bin senseid -- --ignored --nocapture \
      indexer::acceptance::every_source_node_is_reached_by_a_test_and_then_by_other_source

**Open questions — neither blocking.** (2) is a bare identifier READ a
reference? Decides ts Const 2707 / rust Const 436. (3) `Arc<T>` in
`simple_type_name`.

**Known broken** — `real_dbd_single_file_has_deploy_component` (`--ignored`)
reads a hardcoded `~/Developer/dbd-rs`, absent here. The barrier counts
acceptance.rs's own 14 `#[ignore]` tests as source: +14 rust Function.
