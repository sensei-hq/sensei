# Checkpoint

**Slice** — indexer-v2, receiver typing. Rationale lives in
`/sensei:session status` and the 582ce311 / e7bf8ae6 commit bodies.

**Done** — 22 commits. `binding_of` segment fix (582ce311): each initialiser
shape says where in its path the type is. Container labelling (e7bf8ae6):
`reached_by` supplies the vocabulary half of `can_be_named`, so containers
table apart under `imports | test imports | not imported` and leave the
callable TOTAL — rust 8573→8256 nodes, 3525→3203 `not called`, container total
325/325 printed. Gate: fmt clean, clippy `-D warnings` 0, workspace 3570 / 0.

**Decisions** — grade on unresolved SITES by `Reason`, not `lost`; keep `lost`
printed as an upper bound. `lost_exact` is 0 everywhere and structurally must
be. Sizing lesson: predicted ~60-70 receivers, got 3 — `bound_to_a_call`
already covered the first-party half; check the later rung before sizing.

**Next — the file-module node.** User-approved, unchanged.

1. One `SymbolKind::Module` per FILE, at `Reach::Mod`, from `FileFacts.module`.
2. `RelationKind::Contains` — feeds `parent_id` alongside `Owns`. MUST NOT be
   read by `members_declared_by` (resolve.rs:285-292), or a member call
   resolves onto a module (R4).
3. `RefKind::Imports` — makes the container `imports` column non-zero (0/325).

Missing: no `Contains` variant (facts.rs:666-682); `parent_id` fed only by
`Owns` (reconcile.rs:1129). Cause: walk.rs:599 fires only for `"mod_item"`, so
the 325 Module nodes are inline `mod` blocks. The +1262 file-modules now land
in `not imported`.

    cargo test -p senseid --bin senseid -- --ignored --nocapture \
      indexer::acceptance::every_source_node_is_reached_by_a_test_and_then_by_other_source

**Open** — neither blocking. Is a bare identifier READ a reference? (decides ts
Const 2707 / rust Const 436). `Arc<T>` in `simple_type_name`.

**Known broken** — `real_dbd_single_file_has_deploy_component` (`--ignored`)
reads a hardcoded `~/Developer/dbd-rs`, absent here.
