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

**Demo prep (user: demo this week, sensei + dōjō, metrics and diagrams):**
- FIXED (this tick, see commit): "last 7 days" and a workspace's `sessions7d`/`last_session_at` keyed on a
  session's START, so a long running session vanished from this week. Now keyed on last activity. FTR keeps
  bucketing finished sessions by start; changing that is a metric decision, not this bug.
- NOT a bug: the MCP 404 on `/api/projects` is this session's MCP process, running since 10-15 (pre-R3b
  binary). The installed 0.20.2 MCP answers `get_callers`/`get_communities` correctly. A demo needs a fresh
  Claude session. Product gap: an upgrade leaves running sessions on a stale MCP.
- TODO: community labels read "function ()" / "unknown ()" (kind + shared dir); name them by their central
  module or most-connected function.
- TODO: the dōjō developer console has no metric charts; no dōjō graph view (out of scope this week).
- Watch roots: dbd, gateway, torii added 2026-10-26 (user). Scans running. TS/JS/Svelte call resolution
  57.7% (44.3% by the compiler); the gap is `receiver_type_unknown` (9,942 of 37,060).

**Next:** commit the sessions-window fix after the full senseid suite, verify on the live daemon, then the
community labels, then the dōjō metric chart; then #190 (hours: manifest adapter → `project::survey`,
workspace designs → `Workspace::load`).

**Known:** the dōjō Workers build is intermittent on PR/develop commits. qlty is out of minutes. After a DDL
change, run `make test-template`.
