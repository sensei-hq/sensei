# Checkpoint — the indexer

**State: naming stripped of `v2`, clippy baseline at ZERO, suite green (3,434
passing, 0 failing). Building the JS/TS reader next (`04b-walk-js.md`). The
stage-10 cutover gate STILL BLOCKS on 777 rust regressions — that disposition
is a judgement call and is unchanged by this work.**

Read in order: `docs/plans/indexer-sequence.md`, then
`docs/spec/indexer/04b-walk-js.md`, then `docs/design/indexer.md`.
`docs/plans/indexer-rust-superseded.md` is SUPERSEDED (marked at its top).

## Slice

Build the JS/TS/Svelte reader behind a language-adapter trait; retire the
legacy indexer at cutover, not before. Pre-release DB: `dbd reconcile`, never
hand-written migrations.

## Done / remaining

| | |
|---|---|
| 0–9, and 10 S1/S2 (the gate, run) | `git log --grep=indexer` |
| clippy baseline to zero | `4ede9c5d` |
| `v2` out of every name, doc and spec path | this commit |
| adapter trait + registry in `indexer::lang` | NEXT |
| rust behind `RustAdapter`, module split | pending |
| `Language` gains JavaScript/TypeScript/Svelte | pending |
| flow-sensitive bindings, JS/TS reader, Svelte | pending |
| 10 S3–S7 — switch rust, re-index, retire legacy | BLOCKED on the gate |

## Next command

    cargo test -p senseid --bin senseid -- indexer::lang

## THE GATE BLOCKS — the open decision (unchanged)

    IMPROVEMENT 7,805 | REGRESSION 777 | EXPLAINED 3,861 | UNCLASSIFIED 0

Of the 777: 486 `ReceiverTypeUnknown`, 146 `NoImportInScope`, 1 `Denylisted`,
144 the name appears at NO use site. For the 633 seen-and-declined the question
is whether the legacy edge was CORRECT; sampling a dozen by hand answers it.

## Known-broken — do not build on

- `delete_folder` issues a path-prefix `DELETE` (`process.rs`), which 09 S7
  forbids. Fix is N file reconciles; blocked on stage 10 wiring reconcile.
- The watcher has no manifest/lockfile branch (09 S9). LOCKFILE PATHS ARE
  PERSISTED NOWHERE — stage 2 must store them or the watcher probes per event.
- `demote_symbol` keeps a node and nulls its file, so `target_id` stays
  non-null and consumers read it as resolved. Fixed by 07 S3–S6.
- `library_content.package_name` has no writer.
- `sensei_test` accumulates fixture rows; `metric_status` cross-joins them.
