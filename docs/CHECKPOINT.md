# Checkpoint — the indexer

**Slice: FIELD. TypeScript done (df60a271); rust Field untouched and is next.
Workspace suite 0 failed, clippy 0, fmt clean. Stage 10 cutover still gated.**
Read `docs/plans/indexer-sequence.md`, then `docs/design/indexer.md`.
Canonical reports — do not retype their numbers:

    cargo test -p senseid --bin senseid -- --ignored --nocapture indexer::acceptance::
      report | every_first_party_edge_names_a_declaration_this_scan_holds
      every_source_node_is_reached_by_a_test_and_then_by_other_source
    SENSEI_CORPUS=~/Work/Dayamed … indexer::lang::java::corpus

Two commits, and the ORDER matters — the metric was fixed first and
re-baselined, so the walk fix is neither credited nor blamed for its movement.

1. `173c8f8d` the bare-name set is keyed by LANGUAGE as well as reach. A
   TypeScript use site cannot mint a rust identity. 183 rust + 55 ts field
   declarations were lost on the other language's evidence. No edge moved.
2. `df60a271` the JavaScript reader now consults the type table it was already
   being handed. `member_of` filed every member under the READING file's
   module; of 1,997 candidates minted at field reach, none named a real
   declaration. 936 differed only in that segment, 938 were minted on `Record`
   and friends.

Current `lost` (`exact` + `by name`), after both:

| rust | | typescript | |
|---|---|---|---|
| Field | 1,478 | Field | 2,467 |
| Method | 797 | Const | 145 |
| Function | 8 | Function | 146 (5 exact) |
| Const / Static / Module `*` | 0 | Method 75, Static 22, Property 3 |

ts RESOLVED 24,095 → 24,856, all +761 through `declared_by_its_type`. Dangling
first-party edges UNMOVED at rust 335 / ts 561 — every new edge is real.

## Next — largest first, all measured

1. **rust Field 1,478.** NOT the ladder: 91% of unresolved field sites never
   reach it. `binding_of`/`param_bindings` refuse if-let (1,455), closure
   params (1,046), for-bindings (940) and `let x = f()` (914). Needs
   `World::returns` reachable from a NAMED binding, not just a `call()`
   receiver.
2. **rust Method 797** — the same receiver typing, one reach up.
3. **ts Field 2,467** — now honestly ReceiverTypeUnknown (10,817) rather than
   mis-minted. Same receiver work, in the other language.
4. Re-export tables; macro-call and function-as-value use sites.

## Known-broken — do not build on

- `binding_of` (rust/walk.rs:409) MIS-TYPES `let p = T::new().parse()` as `T`,
  a live wrong edge. A one-hop return-type lookup was tried and REVERTED: it
  costs 18 real `PgStore` edges, because `T::connect().await.unwrap()` needs
  plumbing hops AND a cross-file return type. Do it with #1, not before.
- `acceptance.rs` is `#[cfg(test)]` at its PARENT, so `barrier::test_boundary`
  reads its `#[test]` fns as source. Fourth classifier bug of this probe.
- `constructor` is a node (33 ts); no use site can ever name one.
- Java 26,995 dangling stands; Lombok 10,403 + Spring Data 2,529 not attempted.
- A7: 525 colliding identities. A2 ts drops 14; java 9.
