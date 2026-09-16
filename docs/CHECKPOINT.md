# Checkpoint — the indexer

**Slice: make "reached by NOTHING" a small, explainable list. Defect 1 fixed —
a seventh rung, `declared_by_its_type`, confirms a member candidate against
what some TYPE of the scan was read declaring, not just against this file.
Rust orphans 1,360 -> 1,131; typescript 993 -> 929; +2,966 edges, 0 moved.
Suite 3,506 / 0, clippy + fmt clean. Stage 10 cutover still gated.**

Read `docs/plans/indexer-sequence.md`, then `docs/design/indexer.md`.

## Canonical reports — do not retype their numbers

    cargo test -p senseid --bin senseid -- --ignored --nocapture \
      indexer::acceptance::report
      indexer::acceptance::every_source_node_is_reached_by_a_test_and_then_by_other_source
      indexer::impact::tests::report
    SENSEI_CORPUS=~/Work/Dayamed … indexer::lang::java::corpus

## Done in this slice

- `World::declared_members` — a barrier artifact, the `Owns` children of a
  COMPLETED pass. Both harnesses now take every artifact off the anchored
  (second) pass; a member identity carries its type's module, so the first pass
  spells them differently.
- `Rung::DeclaredByItsType`, seeded at precedence 25. Below `through_an_import`
  (above it, 6 `inbox.rs` sites hit senseid's `ArtifactKind` instead of
  `dojo_protocol`'s) and restricted to `Owns` children (wider, 68 Svelte props
  hit locals in their `.spec.` files). Both pinned by a test.
- The barrier test now DECOMPOSES what remains, by kind, area and cause.

## Next

1. **Defect 1 remainder, the larger half**: 761 of rust's 1,131 are
   `ReceiverTypeUnknown`. Needs a barrier table of first-party return types plus
   `await_expression` unwrapping in `Walk::binding_of` (walk.rs:382), so
   `let s = pg_store().await` types `s`.
2. Defect 3: the JS walk reads no `PrivateFieldExpression` at a call site, so
   `this.#hydrate(api)` names nothing (`scan-state.svelte.ts:323`, decl :349).
3. Trait-impl members: a use site mints `Type·member`, the declaration is
   `Type·Trait·member`. Neither rung reaches it.

## Known-broken — do not build on

- 251 rust nodes are named by no use site: registered task handlers and callees
  used only inside a macro (the walk emits nothing from inside one). An upper
  bound on dead code, not a count of it.
- Java: 26,995 first-party edges name a member no source declares (Lombok,
  Spring Data). Unmoved by this slice.
- A7: 708 colliding identities. A2 TypeScript drops 14 ours / 3,768 foreign.
- `delete_folder` issues a path-prefix DELETE (`process.rs`); 09 S7 forbids it.
- `library_content.package_name` has no writer.
