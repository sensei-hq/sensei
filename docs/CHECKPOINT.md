# Checkpoint — 2026-10-26

**Slice:** issue burndown (docs/roadmap/08-issue-burndown.md), wave 5. The user wants a **demo this week**
(sensei + dōjō, metrics and diagrams); a demo plan has been proposed and the order awaits the user's pick.

**Released:** v0.20.2 (deploy-dojo done; the Worker serves 0.20.2) and dbd 0.27.7 (sensei pins it).

**Closed today:** #289 (153,430 duplicate events and the 40,148 verdicts of their 26 sessions deleted after
a backup in `database/backup/essential/20261026-121744`; sessions re-classified, 0 duplicates), #194 (one
`task_failed` row per task; 2,409 old duplicates deleted), #331, #338. Filed #339: every execution writes
`task_started` and `task_completed` log rows (31.7M in a month).

**Waiting on the next release:** #336 (a week with no `ingest_capture` abandonment), #125 (5dea36e3; 22 → 18
orphans), #161 (all acceptance met), #338's cadence, the VS Code installed scan.

**In progress — #123:** fixed: A5, A6, items 1–5 and 7, adjacent (a), (b), (c) (fad9ef49). Left: (6)
`event_count`'s three meanings, (d) `#L10` fragments, then Track B.

**Found for the demo (not yet fixed):**
- The sessions "last 7 days" filter keys on `started_at` (`pg_store/sessions.rs:48`), so a long session that
  is still running (this one, started 10-15) drops out of the view.
- The MCP graph tools (`get_communities`, `get_callers`) return 404 on `/api/projects` against 0.20.2.
- The dōjō has no graph view, and the developer console has no metric charts.
- `repository_graph` sync is skipped, and the `sensei` workspace has no `dojo_id`.

**Next:** fix the two defects above (red-first), then the demo slice the user picks; then #190 (hours, through
dbd-core's reader).

**Watch roots:** the live DB watches only sensei until the indexer covers JS/TS/Java/.NET. dbd and gateway
are Rust, so they could join now; that is the user's decision.

**Known:** the dōjō Workers build is intermittent on PR/develop commits. qlty is out of minutes. After a DDL
change, run `make test-template`.
