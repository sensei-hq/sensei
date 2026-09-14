# Checkpoint — the indexer

**State: the JS/TS/Svelte reader is BUILT and measured over 998 real files, 0
unreadable. Stage 4b's done-gate is met. Suite green: 3,468 passing, 0 failing;
clippy -D warnings and fmt clean. Judged against §6 and R8 now, not against the
legacy producer: A3, R5 and R8 PASS over the real corpus; A7 reports 1,114
colliding identities with one dominant, named cause.**

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

## ONE canonical report — read this, do not retype it

    cargo test -p senseid --bin senseid -- --ignored --nocapture \
      indexer::acceptance::report

Prints a fixed table: every `Reason` as a row (present or zero), every language
as a column. It exists because a number retyped into prose acquires a new label
each time and a reader cannot tell a movement from a rephrasing. The other
acceptance tests are `indexer::acceptance::*`.

Last run, 1,331 files:

    references     rust   typescript
    RESOLVED      48,677     22,389
    NoImportInScope  18,291    5,317
    ReceiverTypeUnknown 28,627 24,597
    AmbiguousCandidates  8,508        0
    Denylisted       17,678      3,475
    DynamicDispatch       0      1,328
    UnhandledForm         1        165
    Unplaced / NoDeclaredType / MacroExpansion: 0 everywhere

    declarations  13,368     13,074
    relations      5,319      4,021
    imports 1st-party 2,036   2,299 | external 1,455 / 3,270

## Acceptance status

| | |
|---|---|
| A1 import-named targets resolve | PASS, ratcheted (rust 18,291 / ts 5,317 misses) |
| A3 every miss named, 100% | PASS both languages |
| A7 one declaration, one identity | 708 (rust 13, ts 695), ratcheted per language |
| R5 internal vs external | PASS, 153 external packages |
| R8 seven patterns derivable | PASS — 42 Facades, 62 Adapters from nodes+edges |
| A2 zero references dropped | RUST ONLY — no independent oxc counter yet |
| A4 / A5 / A6 / A9 | not built as corpus checks |

## Known-broken — do not build on

- **A local declaration is named at module scope** — A7's 708. A `static` or
  `const` inside a `fn` body mints at module scope, so three unrelated `RE`s in
  one file are one node. The walk needs a container for a function body; a
  `#[cfg(feature)]` pair is a SECOND, different cause and may be one to tolerate.
- **A2 has no TypeScript half.** The Rust reference count is checked against an
  independent counter written from the other side; the oxc side is unchecked, so
  a dropped JS reference would be invisible.
- `delete_folder` issues a path-prefix `DELETE` (`process.rs`), which 09 S7
  forbids. Fix is N file reconciles; blocked on stage 10 wiring reconcile.
- The watcher has no manifest/lockfile branch (09 S9), and LOCKFILE PATHS ARE
  PERSISTED NOWHERE — stage 2 stores them, or the watcher probes per event.
- `demote_symbol` nulls a node's file but keeps `target_id`, so consumers read
  it as resolved. Fixed by 07 S3–S6.
- `library_content.package_name` has no writer. `sensei_test` accumulates
  fixture rows and `metric_status` cross-joins them.
