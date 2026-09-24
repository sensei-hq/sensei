# Checkpoint

**Slice:** indexer v2 — languages, before the wipe + re-index (issue #130, phase `build`)

## Done

- **rust + TypeScript cut over**, both A7 ratchets at **zero** (`2e18e352`, `0efc151f`). v1's TS/Svelte/Vue parsers deleted (2,426 lines); Vue *migrated* to `indexer/lang/vue.rs`.
- **Java's A7 gate built** (`74748f3b`) — it never existed, which is the same absence that hid 511 TypeScript collisions. `SENSEI_CORPUS`-driven over 5,088 real files.
- **Java's blocker cleared** (`6acb528b`) — overload collisions **409 → 14** by treating an overload like rust's `cfg` variant: the callable a caller mints, emitted once, plus one `MemberVariant` arm per signature.

## Remaining

| language | files (watched roots) | v2 adapter | state |
|---|---:|---|---|
| **Java** | 12,630 | ✓ | **unblocked — ready to flip** |
| SQL (+ddl) | 7,435 | ✗ | build |
| Python | 562 | ✓ | needs an A7 measurement first |
| Kotlin | 247 | ✗ | build |
| C (+h) | 160 | ✗ | build |
| Swift | 2 | ✗ | effectively unused |
| **C#** | **19,404** | ✗ | **nothing handles it, either version** |
| PHP | 2,251 | ✗ | nothing handles it |

Then: deploy → `TRUNCATE sensei.nodes, sensei.edges CASCADE` → full re-index → acceptance against the live graph → wire `Stated::Gone`.

## Next command

```
SENSEI_CORPUS=/Users/Jerry/Work/Dayamed cargo test -p senseid --bin senseid \
  indexer::lang::java -- --ignored --nocapture
```
Never pipe a test or build through `tail` — a pipe reports the pipe's exit status.

## Open questions

- Flip Java now, or build the missing adapters first and do one flip?
- C# is the largest single gap in the corpus and has never had an adapter in either version.

## Known-broken / not deployed

- **`languages/jvm.rs` is shared by `java.rs` and `kotlin.rs`** — flipping Java must keep `jvm.rs` for Kotlin. Same coupling shape as typescript/vue; `DetectionOnly` is how a flipped language keeps its extension→language mapping.
- **This repo has no Java/Python/Swift/Kotlin**, so acceptance cannot measure them — "java members 0 | owns 0" was an empty denominator, not a pass. Use `SENSEI_CORPUS`.
- Java's residual 14 collisions: declarations inside enum-constant or anonymous-class bodies, where `overloaded_in` scans only the type body's direct children.
- The daemon predates the v2 wiring, the `mark_file_parsed` fix and both cutovers. The wipe is required — reconcile reads `props->'claims'`, which 0 of 467,707 nodes carry.
