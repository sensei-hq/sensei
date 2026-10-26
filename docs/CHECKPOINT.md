# Checkpoint — 2026-10-26

**Slice:** issue burndown (docs/roadmap/08-issue-burndown.md), wave 5.
**Released v0.20.2** (user go, 2026-10-25): PR #337 merged, tag `v0.20.2`; builds, GitHub release and tap done.
The dōjō Worker serves 0.20.2 (`5fd2eb8`). **`deploy-dojo` is waiting for the user's approval** (no dōjō DDL
in this release). Local daemon on 0.20.2.

**Closed:** #178 and #188, both verified on the released daemon. Earlier this run: #106, #335, #201, #204,
#183, #142, #298, #172, #195.

**Waiting on observation (released in 0.20.2):**
- #336: a week with no `ingest_capture` abandonment (`GET /api/tasks/abandoned`).
- #125: on 0.20.2, 72 → 22 orphan turn sets. 5dea36e3 (develop, next release) ignores the 4 fresh orphans
  that have hook events, leaving 18 with no transcript. Check: count after the next release.

**In progress — #161 (typed rows):** 25 left in `STILL_POSITIONAL` (`typed_rows_tests.rs`), none at 7+ args.
Row types are declared in `pg_store/mod.rs` (user decision 2026-10-26, docs/decisions.md). Re-exporting them
dangled in the code graph; the `indexer::resolve` ratchet went 1,304 → 1,045 and its ceiling is now 1,060.
**Next:** the last 6-arg one, `upsert_subfolder_kind` (13 callers), then the 5-arg list. A new row goes in `mod.rs`, with a round-trip test and a swapped-bind probe.

**Watch roots:** the live DB watches only sensei on purpose until the indexer covers JS/TS/Java/.NET;
then ~/Developer + ~/Work. Test/e2e use ~/Developer/sensei-hq.

**Released 2026-10-26:** dbd 0.27.7 (user go). dbd#107 fixed and closed; the real cause was a dropped
comment on an ADD COLUMN, not line breaks. sensei pins it (af77da72). v0.20.2's `deploy-dojo` succeeded.

**Open (the user's):**
- Reject the stale v0.20.1 `deploy-dojo` (v0.20.2 already deployed over it).
- #331: production count (counts only), run by the user, since there is no prod connection here:
  `psql "$PROD" -Atc "select count(*) from dojo.tenants where origin='personal' and key ~ '-[0-9]+$'"`.

**Known:** the dōjō Workers build fails intermittently on PR/develop commits (it passed on main). The
Cloudflare MCP needs `/mcp` auth to read build logs. qlty is out of minutes. After a DDL change, run
`make test-template`.
