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

**In progress — #161 (typed rows):** 42 left in `STILL_POSITIONAL` (`typed_rows_tests.rs`).
**Next:** `insert_edge_with_props` (19 callers) and `upsert_folder` (36), each with a round-trip test.

**Watch roots:** the live DB watches only sensei on purpose until the indexer covers JS/TS/Java/.NET;
then ~/Developer + ~/Work. Test/e2e use ~/Developer/sensei-hq.

**Open (the user's):**
- Approve `deploy-dojo` for v0.20.2 (no-op) and the v0.20.1 one still pending (also a no-op).
- dbd 0.27.7 (lockfile advisories; could carry dbd#107). Needs a go.
- #331: production count (counts only):
  `psql "$PROD" -Atc "select count(*) from dojo.tenants where origin='personal' and key ~ '-[0-9]+$'"`.

**Known:** the dōjō Workers build fails intermittently on PR/develop commits (it passed on main). The
Cloudflare MCP needs `/mcp` auth to read build logs. qlty is out of minutes. After a DDL change, run
`make test-template`.
