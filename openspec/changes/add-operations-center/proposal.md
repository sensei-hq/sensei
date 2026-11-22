# Change: Unified operations status (eventing, observability, center, alerting)

## Why

Every background job — scan/index, transcript ingest, metrics, sync, the 18
scheduled workers — reports progress and failure through a different channel
(or none): three SSE envelope shapes, four poll cadences, one-shot loaders,
counts-only queue status, and failure classes (`/api/tasks/abandoned`,
`/api/index/errors`, stalled/crashed runs) with zero UI callers. There is no
single location that answers "what is running, what is queued, what failed
and why", and nothing proactively alerts when a job misbehaves.

## What Changes

- One uniform SSE envelope + entity/action vocabulary across scan, relay,
  tasks, schedules, sync, metrics, transcript, and watcher streams, with
  per-stream daemon-side allowlists, snapshot heads, `Last-Event-ID` replay,
  keep-alive/heartbeat, and terminal `done` frames.
- Queue observability: list pending/running/blocked with kind+path+wait+
  blocked-by, per-kind depth, failed-task list, backoff visibility, per-task
  abandonments + nearing-budget signal, schedule skip reasons + pass progress.
- Pipeline progress protocol: `FolderQueued`-style totals for every pipeline,
  terminal `done` events (incl. the dead `Indexed` emitter fix), metrics
  parent linkage, transcript repair-count surfacing, embed-tail signal.
- Incremental transcript ingest on file changes (new): transcript source dirs
  watched, per-file `IngestCapture` on change, consent-gated, debounced —
  in addition to the boot/tick backfill.
- Sync passes as first-class jobs with task ids and progress events.
- Per-domain singleton stores (`{source, state, selectors, ui, fault}`) after
  the relay/scan exemplars; one Operations center surface (Now/Queue/
  Failures/Schedules/Sync) with drill-down, retry, and ack/silence.
- Unified alerting: cross-class severity mapping, dedupe store, ack +
  escalation, proactive notify beyond assistant-capture, daemon-down and
  provisioning banners — all through Notice/Banner/AlertList.
- E2E matrix over Tauri Playwright: root-add, parent-add, file add/modify/
  delete, transcript/metric/sync progress, job-error alerting.

## Impact

- Affected specs: `job-eventing`, `queue-observability`, `pipeline-progress`,
  `transcript-incremental`, `sync-jobs`, `ops-center`, `unified-alerting`
  (all new capabilities).
- Affected code: `crates/senseid` task queue + schedulers + SSE handlers +
  transcript/sync pipelines; `packages/state` (new domain stores);
  `app` (Operations center + alert wiring); `app/e2e` (new specs + watcher
  capture + transcript test-mode harness flag).
- No DDL breaking changes expected; new list endpoints are additive. The
  envelope unification changes SSE frame shapes (consumers migrate together).
