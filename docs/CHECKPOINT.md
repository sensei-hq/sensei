# Checkpoint — the indexer

**Every edge now carries its provenance: a placed one says WHICH RUNG placed it,
an unplaced one says WHY. Both have prose in `sensei.reason_codes` and a
group-by view. MCP serves them. Suite 3,493 / 0, clippy + fmt clean. Stage 10
cutover still gated.**

Read `docs/plans/indexer-sequence.md`, then `docs/design/indexer.md`.

## Canonical reports — do not retype their numbers

    cargo test -p senseid --bin senseid -- --ignored --nocapture \
      indexer::acceptance::report      # resolve rate by reason AND by rung
      indexer::impact::tests::report   # blast radius at depth 3
    SENSEI_CORPUS=~/Work/Dayamed … indexer::lang::javascript::tests::a_foreign_corpus

    select source_language, reason_code,  count(*) from sensei.graph_boundary   …
    select source_language, resolved_via, count(*) from sensei.graph_resolution …

## Load-bearing decisions, each with its measurement

- Six rungs = six `Ladder` methods, so a wrong edge names the function to read.
  Only 2,150 of 83,562 placed edges (2.6%) rest on `through_a_glob`, the
  weakest. Rung totals must equal RESOLVED — asserted.
- `Denylisted` + `ExternalBoundary` are verdicts, NOT doubt — excluded from a
  radius. Including them added 10,398 sites headed by std methods (`map`,
  `into`, `as_str`, `collect`) colliding by name.
- Both views LEFT-join `reason_codes`; INNER drops unseeded codes (tests name
  this mutation). `Reason::as_label` / `Rung::as_label` are the sole producers.
- Foreign corpus: A2 drops 2.0% on code nobody here wrote vs 0.03% on ours.
  Build remaining adapters corpus-first, never fixtures-first.

## Next

1. Java adapter, corpus-first (~/Work/Dayamed: 8,010 java, 186 sql).
2. Stage 10 cutover. Until then the LEGACY indexer writes neither `props.reason`
   nor `props.rung`, so both surface null on live data. Honest, not broken.

## Known-broken — do not build on

- A7: 708 colliding identities (a `static`/`const` in a fn body mints at module
  scope; a `#[cfg(feature)]` pair is a second, tolerable cause).
- A2 TypeScript drops 14 ours / 3,768 foreign. Head: `[a[0], a[1]] = …`.
- `delete_folder` issues a path-prefix DELETE (`process.rs`); 09 S7 forbids it.
- Watcher has no manifest/lockfile branch (09 S9); lockfile paths persisted nowhere.
- `demote_symbol` nulls a node's file but keeps `target_id`. Fixed by 07 S3–S6.
- `library_content.package_name` has no writer.
