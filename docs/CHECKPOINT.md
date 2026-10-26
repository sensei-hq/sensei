# Checkpoint — 2026-10-25

**Slice:** issue burndown (docs/roadmap/08-issue-burndown.md). Wave 4 is closed; wave 5 is in progress.
Released v0.20.0 (dōjō deployed; Worker `/version` = 0.20.0 `fcf5d87`) and v0.20.1 (daemon only).
Develop CI is green on 45bff79b.

**Closed this run:** #106, #335, #201, #204, #183, #142, #298, #172, #195 (#120 updated).

**Landed, each closing on the next release's binary:**
- #178 (2e5c6c1f): a file over 2 MiB, or with a line over 10,000 bytes, is skipped as `oversized`.
  Local DB reconciled; 137 of 61,349 corpus files refused.
  Check: `select count(*) from sensei.files where skip_reason='oversized'` > 0.
- #188 (d5bfdc7d): `GET /api/tasks/abandoned?days=N` and a Watchdog section in `sensei doctor`.
  Live: 204 abandonments in 14 days. Check: the released `sensei doctor` prints the section.

**Next:** #336. The worst abandonment is `ingest_capture` on one 1.28 GB active transcript (117×).
Measure first whether a pass re-reads the file from the start. Start by reading
`crate::transcript::run_ingest_capture`.

**Open (the user's):**
- dbd 0.27.7: the v0.27.6 lockfile pins rustls-webpki/webpki-roots/ctutils for
  `cargo install --locked` (fixed on dbd develop, 784db3d2). Needs a go.
- v0.20.1's `deploy-dojo` is still waiting; it would change nothing. Approve or reject.
- #331: count production's duplicate personal dōjōs (counts only):
  `psql "$PROD" -Atc "select count(*) from dojo.tenants where origin='personal' and key ~ '-[0-9]+$'"`.
- A release (patch) would let #178 and #188 close; it's the user's call when.

**Known:** DeepSource is red on kavach main; qlty is out of minutes. After a DDL change, run
`make test-template` (#142). `update_phase` returned 404 from the released daemon.
