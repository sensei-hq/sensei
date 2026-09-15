# Checkpoint — the indexer

**JS/TS/Svelte reader built. `impact.rs` answers n-depth blast radius with the
reason it stops; `sensei.graph_boundary` serves the same answer from SQL with
prose. Suite 3,484 / 0, clippy -D warnings + fmt clean. Stage 10 cutover gated.**

Read `docs/plans/indexer-sequence.md`, then `docs/design/indexer.md`.

## Three canonical reports — do not retype their numbers

    cargo test -p senseid --bin senseid -- --ignored --nocapture \
      indexer::acceptance::report      # resolve rate + reason, per language
      indexer::impact::tests::report   # blast radius at depth 3
    SENSEI_CORPUS=~/Work/Dayamed … indexer::lang::javascript::tests::a_foreign_corpus

    select source_language, reason_code, reason_summary, count(*)
      from sensei.graph_boundary where folder_id = $1 group by 1,2,3;

## Load-bearing decisions, with the measurement

- `Denylisted` + `ExternalBoundary` are verdicts, NOT doubt — excluded from a
  radius. Including them added 10,398 sites headed by `map`/`into`/`as_str`/
  `collect`, all std methods colliding by name.
- `graph_boundary` LEFT-joins `reason_codes`; INNER drops unseeded codes (test
  names this mutation). `Reason::as_label` is the one producer of the codes.
- Foreign corpus: A2 drops 2.0% on code nobody here wrote vs 0.03% on ours.
  Build remaining adapters corpus-first, never fixtures-first.

## Next

1. MCP reads `graph_boundary` at cutover, so `get_callers` answers with reasons.
2. Java adapter, corpus-first (~/Work/Dayamed: 8,010 java, 186 sql).

## Known-broken — do not build on

- A7: 708 colliding identities (a `static`/`const` in a fn body mints at module
  scope; a `#[cfg(feature)]` pair is a second, tolerable cause).
- A2 TypeScript drops 14 ours / 3,768 foreign. Head: `[a[0], a[1]] = …`.
- `delete_folder` issues a path-prefix DELETE (`process.rs`); 09 S7 forbids it.
- Watcher has no manifest/lockfile branch (09 S9); lockfile paths persisted nowhere.
- `demote_symbol` nulls a node's file but keeps `target_id`. Fixed by 07 S3–S6.
- `library_content.package_name` has no writer.
