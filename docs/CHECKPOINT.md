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

## The gate: 223, re-measured over 385 files

    resolved  legacy 8,076 -> current 11,774
    IMPROVEMENT 8,356 | REGRESSION 223 | EXPLAINED 4,435 | UNCLASSIFIED 0

    496  ghost — NOTHING declares what legacy pointed at   } 533 EXPLAINED:
     37  legacy self-disagreement (trait-qualified)        } legacy was wrong
     46  identity disagreement — the two grammars MINT differently  } 223 still
    177  REAL LOSS OF REACH                                        } blocking

`explain_dangling` reclassifies the first two: legacy's reference side derives a
target's module from the CALL SITE, so those targets name nothing that exists,
and R4 already says a wrong edge is worse than a missing one. The other two keep
blocking on purpose — an identity disagreement is THIS indexer's grammar
differing, and hiding it behind a rule about legacy's mistakes is the failure
the gate exists to prevent.

**The 177 real losses have a NEW dominant cause.** It was 184-of-205
`ReceiverTypeUnknown`; the receiver routes cut that to 12. Now:

    145  NoImportInScope   <- the next thing to fix
     20  no reference emitted at any use site
     12  ReceiverTypeUnknown

Do not compare a regression count across commits without re-running: the corpus
is THIS repo's Rust, so every commit that adds Rust moves the denominator.

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
