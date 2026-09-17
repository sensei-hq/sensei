# Checkpoint

**Slice** — indexer-v2, receiver typing. rust `lost` **2048 → 1855** this
session. Full rationale: `/sensei:session status` and the commit bodies.

**Done** — 24 commits. `binding_of` segment fix (582ce311). Container
reporting (e7bf8ae6, a9d5c427): a container's row is its COUNT and nothing
else — `not called` was the wrong word, `not imported` a false claim, since
those modules ARE imported and what is absent is the edges. One-binding
destructured params (e256f572): Field calls 758→936, lost 1441→1248.
Gate: fmt clean, clippy `-D warnings` 0, workspace 3575 / 0.

**The FQN scheme is not the problem — checked, don't re-open.** Field FQNs are
already type-qualified (`Form::Member` takes `ty`, `Required::Yes`,
fqn.rs:237-242), and `Reach::Field` already separates a field from a same-named
method. The qualifier is unfillable at the USE site, and since `ty` is required
the walk refuses to invent one — so it records a bare name and `verdict()`
blames every field of that name. Hence `lost_exact = 0` is **guaranteed**, not
accidental: exact evidence needs a candidate, a candidate needs the type.

**The 5347 untyped field reads, decomposed** (probe written, read, removed):
2290 closure arg / match arm · **632 destructured param — FIXED** · 616 `let` =
path call · 595 `let` = plain call · 446 `let` = method chain (external
signature, correctly refused) · 378 chained/indexed · 150 call receiver.

**Grading** — on unresolved SITES by `Reason`, not `lost`; keep `lost` as the
alarm (reliably 0 good, high = investigate). Containers can't contribute to it.

**Next — the file-module node.** User-approved, unchanged.

1. One `SymbolKind::Module` per FILE, at `Reach::Mod`, from `FileFacts.module`.
2. `RelationKind::Contains` — feeds `parent_id` alongside `Owns`. MUST NOT be
   read by `members_declared_by` (resolve.rs:285-292), or a member call
   resolves onto a module (R4).
3. `RefKind::Imports` — import→module edges.

Missing: no `Contains` variant (facts.rs:666-682); `parent_id` fed only by
`Owns` (reconcile.rs:1129). Cause: walk fires `module_item` only for
`"mod_item"`, so the 325 Module nodes are inline `mod` blocks. The +1262
file-modules land in the container count — harmless now.

    cargo test -p senseid --bin senseid -- --ignored --nocapture \
      indexer::acceptance::every_source_node_is_reached_by_a_test_and_then_by_other_source

**Open** — neither blocking. Is a bare identifier READ a reference? (ts Const
2707 / rust Const 436). `Arc<T>` in `simple_type_name`.

**Known broken** — `real_dbd_single_file_has_deploy_component` (`--ignored`)
reads a hardcoded `~/Developer/dbd-rs`, absent here.
