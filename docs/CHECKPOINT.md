# Checkpoint — 2026-10-26

**Slice:** issue burndown (docs/roadmap/08-issue-burndown.md), wave 5.
**Released v0.20.2** (user go, 2026-10-25): PR #337 merged, tag `v0.20.2`; builds, GitHub release and tap done.
The dōjō Worker serves 0.20.2 (`5fd2eb8`). **`deploy-dojo` is waiting for the user's approval** (no dōjō DDL
in this release). Local daemon on 0.20.2.

**Closed:** #331 (production count 0, user 2026-10-26), #178 and #188, both verified on the released daemon. Earlier this run: #106, #335, #201, #204,
#183, #142, #298, #172, #195.

**Waiting on observation (released in 0.20.2):**
- #336: a week with no `ingest_capture` abandonment (`GET /api/tasks/abandoned`).
- #125: on 0.20.2, 72 → 22 orphan turn sets. 5dea36e3 (develop, next release) ignores the 4 fresh orphans
  that have hook events, leaving 18 with no transcript. Check: count after the next release.

- #161 (853c46a5 and the guards after it): every pg_store function takes a typed row or query struct (≤4
  positional args), and each row is destructured exhaustively, guarded by `typed_rows_tests.rs`. The
  `too_many_arguments` allows are gone. Check: the next release's daemon runs it.

**In progress — #123:** A5 (0c2d452b) and A6 (7f0d82c6) fixed. A1–A4 were done earlier.
**Next:** #123's lower-severity list, mostly in `tools/session-report` (`units()` scan depth, the p90 vs median
turn-length gate, `--compare` contradictions, fabricated zero-length turns, `event_count` meaning three
things, `#` in a workspace path). Then Track B. Verify each before fixing: the original review had one false
claim.

**Watch roots:** the live DB watches only sensei on purpose until the indexer covers JS/TS/Java/.NET;
then ~/Developer + ~/Work. Test/e2e use ~/Developer/sensei-hq.

**Released 2026-10-26:** dbd 0.27.7 (user go). dbd#107 fixed and closed; the real cause was a dropped
comment on an ADD COLUMN, not line breaks. sensei pins it (af77da72). v0.20.2's `deploy-dojo` succeeded.

**Open (the user's):** none.

**Known:** the dōjō Workers build fails intermittently on PR/develop commits (it passed on main). The
Cloudflare MCP needs `/mcp` auth to read build logs. qlty is out of minutes. After a DDL change, run
`make test-template`.
