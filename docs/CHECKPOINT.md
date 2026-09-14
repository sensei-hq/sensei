# Checkpoint — the indexer

**State: the JS/TS/Svelte reader is BUILT and measured over 998 real files, 0
unreadable. Stage 4b's done-gate is met. Suite green: 3,466 passing, 0 failing;
clippy -D warnings and fmt clean. Cutover STILL BLOCKS on 750 rust
regressions — unchanged by this work, and still a judgement call.**

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

## The gate: 750, and what they ARE

Latest run `35bdd61a`: resolved 11,434 (legacy 7,923), regressions 750. NOT 777
— that was the FIRST run, and three receiver routes landed after it
(`6a72f80f` →755, `940cc178` →756, `35bdd61a` →750).

Classified at 777 by `differential::why` (`d8803422`) and NOT re-classified, so
the shares below are stale even though the total is not:

    492  dangling — legacy pointed at an identity NOTHING declares
     46  identity disagreement · 36 legacy self-disagreement
    205  REAL LOSS OF REACH, of which 184 are ReceiverTypeUnknown

**574 of 777 were the LEGACY producer being wrong** — its reference side derives
the target's module from the CALL SITE. Not reach this indexer lost. The
receiver routes attacked exactly the 184, so the real-loss slice is smaller now
and nobody has measured how much. Re-running `differential::why` turns the
cutover decision from a judgement into a number.

## Next command

    cargo test -p senseid --bin senseid -- --ignored --nocapture \
      indexer::differential::why

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
