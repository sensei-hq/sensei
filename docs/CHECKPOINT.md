# Checkpoint — the indexer

**State: the JS/TS/Svelte reader is BUILT and measured over 998 real files, 0
unreadable. Stage 4b's done-gate is met. Suite green: 3,466 passing, 0 failing;
clippy -D warnings and fmt clean. The stage-10 cutover gate STILL BLOCKS on 777
rust regressions — unchanged by this work, and still a judgement call.**

Read in order: `docs/plans/indexer-sequence.md`, then
`docs/spec/indexer/04b-walk-js.md` (§7 and §8 are new), then
`docs/design/indexer.md`.

## Slice

Build the JS/TS/Svelte reader behind a language-adapter trait; retire the
legacy indexer at cutover, not before. Pre-release DB: `dbd reconcile`.

## Done / remaining

| | |
|---|---|
| 0–9, and 10 S1/S2 (the gate, run) | `git log --grep=indexer` |
| clippy baseline to zero | `4ede9c5d` |
| `v2` out of every name, doc and spec path | `cbb42e96` |
| `LanguageAdapter` trait + registry; rust behind it, module split | `44b40a46` |
| 4b — the JS/TS/Svelte reader, corpus-verified | this commit |
| 10 S3–S7 — switch rust, re-index, retire legacy | BLOCKED on the gate |

## Next command

    # the JS corpus measurement, re-run:
    cargo test -p senseid --bin senseid -- --nocapture \
      indexer::lang::javascript::tests::every_real_file

## What the JS reader reaches, measured (04b §8)

    998 files, 0 unreadable | 13,832 symbols | 60,391 refs | 4,021 relations
    receivers typed by a STATED route: 3,299 of 33,028
    ReceiverTypeUnknown 29,729 | Unplaced 29,133 | DynamicDispatch 1,364
    UnhandledForm 165

`UnhandledForm` at 165 says no common shape is missed. The 29,729 is the
CROSS-FILE question and is #174's, not this stage's. Do NOT compare 3,299/33,028
against 04b §2's projected 27% — different denominators (accesses vs calls).

## Known-broken — do not build on

- **The JS reader has no differential against the legacy producer.** The rust
  gate exists; the JS one does not, so "is this better than `crate::languages`
  for js/ts" is UNMEASURED. Build it before switching js/ts at cutover.
- `delete_folder` issues a path-prefix `DELETE` (`process.rs`), which 09 S7
  forbids. Fix is N file reconciles; blocked on stage 10 wiring reconcile.
- The watcher has no manifest/lockfile branch (09 S9). LOCKFILE PATHS ARE
  PERSISTED NOWHERE — stage 2 must store them or the watcher probes per event.
- `demote_symbol` keeps a node and nulls its file, so `target_id` stays
  non-null and consumers read it as resolved. Fixed by 07 S3–S6.
- `library_content.package_name` has no writer.
- `sensei_test` accumulates fixture rows; `metric_status` cross-joins them.
