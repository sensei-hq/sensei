# Checkpoint — the indexer

**Slice: final measurement. No code changed; the three reports were re-run and
every remaining "reached by nothing" entry classified against source. Suite
3,519 / 0 failed, clippy 0, fmt clean, both `SENSEI_CORPUS` java groups green.
Stage 10 cutover still gated.** Read `docs/plans/indexer-sequence.md`, then
`docs/design/indexer.md`. Canonical reports — do not retype their numbers:

    cargo test -p senseid --bin senseid -- --ignored --nocapture indexer::acceptance::
      report | every_first_party_edge_names_a_declaration_this_scan_holds
      every_source_node_is_reached_by_a_test_and_then_by_other_source
    SENSEI_CORPUS=~/Work/Dayamed … indexer::lang::java::corpus

| | nodes | no test | exercised | nothing | resolve |
|---|---|---|---|---|---|
| rust | 4,376 | 3,408 78% | 2,607 60% | 1,131 26% | 50.6% |
| typescript | 2,186 | 1,543 71% | 821 38% | 508 23% | 41.6% |
| java | 23,593 | 14,854 63% | 9,478 40% | 11,391 48% | 64.3% |

Only "named by no use site" can mean no caller: rust 252 of 1,131, ts 196 of 508,
java 3,105 of 11,391 — the rest are resolver misses already decomposed. Inside
it, "no test AND no caller" holds for **33 rust** (4 `main`, 17 task table, 12
serde), **11 ts**, **~300 java**. The remainder is the instrument, not dead code.

## Next — largest first, all measured

1. **Re-export tables.** ts `$lib` hides 93 orphans / 1,456 refs; rust `pub use`
   hides 11; ts barrels 561 A8 edges. One artifact, three corpora.
2. **A call inside a macro emits nothing** — 90 of rust's 252.
3. **A function passed as a VALUE emits nothing** — 59 ts (Svelte props).
4. Java field reach 9,932; cause B (rust 761, java 41,969 receiver-untyped).

## Known-broken — do not build on

- `acceptance.rs` is `#[cfg(test)]` at its PARENT, so `barrier::test_boundary`
  reads its 13 `#[test]` fns as source. Fourth classifier bug of this probe.
- `constructor` is a node (33 ts); no use site can ever name one.
- Java 26,995 dangling stands; Lombok 10,403 + Spring Data 2,529 not attempted.
- A7: 525 colliding identities. A2 ts drops 14; java 9.
- `real_dbd_single_file_has_deploy_component` needs a sibling `dbd-rs`. `#[ignore]`d.
