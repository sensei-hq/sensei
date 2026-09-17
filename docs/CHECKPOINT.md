# Checkpoint

**Slice** — indexer-v2. The `lost` account is complete and green. Next: the
file-module node, which both containment and import edges need.

**Done** — 20 commits. `lost` is 0 for 9 of 11 rust kinds and 6 of 9 typescript
kinds. Gate at HEAD: fmt clean, clippy 0, 3566 passed / 0 failed.

**Remaining `lost` — DEFECTS, all one cause (untyped receiver).** rust Field
1441, rust Method 603, ts Field 2118, ts Method 59, ts Property 3.
`config().daemon_url()` (bootstrap/src/lib.rs:47) chains off a call, not a
binding; `cfg.db_url` (api/server.rs:110) because `binding_of` reads the HEAD of
`sensei_bootstrap::SenseiConfig::from_env`, a crate name.

**FINDING (this session, verified).** A Rust FILE emits no Module node —
`module_item` fires only for `"mod_item"` (lang/rust/walk.rs:599), so the 325
Module nodes are inline `mod` blocks, mostly `mod tests`. Consequences: an
import→module edge has no target for `use crate::indexer::resolve::…`, and
top-level items have `nodes.parent_id` NULL because no file-module can parent
them. `nodes.parent_id` already exists and is fed ONLY by `RelationKind::Owns`
(reconcile.rs:1129), which is type→member only — so the containment tree is two
levels deep.

**Next — forward-only, user-approved design**

1. One `SymbolKind::Module` per FILE, at `Reach::Mod`, from `FileFacts.module`.
2. `RelationKind::Contains` — file-module → top-level items, inline mod → its
   items. Feeds `parent_id` alongside `Owns`. MUST NOT be read by
   `members_declared_by`: widening `Owns` would let a member call resolve onto a
   module (a fabricated edge, R4).
3. `RefKind::Imports` — import→module edges, now that a target exists.

    cargo test -p senseid --bin senseid -- --ignored --nocapture \
      indexer::acceptance::every_source_node_is_reached_by_a_test_and_then_by_other_source

**Open questions — (1) IS BLOCKING, asked, unanswered.** Grade on `lost` or on
unresolved SITE counts? `lost` is a by-name upper bound that CANNOT reach zero:
952 unresolved `.id` reads blame all 126 field declarations spelled `id`. Three
slices have now raised it. (2) should a bare identifier READ be a reference?
Decides ts Const 2707 / rust Const 436 `not called`; both walks chose "no".
(3) `Arc<T>` unwrapping in `simple_type_name`.

**Known broken** — `lib_indexer::tests::real_dbd_single_file_has_deploy_
component` (`--ignored`) reads a hardcoded `~/Developer/dbd-rs` that does not
exist here. The barrier counts acceptance.rs's own 14 `#[ignore]` tests as
source: +14 rust Function `not called`, no `lost`.
