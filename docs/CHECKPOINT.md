# Checkpoint — 2026-10-18

**Slice:** #268, the dōjō diagram screens (next). #225 is done; #238 is built.

**Done** (pushed to develop through 56bb5d4f; 3,535 Rust tests pass; rebased on PR #270):
- #225 live verification, whole checkout: `synced`, all 7 tables equal local.
  - `pg_store/tests.rs` is split into 18 `tests/<topic>_tests.rs` modules, with the same 353 tests.
  - New `sync_state` state `partial`: agreed except the named files; `synced_at` is set.
  - A file awaiting its parse is held back from the graph push (`graph_parse_pending`).
  - Earlier fixes: unnamed index nullable; deleted files leave the index; run.sh scope routing.
- #238 steps 2–5, built and live-verified: the usage, routing, gate and drive mods.

**Remaining:** #268 dōjō diagram screens → #254 JPA 8b. #238 stays open until `make bump`.
Filed #272: `.py`/loose `.sql` never record a parse outcome (they never reach the dōjō).

**Next command:** `/sensei:session refocus`, then #268. The fixture is the local Supabase tenant
`personal/graphdev`, holding this repository's develop graph. Start the Worker with
`cd dojo && bun run dev -- --host 127.0.0.1` (note: dojo/CLAUDE.md now says wrangler `--port 5175`).

**Open questions:**
- Does `--resume` keep the session id and reset /cost? (`GREATEST` would under-count)
- Should `check_hooks` WARN when the mods module is missing?

**Known-broken / pending:**
- The mods, `list_cost_usd` and `partial` reach installs only at the next release
  (bootstrap reconcile converges the CHECK; verified on a scratch database).
- The committed `database/design.dbml` is stale against `dbd dbml` (pre-existing).
