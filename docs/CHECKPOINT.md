# Checkpoint — 2026-10-29 (tracker SDLC metrics)

**Slice:** tracker SDLC metrics, on branch `feature/sdlc_metrics`. The branch was cut from
`feature/tracker_metrics_writeback`, which is still unmerged.
**Stage:** build, unattended (user, 2026-10-29). F1 done (`886b5df1c`), F2 + F3 done
(`87c7d8316`); F4 (ticket figures) next. Run contract: `.sensei/slice-state.json`
(amendments A1–A4; F4 carry-forward).

**For the user:** the indexer's dangling-reference ceiling was raised 1,300 → 1,302 for
the new settings handler's two `state.pg` reads (AppState type alias); and only 9 of 118
hook-event sessions have a session row on this machine (backlog P0), so ticket metrics
will be sparse here until P0 lands.
- Analysis: `docs/analysis/2026-10-29-sdlc-metrics-extension.md` (decisions D1–D11)
- Spec (reviewed): `docs/spec/daemon/tracker-sdlc-metrics.md` (S1–S9, gates 1–13, claims
  C1–C39)
- Plan: `docs/plans/2026-10-29-tracker-sdlc-metrics.md` (F1–F9). Issues are TBD because
  `gh` isn't installed.

**User decisions (2026-10-29):**
- Show the metrics in screens and in write-back.
- Backfill history from Jira.
- Use a project-level `hourly_rate`.
- Keep the D5 names.
- Story points are the estimate.
- Review defects are escaped task defects plus review fixes.
- Support every framework, with the catalogue held as data.
- Defer the write-back field set.
- A finding counts as one subsequent fix.
- Escaped defects count across sessions.

**Claims that verification found false, and what changed:**
- Event time is `ts`, not `created_at` (C20). Every signal is now dated by `ts`.
- `update_task_status` takes `state`, not `status` (C17).
- There is no `/tickets` route; ticket reads are `/tasks` and `/tracker-tasks/{key}` (C21).
  There is no `GET …/trackers/{t}` either; `sdlc_project` goes on the `GET …/trackers` list (C38).
- A `Write` update carries `originalFile`, so a full rewrite of `tasks.md` now counts as a tick (C22).
- NotebookEdit's path is `notebook_path` (C23).
- The usage-only doc surfaces were under-listed (C39). The list is now gate 12's output:
  53 lines in 24 files.

**Claims that were misleading, and what changed:**
- A lane is a board, not a sprint (C19). The sprint row now goes in the lane header when the
  lane's filter selects one sprint.
- A sync timeout aborts the whole sync (C24), so ticket history moved to its own task. It
  runs inside the write-back pass's lock, which is a declared override of R5 and W2.
- `metric_value` takes `&Usage` only (C25), so it now takes `TicketFigures`, a declared
  override of W3.

**What the review changed:**
- **Outcomes come from category-scoped output markers before the exit code.** 318 of 370
  real build/test runs were piped, and 4 real rows would have logged phantom compile defects.
- **Lost failures count as "unknown".** A call with no result event becomes
  `unmatched_call` and is never read as 0.
- **Write-back now covers SDLC metrics,** but rate- and calibration-derived figures are
  never written.
- **The settings routes use `local_caller_guard`.**
- **Data-correctness fixes:**
  - `submitted_at` comes after the first code edit
  - `lead_hours` and holds are never negative
  - trigger fails are limited to their category
  - an un-derived session is `not_derived`
  - shared sessions count once
  - a points sum over nothing is null

**Open (human):** update the installed sensei plugin (0.11.0 lacks `PostToolUseFailure`),
and start the daemon for live gate 9.

**Next command:** `/sensei:build` on `docs/plans/2026-10-29-tracker-sdlc-metrics.md`,
starting with F1 (command catalogue + classifier, gate 1). Re-run the claims ledger first.
