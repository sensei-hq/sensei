# Checkpoint — the indexer

**Slice: Java. The two coverage barriers are now ONE implementation
(`indexer/barrier.rs`) run over both corpora, taught Java's JUnit convention via
`languages::is_test_path`. Java's first barrier numbers: 23,593 nodes, 63% no
test edge, 40% exercised, 48% reached by nothing. Java's 26,995 dangling edges
are decomposed into 10 named buckets — the "chiefly Lombok and Spring Data"
reading was two thirds wrong. Suite 3,514 / 0, clippy + fmt clean. Stage 10
cutover still gated.**

Read `docs/plans/indexer-sequence.md`, then `docs/design/indexer.md`.

## Canonical reports — do not retype their numbers

    cargo test -p senseid --bin senseid -- --ignored --nocapture \
      indexer::acceptance::report
      indexer::acceptance::every_first_party_edge_names_a_declaration_this_scan_holds
      indexer::acceptance::every_source_node_is_reached_by_a_test_and_then_by_other_source
    SENSEI_CORPUS=~/Work/Dayamed … indexer::lang::java::corpus

## Done in this slice

- **`indexer/barrier.rs`** — the two barriers, extracted so both corpora run one
  copy. `acceptance` supplies this repo's Rust and TypeScript; `lang::java::corpus`
  supplies `SENSEI_CORPUS`.
- **One test-vs-source classifier**, delegating to `languages::is_test_path` —
  already the source of truth for `nodes.is_test` and already Java-aware. Taught
  it the `_tests.rs` stem it was missing (5 real files). The inline
  `#[cfg(test)]` region stays, because no path can show it.
- **Java interface inheritance** — `interface_declaration` states its parents in
  an `extends_interfaces` child with NO field name, so every Java interface read
  as having no parents. Same change stopped a clause's type ARGUMENTS being
  recorded as supertypes (`extends JpaRepository<User, Long>` was emitting
  `UserRepository extends Long`).
- **Rust `for` patterns** — `for_expression` skipped the pattern between the
  collection and the body, so `for Placed { .. } in &corpus` named no type.
  Found by A2 going 0 -> 1 files disagreeing.

## Next

1. **Java's field reach** — 9,932 of the 26,995 are a declaration at
   `Reach::Field` against a use site at `Reach::Item`. NOT a one-liner: 2,856
   field reads land today and every one lands on an `item` (enum constants), so
   the naive change trades dangling for dangling. Detail in `docs/backlog.md`.
2. **Cause B, rust**: 761 of rust's orphans are `ReceiverTypeUnknown`. Java has
   the same shape at 41,969, plus a same-package rung it has no equivalent of.
3. A8's two finds (652 rust `Owns` parents, 561 typescript barrel targets).

## Known-broken — do not build on

- Java: 26,995 dangling, decomposed. 10,403 Lombok + 2,529 Spring Data are
  compile-time synthesis and deliberately NOT attempted; 9,932 field-reach and
  2,356 interface-constant are indexer defects with their own slice.
- Java barrier 2 is 62% for two measured reasons: a same-package reference needs
  no import and no rung places one (33,597 misses), and a local's type does not
  survive to the next statement (41,969 misses).
- 252 rust nodes are named by no use site: registered task handlers and callees
  used only inside a macro. An upper bound on dead code, not a count of it.
- A7: 525 colliding identities. A2 TypeScript drops 14 ours; Java 9.
- `indexer::lib_indexer::tests::real_dbd_single_file_has_deploy_component` needs
  a sibling `dbd-rs` checkout at a hard-coded absolute path. `#[ignore]`d.
- `delete_folder` issues a path-prefix DELETE (`process.rs`); 09 S7 forbids it.
- `library_content.package_name` has no writer.
