# Checkpoint

**Slice:** indexer v2 — before the wipe + re-index (issue #130, phase `build`)

## Done

`PRODUCTION_LANGUAGES = [Rust, TypeScript, Java, Python, CSharp, Php, C]` — 7 cut
over, ~5,600 lines of v1 deleted. A7 per language: Rust 0, TS 0, Java 14, Python 0,
C# 392 (all buckets explained), PHP 312 (cross-file only), C 11.

**Built, not flipped:** SQL both halves (`tsql` this crate's own lexer; Postgres over
dbd 0.14.0 `parse_sql`) — covers 86% of 7,435 files, `Unstated` 961 is why it waits.
Kotlin (245 files, A7 39) blocked on anonymous objects + Android flavours.

**Recent fixes:** UTF-16 decode owned by `classifiers::decode_source` (+377 files);
capture_drain's 8-day 176MB poison pill (`66af0d97`); names-are-not-programs
(`4aca8f99`) — 357 edges were named by source text, 10 broke an index and cost
9 files every symbol they had; transcript ingestion had no schedule at all.

## Remaining

| | files | blocker |
|---|---:|---|
| SQL flip | ~1,013 would go dark | `Unstated` dialect |
| Swift | 2 | `tree-sitter-swift =0.6.0` |
| Kotlin flip | 245 | two named shapes |

Then: deploy → `TRUNCATE sensei.nodes, sensei.edges CASCADE` → re-index →
acceptance → wire `Stated::Gone`.

## Next command

```
make install-service    # ship 4aca8f99 + the transcript scheduler to the daemon
```

## Known-broken

- **The parse stack is still unbounded — but the known trigger is gone.** `deba28fb`
  widened the globs to `**/*.min.*.js`; the 9.4MB file was a vendored Syncfusion
  bundle nothing references. Both watch roots now parse clean (9,697 + 17,953 files,
  0 programs), so the wipe is no longer blocked on it. An input nobody has seen yet
  still aborts the process rather than one file — options in `docs/backlog.md`.
- Daemon log is 18 GB. `log_prune` runs daily.
- Grammars pinned for ABI: c-sharp `=0.23.1`, php `=0.23.11`, c `=0.23.4`, swift `=0.6.0`.
- This repo has no Java/Python/C#/Kotlin/PHP/Swift — use `SENSEI_CORPUS`.
- Never pipe a test or build through `tail`; you read the pipe's exit status.
