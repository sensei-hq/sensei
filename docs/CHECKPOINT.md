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

## Judged against the GOAL, not against the legacy producer

The differential harness is DELETED. "Does this agree with the producer being
replaced" is a transition question, and letting it set the agenda meant chasing
a number that says nothing about whether the graph is any good. §6's first line
was always right: thresholds, not comparisons.

`indexer::acceptance` runs §6 and R8 over the real corpus, every language:

    cargo test -p senseid --bin senseid -- --ignored --nocapture indexer::acceptance

    A3  every miss named, 100% accounted for, both languages   PASS
        rust       resolved 48,604 | unresolved 72,962
        typescript resolved 23,347 | unresolved 37,044
    R5  internal vs external           PASS
        local 4,479 | external 4,725 across 153 packages | globs 408
    R8  the seven patterns' facts all emitted, and DERIVED     PASS
        42 Facades, 62 Adapters, from nodes and edges alone
    A7  one declaration, one identity  1,114 COLLIDING (ratchet)

## A7's 1,114 have one head, and it is a real defect

A declaration inside a FUNCTION BODY is minted as if it sat at module scope.
Three `static RE` in three `fn`s of `adapters/manifest/gradle.rs` mint ONE
identity, so "where is `RE` defined" answers with whichever was written last.
Same shape in `provision_status`, `COPY_CAP`, `status_of`, `ARMS`, and
TypeScript's `deadline` — not a Rust quirk.

The repair is the walk giving a function body its own container. Whether a
local is named under its function or not emitted at all is a grammar decision
and is NOT yet made.

## Next command

    cargo test -p senseid --bin senseid -- --ignored --nocapture indexer::acceptance

## Known-broken — do not build on

- **A local declaration is named at module scope** — A7's 1,114, above. The
  walk needs a container for a function body.
- `delete_folder` issues a path-prefix `DELETE` (`process.rs`), which 09 S7
  forbids. Fix is N file reconciles; blocked on stage 10 wiring reconcile.
- The watcher has no manifest/lockfile branch (09 S9), and LOCKFILE PATHS ARE
  PERSISTED NOWHERE — stage 2 stores them, or the watcher probes per event.
- `demote_symbol` nulls a node's file but keeps `target_id`, so consumers read
  it as resolved. Fixed by 07 S3–S6.
- `library_content.package_name` has no writer. `sensei_test` accumulates
  fixture rows and `metric_status` cross-joins them.
