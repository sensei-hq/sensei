# Checkpoint — the indexer

**State: JS/TS/Svelte reader built and measured; `impact.rs` answers n-depth
blast radius WITH the reason it stops. Suite green (3,482 / 0), clippy -D
warnings and fmt clean. Stage 10 cutover still gated.**

Read: `docs/plans/indexer-sequence.md`, `docs/spec/indexer/04b-walk-js.md`,
`docs/design/indexer.md`.

## Two canonical reports — read these, do not retype the numbers

    cargo test -p senseid --bin senseid -- --ignored --nocapture \
      indexer::acceptance::report        # resolve rate, reason per language
      indexer::impact::tests::report     # blast radius, depth 3

Impact, 1,332 files: rust 7,893 subjects / 39,914 placed callers / 170,431
doubting sites; ts 2,029 / 1,515 / 23,471. Boundaries: receiver_type_unknown
151,214 | no_import_in_scope 41,736 | dynamic_dispatch 570 |
ambiguous_candidates 382. `Denylisted` and `ExternalBoundary` are EXCLUDED —
both are verdicts, not doubt; counting them added 10,398 sites headed by `map`,
`into`, `as_str`, `collect`, all std methods colliding by name.

## Reasons have prose, and the graph has a boundary view

`sensei.reason_codes` domain `code_graph` — 10 rows, one per `Reason`, with
summary/detail/remedy/actor. `Reason::as_label` is the only producer of the
codes; `every_reason_is_explained_by_a_seeded_reason_code` pins both directions.

    select source_language, reason_code, reason_summary, count(*)
      from sensei.graph_boundary where folder_id = $1 group by 1,2,3;

`graph_boundary` = the unresolved slice of `call_graph` LEFT-joined to the
vocabulary. LEFT is load-bearing: an unseeded code surfaces raw, never drops the
row. Applied to `sensei` and `sensei_test`; `dbd diff` in sync.

## Next

1. MCP reads `graph_boundary` at cutover so `get_callers` answers with the reason.
2. Java adapter, corpus-first — see the foreign-corpus finding below.

## A FOREIGN corpus says the JS/TS reader does not generalise

    SENSEI_CORPUS=~/Work/Dayamed cargo test -p senseid --bin senseid -- \
      --ignored --nocapture indexer::lang::javascript::tests::a_foreign_corpus

1,646 files, 0 unreadable. A2 dropped 3,768 of 191,383 (2.0%) against our own
corpus's 14 of 50,106 (0.03%). Seventy times the rate. Do NOT build the
remaining adapters against fixtures and this repo; `web_sources_under` takes a
root so any repo can be pointed at.

## Known-broken — do not build on

- A7: 708 colliding identities. A `static`/`const` in a fn body mints at module
  scope; a `#[cfg(feature)]` pair is a second, tolerable cause.
- A2 TypeScript drops 14 (ours) / 3,768 (foreign). Head: `[a[0], a[1]] = ...`.
- `delete_folder` issues a path-prefix DELETE (`process.rs`), which 09 S7 forbids.
- Watcher has no manifest/lockfile branch (09 S9); lockfile paths persisted nowhere.
- `demote_symbol` nulls a node's file but keeps `target_id`. Fixed by 07 S3–S6.
- `library_content.package_name` has no writer.
