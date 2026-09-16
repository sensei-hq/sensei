# Checkpoint — the indexer

**Slice: both TypeScript defects fixed. The import CLAUSE now names the member
(3,304 edges that pointed at nothing now land), and a `#private` member is a use
site in every position a public one is (+221 edges, 0 dangling). TypeScript's
categorical zero is gone: no test edge 100% -> 71%, exercised 0% -> 37%, reached
by nothing 993 -> 517. Suite 3,513 / 0, clippy + fmt clean. Stage 10 cutover
still gated.**

Read `docs/plans/indexer-sequence.md`, then `docs/design/indexer.md`.

## Canonical reports — do not retype their numbers

    cargo test -p senseid --bin senseid -- --ignored --nocapture \
      indexer::acceptance::report
      indexer::acceptance::every_first_party_edge_names_a_declaration_this_scan_holds
      indexer::acceptance::every_source_node_is_reached_by_a_test_and_then_by_other_source
      indexer::lang::javascript::tests::the_reference_count_equals_an_independent_count_of_use_sites
    SENSEI_CORPUS=~/Work/Dayamed … indexer::lang::java::corpus

## Done in this slice

- **A8, `every_first_party_edge_names_a_declaration_this_scan_holds`** — the
  measurement that was missing. `report` counts a reference RESOLVED the moment a
  rung answers and never asks whether the identity exists, so a whole language's
  import rung was broken in plain sight. Split by language, fact and rung.
- **`Binding::MemberOf { local, member }`** — the walk states whether the
  specifier already spells the bound name. NOT a per-language flag: `import * as`
  and `import { }` sit in one file, and a flag breaks the namespace case.
- **`Grammar::module_segment`** — the language's own file-stem rule, the same
  function `javascript::module_path` applies to the declaration side.
- **`PrivateFieldExpression`** in `name_callee`, `expression`, `assignment` and
  the update arm, plus `visit_private_field_expression` in A2's counter — fixed
  counter FIRST, watched A2 go 14 -> 181, then the walk took it back to 14.

## Next

1. **Cause B, the largest remaining**: 761 of rust's 1,133 orphans are
   `ReceiverTypeUnknown`. Needs a barrier table of first-party return types plus
   `await_expression` unwrapping in `Walk::binding_of` (walk.rs:382).
2. **A8's two finds, both pre-existing** (detail in `docs/backlog.md`): 652 rust
   `Owns` relation parents minted at the impl's module instead of the type's
   home, and 561 typescript re-export-barrel targets.
3. Trait-impl members: a use site mints `Type-member`, the declaration is
   `Type-Trait-member`. Neither rung reaches it.

## Known-broken — do not build on

- 251 rust nodes are named by no use site: registered task handlers and callees
  used only inside a macro (the walk emits nothing from inside one). An upper
  bound on dead code, not a count of it.
- Java: 26,995 first-party edges name a member no source declares (Lombok,
  Spring Data). Unmoved by this slice — re-measured, identical.
- A7: 708 colliding identities. A2 TypeScript drops 14 ours.
- `indexer::lib_indexer::tests::real_dbd_single_file_has_deploy_component` needs
  a sibling `dbd-rs` checkout at a hard-coded absolute path and fails without
  one. Environmental, `#[ignore]`d, not in the gate.
- `delete_folder` issues a path-prefix DELETE (`process.rs`); 09 S7 forbids it.
- `library_content.package_name` has no writer.
