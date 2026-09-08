# Checkpoint

**Slice:** #130 per-language indexer gaps → MCP tool surface (#148, #151, #152). Branch `develop`.

## Done — all gated: fmt 0, clippy 0, 3,147 tests / 0 failed

- `24289b29` **search: bound the ANN query.** `ORDER BY embedding <=> $2` computed the
  distance to rank by then discarded it, so a query for a symbol existing NOWHERE
  returned its k nearest strangers as a confident list. Ceiling + `relevance` returned.
- `24cf2703` **search: score the lexical arm too.** Exact name match never dropped (1.0);
  a row with no distance is kept UNSCORED, never invented.
- `28336db9` **rust: a sibling module is not a crate.** `use super::scan_logic;` +
  `scan_logic::f()` minted `lib·scan_logic·…`, giving real functions a phantom lib twin
  callers resolved to instead.
- `5e898712` **mcp: partition callees by `locality`** — library calls reported in full
  under `library_calls`, not interleaved. No denylist (a name can't tell `Vec::new`
  from `PgStore::new`).
- `4bfb1636` test pinning the `use super::x::{self, …}` group form.

## Verified live after a FORCED FULL REINDEX (all 48,654 files)

| | before | after |
|---|---:|---:|
| phantom lib twins | 639 | **433** |
| local-module phantoms | 22 | **5** |
| `calls` edges | 411,303 | **481,924** |
| `get_callees(scan_root)` | 55 interleaved | **45 first-party + 10 library** |
| `search("retract_undefined_stubs")` (exists nowhere) | 10 "retry" hits | **0** |
| `search("clear_scan_state_for_root")` | 27, top unscored | **3, exact first @1.0** |

`classify_folders`/`find_git_folders`/`is_excluded`/`all_directories` moved to the
first-party side — before the rust fix, the partition would have BURIED real deps.

**How to force a reindex** (`sensei scan` will NOT do it — `plan_reindex` skips on
(mtime, hash) and knows nothing about the indexer version):
`UPDATE sensei.config SET value='0.0.0-x' WHERE key='daemon.last_version';` then
`sensei restart`. Gate is committed only after the queue drains (crash-safe by design);
it was restored to `0.9.1` manually here because embedding backfill was still running.

Threshold 0.45 is MEASURED (corpus p01 0.4865, median 0.8892, relatives 0.12–0.26).
Tightening to 0.40 was **refuted live** — it cut a correct NL hit at 0.559. Do not retry.

## Next

1. **#151 not started** — `get_callees` 44% blind on rust (24 resolved / 31 unresolved on
   `scan_root`, unchanged by the reindex). `ctx.pg().method()` receivers are unresolvable:
   `type_of_value` only covers `let x = Type::new()`. The receiver's type is the RETURN
   TYPE of `pg()`, declared in another file — so a per-file producer cannot see it. Likely
   needs a post-index link phase (like the existing moniker merge), not a producer change.
2. **#152 open** — 5 survivors are `imports` edges, not calls: the rust IMPORT resolver
   never consults `local_modules`, so a test's `mod common;` + `use common::X;` mints
   `lib·common·common`. Separate code path from the one fixed. (My glob guess was wrong;
   verified and corrected on the issue.)
3. `use tokio::time;` + `time::sleep()` → `lib·time·time·sleep`: alias keyed on the module
   segment, not the crate. Correctly external, imprecisely named.
4. 1 folder failed the reindex: `cluster:scheduler`. Not diagnosed.

## Issues
Updated #146, #147, #148, #152. Filed #151, #152.
