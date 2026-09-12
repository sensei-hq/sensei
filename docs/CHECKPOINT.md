# Checkpoint — indexer v2

**State: STAGES 0–9 LANDED (9 is its pure half; its DoD is met bar one item
blocked on 10). Suite GREEN — 3052 passing, 0 failing, fmt clean, no clippy
finding in any of the three commits. Stage 10, the cutover, is what remains.**

Read in order: `docs/plans/indexer-v2-sequence.md` (master plan), then
`docs/spec/indexer/10-cutover.md`, then `docs/design/indexer-v2.md`.
`docs/plans/indexer-v2-rust.md` is SUPERSEDED (marked at its top).

## Slice

Retire v1, implement v2. Pre-release: `dbd reconcile`, never hand-written
migrations. DDL is applied to `sensei` and `sensei_test`.

## Done / remaining

| | |
|---|---|
| 0–3 structure, 2b libraries | `git log --grep=indexer-v2` |
| **4–7 — the graph reads through `file_id`** | `6b5fe44f` |
| **8 — the v2 stages reach the progress stream** | `c13ae29c` |
| **9 — incremental, pure half** | `68c88f8f` |
| 10 — cutover: wire the v2 pipeline, delete v1 | TODO |

## Next command

    cat docs/spec/indexer/10-cutover.md
    cargo test -p senseid --bin senseid          # the baseline to keep green

## Known-broken / known-wrong — do not build on

- **`delete_folder` issues a path-prefix `DELETE`** (`process.rs`), which 09 S7
  forbids. The fix is N file RECONCILES; blocked on stage 10 wiring v2's
  reconcile. This is stage 9's one unmet DoD item.
- **The watcher has no manifest/lockfile branch.** A changed `Cargo.lock` is
  enqueued as a plain `ProcessFile`, parses as nothing, and the graph keeps
  yesterday's dependency set until a full re-scan runs (09 S9, live today).
  `incremental::retrigger_for` fixes it. OPEN QUESTION: `manifest_dirs` is
  derivable from `folders` (D11), but LOCKFILE PATHS ARE PERSISTED NOWHERE —
  either stage 2 stores them or the watcher probes per event.
- **`folder_kind.standalone`** is dead in v2's model; v1 scan paths still write
  it (`scan.rs` x4, `project_detail.rs`). Goes at stage 10.
- **`demote_v2_symbol`** keeps a node and nulls its file, so `target_id` stays
  non-null and consumers read it as resolved (67,839 edges). Fixed by
  `07-reconcile.md` S3-S6.
- **`v2_edges_contributed_by`** narrows with `AND (s.fqn = ANY($3) OR
  s.resolved = false)`, so an edge whose source this file deleted, resolving
  elsewhere, is never revisited. Fixed by S7.
- **`library_content.package_name`** has no writer: nothing states which package
  a page documents, and inferring it from the component name is the R4 guess.
- **`sensei_test` accumulates fixture rows.** Nothing cleans `_test:` metrics or
  `test/` repositories, and `metric_status` cross-joins them — it reached 88M
  rows and a summary query failed on disk. `DELETE FROM sensei.metrics WHERE key
  LIKE '\_test:%'` plus the same for `repositories.repo_key LIKE 'test/%'`.
- **Clippy: 88 findings on `develop`**, all `never used` in v2 modules with no
  caller yet. Stage 10 is where that count should go to ZERO — it is now the
  tenth capability built ahead of its caller, and the pattern is the design's
  most repeated defect.

## Open questions

Only the lockfile-persistence one above. Everything else is resolved in git
history.
