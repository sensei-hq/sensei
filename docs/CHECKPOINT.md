# Checkpoint — 2026-10-26 (tracker metrics write-back)

**Slice:** tracker metrics write-back, on branch `feature/tracker_metrics_writeback`.
**Stage:** design done, and the gate passes. Next is build.
- Analysis: `docs/analysis/2026-10-26-tracker-metrics-writeback.md`
- Spec (reviewed): `docs/spec/daemon/tracker-metrics-writeback.md`
- Plan: `docs/plans/2026-10-26-tracker-metrics-writeback.md`. It has F1–F6, and its issues are TBD (no `gh`).

**User decisions (2026-10-26):**
- Only the assignee's daemon writes.
- Write to mapped custom fields, or else to one sensei comment edited in place.
- Write on every sync tick.
- Write all metrics.
- Store the mappings in `tracker_fields`, one row per metric.
- Write-back is on by default, for existing boards too.
- Reverse the read-only rule in the PRD, decisions.md and design §2.6.

**Claims that verification found false, and what changed:**
- The keyring status is 500 (not 503), so W7 now reuses connect's statuses.
- Tracker SQL lives in both `trackers.rs` and `tracker_usage.rs`.
- `api-trackers.ts` doesn't exist; the app wrappers are `try*` methods on `senseiApi` in `api.ts`.
- Gate 13's pattern matched the keyring delete.

**Claims that were misleading, and what changed:**
- A re-connect also re-keys sibling boards, on a new connect as well as a re-connect, so W11 resets their owner and write state.
- `metered_cost` isn't USD, so no `$` is shown for it.
- Discovery holds the transport factory, so gate 13 now checks who calls `write_transport(`.
- An empty-body 2xx is mapped for writes only.
- A keyring failure in the pass is now recorded in `writeback_error`.

**Next:** `/sensei:build` F1 (`AccountRef`, `OwnerLookup`, Jira `owner()`), red-first against gate 2.
Re-run the claims ledger first.

**Blocks the F2 merge:** a Jira tenant for gate 27 (X1–X8). It needs a person token, an app token and two assignees.

---

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

**In progress — #161 (typed rows):** 38 left in `STILL_POSITIONAL` (`typed_rows_tests.rs`), none at 7+ args.
Latest: `EdgeRow`/`EdgeTarget` (ee8fabd2, 4950eff0), `FolderRow` (0d6d67df), `TaskExecutionStart`.
**Next:** the 14 at six args, starting with the writers (`upsert_transcript_turns`,
`upsert_library`, `upsert_pattern`, `create_extension`, …). Each gets a round-trip test and a swapped-bind probe.

**Watch roots:** the live DB watches only sensei on purpose until the indexer covers JS/TS/Java/.NET;
then ~/Developer + ~/Work. Test/e2e use ~/Developer/sensei-hq.

**Open (the user's):**
- Approve `deploy-dojo` for v0.20.2 (no-op) and the v0.20.1 one still pending (also a no-op).
- dbd 0.27.7 (lockfile advisories; could carry dbd#107). Needs a go.
- #331: production count (counts only):
  `psql "$PROD" -Atc "select count(*) from dojo.tenants where origin='personal' and key ~ '-[0-9]+$'"`.

**Develop was red on a4e7171c:** a tracker test raced its own setup; 052d9b4f fixes the test (not reproduced
locally, 0/120). Confirm the next develop run is green.

**Known:** the dōjō Workers build fails intermittently on PR/develop commits (it passed on main). The
Cloudflare MCP needs `/mcp` auth to read build logs. qlty is out of minutes. After a DDL change, run
`make test-template`.
