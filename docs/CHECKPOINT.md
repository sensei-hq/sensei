# Checkpoint — indexer v2

**State: STAGES 0–7 LANDED. The suite is GREEN — 3030 passing, 0 failing, fmt
clean, no clippy finding in the diff. v1's `nodes.file_path` and the `lib_*`
kinds are gone from every reader. Stages 8–10 remain.**

Read in order: `docs/plans/indexer-v2-sequence.md` (master plan, 12 stages),
then the stage spec under `docs/spec/indexer/`, then `docs/design/indexer-v2.md`.
`docs/plans/indexer-v2-rust.md` is SUPERSEDED (marked at its top).

## Slice

Retire v1, implement v2. Pre-release: `dbd reconcile`, never hand-written
migrations. DDL is applied to `sensei` and `sensei_test`.

## Done / remaining

| | |
|---|---|
| 0–3 structure, 2b libraries | see `git log --grep='indexer-v2'` |
| **4–7 — the whole graph reads through `file_id`** | `6b5fe44f` |
| 8 — commands | TODO |
| 9 — incremental | TODO |
| 10 — cutover (v1 paths deleted) | TODO |

## Next command

    cat docs/plans/indexer-v2-sequence.md      # stage 8
    cargo test -p senseid --bin senseid         # the baseline to keep green

## Known-broken / known-wrong — do not build on

- **`folder_kind.standalone`** is dead in v2's model but v1 scan paths still
  write it (`scan.rs` ×4, `project_detail.rs`). It goes in stage 10.
- **`demote_v2_symbol`** keeps a node and nulls its file, so `target_id` stays
  non-null and consumers read it as resolved (67,839 edges measured). Fixed by
  `07-reconcile.md` S3–S6.
- **`v2_edges_contributed_by`** narrows with `AND (s.fqn = ANY($3) OR
  s.resolved = false)`, so an edge whose source this file deleted, resolving
  elsewhere, is never revisited. Fixed by S7.
- **`library_content.package_name`** has no writer: nothing states which package
  a page documents, and inferring it from the component name is the R4 guess.
- **`sensei_test` accumulates fixture rows.** Nothing cleans `_test:` metrics or
  `test/` repositories, and `metric_status` cross-joins them — it reached 88M
  rows and a summary query failed on disk. `DELETE FROM sensei.metrics WHERE key
  LIKE '\_test:%'` + the same for `repositories.repo_key LIKE 'test/%'`.
- Clippy is not clean on `develop` (80 findings, all in v2 modules with no
  caller yet — stage 10 wires them). None are in the stage 4–7 diff.

## Open questions

None blocking. Every earlier one is resolved in the git history.
