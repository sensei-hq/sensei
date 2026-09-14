# Checkpoint — the indexer

**State: the JS/TS/Svelte reader is BUILT and measured over 998 real files, 0
unreadable. Stage 4b's done-gate is met. Suite green: 3,468 passing, 0 failing;
clippy -D warnings and fmt clean. Cutover BLOCKS on 223 rust regressions —
down from 756 because the gate now knows which of them were the LEGACY producer
being wrong. The 223 are real and are a number, not a judgement call.**

Read: `docs/plans/indexer-sequence.md`, then `docs/spec/indexer/04b-walk-js.md`
(§7 decisions, §8 measurements are new), then `docs/design/indexer.md`.

## Slice

Build the JS/TS/Svelte reader behind a language-adapter trait; retire the
legacy indexer at cutover, not before. Pre-release DB: `dbd reconcile`.

## Done / remaining

| | |
|---|---|
| 0–9, and 10 S1/S2 (the gate, run) | `git log --grep=indexer` |
| clippy baseline to zero; `v2` out of every name | `4ede9c5d`, `cbb42e96` |
| `LanguageAdapter` trait + registry; rust behind it, split | `44b40a46` |
| 4b — the JS/TS/Svelte reader, corpus-verified | `630275d5` |
| 10 S3–S7 — switch rust, re-index, retire legacy | BLOCKED on the gate |

## The gate: 397, and it went UP when the identities got RIGHT

    resolved  legacy 8,104 -> current 11,859   (was 11,774)
    IMPROVEMENT 8,448 | REGRESSION 397 | EXPLAINED 4,296 | UNCLASSIFIED 0

                             before anchoring   after
    ghost, nothing declares          496         344   <- -152
    legacy self-disagreement          37          37
    identity disagreement             46          71   <- +25, NEW, unexplained
    REAL LOSS OF REACH               177         326   <- +149
    reported REGRESSION              223         397

**Read the direction before reading the number.** Anchoring a member to its
TYPE's module (634 declarations moved) made this indexer DECLARE 152 things it
previously did not, at the spelling legacy uses. They stop being "legacy
pointed at nothing" and become honest real losses: we declare it and still do
not connect the reference. The classifier got more truthful, not the graph
worse — resolved targets went UP by 85.

**The +25 identity disagreements are a real new problem.** Anchoring moved 25
members to a module legacy disagrees with. Small, unexplained, and worth a
sample before cutover.

**Minting the same string is not the ladder resolving it.** 2,538 references
now mint what their declaration mints, so they meet by the merge contract (§2)
at persist time — but the gate counts LADDER-resolved targets only, so that
gain does not appear here and is UNMEASURED. The ladder has no rung for "a
member of a type whose home I know"; that rung is what would convert the 326.

## Next command

    cargo test -p senseid --bin senseid -- --ignored --nocapture \
      indexer::differential::corpus::legacy_and_current_over_this_repos_rust

## Known-broken — do not build on

- **The JS reader has NO differential against the legacy producer.** The rust
  gate exists; the JS one does not, so "is this better for js/ts" is UNMEASURED.
  Build it before cutover switches them.
- `delete_folder` issues a path-prefix `DELETE` (`process.rs`), which 09 S7
  forbids. Fix is N file reconciles; blocked on stage 10 wiring reconcile.
- The watcher has no manifest/lockfile branch (09 S9), and LOCKFILE PATHS ARE
  PERSISTED NOWHERE — stage 2 stores them, or the watcher probes per event.
- `demote_symbol` nulls a node's file but keeps `target_id`, so consumers read
  it as resolved. Fixed by 07 S3–S6.
- `library_content.package_name` has no writer. `sensei_test` accumulates
  fixture rows and `metric_status` cross-joins them.
