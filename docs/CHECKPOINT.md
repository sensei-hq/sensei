# Checkpoint — 2026-11-18 (relay open items — #414 slice 2, review findings fixed)

**Slice:** `update-shared-state-sources` slice 2 (#414): the realtime channel adapter and the dōjō
host. Branch `lane/relay-414-s2` (worktree `lanes/relay-open`), PR #499 to develop.

**Done (tasks 2.1–2.11):** see `openspec/changes/update-shared-state-sources/tasks.md`.

**Done (PR #499 review, red then green each):**
- Shared topic: `realtimeChannel` opens each subscription as `<topic>:<n>`. supabase-js reuses a
  topic's channel and throws on a listener added after `subscribe()`; the dōjō spec proves it with a
  real `createClient()`.
- Silent channel errors: `CHANNEL_ERROR` / `TIMED_OUT` / `CLOSED` are logged with the supabase-js
  error; `StateStream` gains optional `onFault`; `RelayState` and `RelayTaskState` hold
  `streamFault` until the next `SUBSCRIBED`.
- Table rows vs screen rows: `fromPostgresChange` maps a table row (default `RELAY_TABLES`) to
  `{op, <entity>_id}` and the state re-reads; `{entity, screenRow: true}` carries the row. Specs use
  rows with the DDL's columns; parity specs run only on a screen-shaped map.
- Checks: app unit 3786/3786, app check 0 errors, dōjō test 2409/2409, check and build clean,
  `openspec validate --strict` valid. Tauri e2e: diagrams-schema + relay-run + relay-timeline +
  relay.spec 6/6 green; real brew daemon restored on :7744.

**Remaining:** merge PR #499 (closes #414). No screen shows `streamFault` yet: the desktop is on SSE
(no faults), and the dōjō relay page is #427 step 4.

**Next command:** `gh pr checks 499` — merge #499 into develop when green.

**Open questions:** none blocking. #427's publisher decides whether its tables are screen-shaped
(`screenRow: true`) or table rows (re-read).

**Known broken:** none from this branch. Running relay e2e specs alone after `make reset-e2e-db`
needs a workspace first: include `diagrams-schema` in the run (it seeds one).
