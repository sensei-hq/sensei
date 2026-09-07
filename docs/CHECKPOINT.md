# Checkpoint

**Slice:** #130 per-language indexer gaps → MCP tool surface (#148, #151, #152). Branch `develop`.

## Done this session — all gated: fmt 0, clippy 0, 3,147 tests / 0 failed

- `24289b29` **search: bound the ANN query.** `ORDER BY embedding <=> $2` computed the
  distance to rank by then threw it away, so a query for a symbol existing NOWHERE
  returned its k nearest strangers as a confident list. Added a distance ceiling and
  returned the distance as `relevance`.
- `24cf2703` **search: score the lexical arm too.** `ILIKE '%term%'` carried no
  relevance. Exact name match never dropped (1.0); a row with no distance is kept
  UNSCORED, never invented.
- `28336db9` **rust: a sibling module is not a crate.** `use super::scan_logic;` +
  `scan_logic::f()` minted `lib·scan_logic·…`, giving real functions a phantom lib twin
  callers resolved to instead. 979 lib_symbols share a name with a real in-repo rust fn.
- `5e898712` **mcp: partition callees by `locality`** — library calls reported in full
  under `library_calls`, not interleaved. No denylist: a bare name cannot tell
  `Vec::new` from `PgStore::new`.
- `4bfb1636` test pinning the `use super::x::{self, …}` group form.

## Measured live

| | before | after |
|---|---|---|
| `search("retract_undefined_stubs")` (exists nowhere) | 10 "retry" hits | **0** |
| `search("clear_scan_state_for_root")` | 27, top unscored | **3, exact first @1.0** |
| `search("what parses svelte script blocks")` | 18 | **4, all svelte** |
| `get_callees(scan_root)` | 55 interleaved | **41 first-party + 14 library** |

Threshold 0.45 is MEASURED: corpus p01 0.4865, median 0.8892, real relatives 0.12–0.26.
Tightening to 0.40 was **refuted live** — it cut a correct NL hit at 0.559. Do not retry.

## Known-broken / next

1. **The rust fix is not in the live corpus yet.** `sensei scan` skips files whose
   CONTENT HASH is unchanged; the indexer changed, not the source, so those files were
   never re-parsed and the 22 phantom local-module lib nodes remain. Proven by unit test
   on the real import form. **Next: forced full reindex (`version_rescan` clears
   `scan_state` per root), then re-measure the 979 twins.**
2. **#151 not started** — `get_callees` 44% blind on rust: `ctx.pg().method()` receivers
   unresolvable; `type_of_value` only covers `let x = Type::new()`. Needs a per-crate
   `fn name → return type` map.
3. **#152 partial** — `use tokio::time;` + `time::sleep()` mints `lib·time·time·sleep`:
   alias keyed on the module segment, not the crate. Correctly external, imprecisely
   named. Called out in the test, not fixed.
4. Corrected on #152: the `Array` "cross-language leak" was **my error** — rust callers
   resolve to `lib·serde_json·serde_json·Array`, which is correct.

## Issues
Updated #146 (stubs 108,174 → 38,945), #147 (TS/JS inheritance 0 → 873), #148. Filed #151, #152.
