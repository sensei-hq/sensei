# Checkpoint

**Slice:** indexer v2 cutover — observability leg (issue #130, phase `build`)

## Done

- **v2 pipeline wired** (`888b97cc`) — `ScanRoot → ProcessGitFolder → ProcessManifest → ProcessRepoFiles` (gate) `→ ProcessFile`. v1's scan/process orchestration deleted.
- **Observability views** (`c00f7406`):
  - `sensei.graph_nodes` / `graph_resolution` carry `repository_id`/`repository` — direct `folders.repository_id` read, **95.1% at node grain** (the 1.4% figure is folder grain, the wrong denominator).
  - `activity.task_health` (every execution) + `activity.task_failures` (the restart list, keyed `(task_kind, folder_path, path)`, keeping only jobs whose *latest* run failed).
  - `sensei.error_signature(text)` collapses paths so failures group by cause.
- **Failure lake reclaimed** — rolled into `task_execution_daily` first (the 14-day rollup bound meant its 7 days had never been aggregated), then 17,577,049 rows deleted + `VACUUM FULL`: **14 GB → 656 MB**.

## Remaining

2. Align TS/JS adapters to rust v2 and add to `PRODUCTION_LANGUAGES`; requires deleting `languages/typescript.rs` + `javascript.rs`.
3. Port SQL, Swift, Kotlin, Vue, C.
4. Stage 10's differential harness — **does not exist**.
5. Cut over, re-index, acceptance; delete `languages/` and break its two couplings (`languages::is_test_path` ×2 in `indexer/barrier.rs`, `languages::fqn::is_external` ×1 in `indexer/community.rs`).

## Next command

```
cargo test -p senseid --bin senseid indexer::lang::javascript
```

## Open questions

- `javascript.rs` is 4,786 lines in one file vs rust's `mod`/`walk`/`types` split — **structural split undecided**.
- Failure retention has no cap on repeated identical failures, so the lake can refill. Fix would keep N exemplars per (kind, signature, day); not implemented.

## Known-broken / not deployed

- Running daemon **predates this session** — `parsed_at` is 0 on all 111,276 files (the `mark_file_parsed` fix is committed, not shipped), and the v2 pipeline is not live.
- `graph_resolution.resolved_via` is **structurally NULL on all 965,518 edges**: the only `rung` writer is `indexer/persist.rs` (v2) and every production edge came from v1. Same split leaves `EnumVariant`/`Field`/`Property`/`Trait`/`Macro`/`Static` at zero rows. Documented in `docs/spec/indexer/19-observability.md`.
