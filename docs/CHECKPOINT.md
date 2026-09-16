# Checkpoint — the indexer

**Slice: fix the per-kind report before anyone chases it. Done (f3e105dc).
Suite 3,532 / 0 failed, clippy 0, fmt clean. Stage 10 cutover still gated.**
Read `docs/plans/indexer-sequence.md`, then `docs/design/indexer.md`.
Canonical reports — do not retype their numbers:

    cargo test -p senseid --bin senseid -- --ignored --nocapture indexer::acceptance::
      report | every_first_party_edge_names_a_declaration_this_scan_holds
      every_source_node_is_reached_by_a_test_and_then_by_other_source
    SENSEI_CORPUS=~/Work/Dayamed … indexer::lang::java::corpus

`lost` is now two columns. `exact` is an unresolved use site carrying the
node's own identity; `by name` is a bare name matched AT THE SAME REACH.
Kinds no reference can name are marked `*` and read 0 by construction
(`SymbolKind::can_be_named`). Current `lost`, after the correction:

| rust | | typescript | |
|---|---|---|---|
| Field | 1,602 | Field | 2,684 |
| Method | 800 | Const | 357 |
| Function | 12 | Function | 156 (5 exact) |
| Const / Static / Module `*` | 0 | Static 32, Method 82, Property 9 |

The drop from the old table is the metric getting honest, NOT progress.

## Next — largest first, all measured

1. **Field, both languages (4,286).** Hypothesis unconfirmed: a field read is
   `receiver.name` with the receiver-typing problem calls had, at `Reach::Field`.
   Both new rungs were written for calls. Read the PATH-rung guard in
   `Ladder::climb` before touching it. PROBE FIRST.
2. **rust Method 800** — same receiver typing, one reach up.
3. **Re-export tables.** ts `$lib` hides 93 orphans / 1,456 refs; rust `pub use`
   hides 11; ts barrels 561 A8 edges.
4. A call inside a macro emits nothing (90 rust); a function passed as a VALUE
   emits nothing (59 ts, Svelte props).

## Known-broken — do not build on

- `acceptance.rs` is `#[cfg(test)]` at its PARENT, so `barrier::test_boundary`
  reads its 13 `#[test]` fns as source. Fourth classifier bug of this probe.
- `constructor` is a node (33 ts); no use site can ever name one.
- Java 26,995 dangling stands; Lombok 10,403 + Spring Data 2,529 not attempted.
- A7: 525 colliding identities. A2 ts drops 14; java 9.
