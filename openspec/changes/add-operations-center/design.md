## Context

Deep-dig report (6 researchers + synthesis, all claims file:line-backed):
scan/index and relay have SSE + singleton state; ~30 subsystems emit
nothing; envelopes differ per stream; queue status is counts-only; failure
classes lack UI; alerting is one toast trigger. The scan footer
(`add-scan-progress-footer`, complete) is the first consumer of the pattern
this change generalizes.

## Goals / Non-Goals

- Goals: one envelope; counts → listings; totals + terminal frames per
  pipeline; change-driven transcript ingest; one center; one alerting path;
  e2e proof per phase.
- Non-Goals: changing scheduling cadences or retry budgets; dojo-side
  rendering (daemon+app+shared state only); promoting components to
  `@sensei/ui` (follow-up per component).

## Decisions

### D1. Envelope (replaces 3 shapes)

```json
{ "v": 1, "id": "<seq|uuid>", "ts": "<rfc3339>",
  "entity": "<vocab>", "action": "add|update|remove|set",
  "key": "<stable id>", "data": { },
  "done": false, "error": { "code": "<kebab>", "message": "<reason>" } }
```

- `entity`: today's scan set + relay family, extended with
  `job|schedule|sync|metric|transcript|index|library|watcher`.
- TaskEvent maps: queued→add, started/stage_started→update,
  completed/stage_completed→update+`done:true`,
  failed/stage_failed→update+`done:true`+`error`,
  folder_queued→add with totals.
- `key`: taskId (boot-stable) + execution row id for attempts; folder path;
  schedule name; sync entity; relay ids. `id` is the stream cursor for
  `Last-Event-ID` replay (served from `task_executions`/sync/schedule rows +
  queue snapshot), replacing blind full re-reads.
- `done`/`error` optional, omitted when unset. Streams keep their routes;
  frames converge on this shape. Consumers migrate in the same slices (no
  version negotiation: daemon and app ship together).

### D2. Stream reliability

- Every stream sends a snapshot head first (the `/api/tasks/{id}/events`
  pattern), then live frames; per-stream entity allowlists enforced
  daemon-side (fixes the unfiltered scan socket).
- `keep_alive` + `retry:` + heartbeat frames on all handlers; lagged-receiver
  drops surface as `{entity:stream, error}` instead of silent drops.
- `sseEvents` wires `onFault` through to `streamFault`/`daemonReachable`, so
  SSE faults render like realtime faults do today.

### D3. Terminal truth moves onto the stream

- Fix the dead `Indexed` arm (`build_connections` → `detect_communities`);
  scans complete on `done:true` frames, and the scan poller drops to a slow
  liveness fallback. Same rule for every pipeline: no poller-decided `done`.

### D4. Store contract `{source, state, selectors, ui, fault}`

- Source injected at construction; validated parsing everywhere (relay's
  `parseRelayStreamEvent` style replaces scan's raw `JSON.parse`).
- Apply: snapshot hydrate on open, in-place row apply for full rows,
  targeted re-read for id-only events; passive `attach()` vs owning
  `start()` lifecycle from scan.
- Selectors pure + unit-specs; `ui` holds progress `{total,done,current}`
  and view state; `fault` is a rendered `streamFault|null` + reachable bool
  on every store.
- Ownership: singletons for cross-screen domains, page-local only for
  single-visit runs (fixes consolidation's nav-losing state).

### D5. Operations center (one surface, observatory section)

Sections: Now (running + progress + current file/stage), Queue (pending/
running/blocked lists + per-kind depth + backoff), Failures (aggregated
feed across task/index/abandonment/crash/schedule/sync/stream faults),
Schedules (rows + log tails + PATCH controls), Sync/Metrics/Transcript
(state rows + triggers). Drill-down to children/attempts; retry per failed
job/schedule/pass; ack/silence/escalation per alert.

### D6. Alerting

- Severity: `reason_kind` (normal/refusal/fault) + `action` tone ×
  `NotifyLevel` (Info/Warn/Critical); every failure class gets reason codes;
  scheduled domain joined per reason-codes.
- Dedupe by `(class,key,retry_number/window)` with success-clear (the
  schedules pattern); stream/reachability faults clear on open/poll.
- Proactive notify extends beyond assistant-capture (terminal fail, retry
  exhaustion, run crash, failing schedule, degraded/down) via the existing
  notify channel; feed reads fail closed (500) per D83.
- Rendering: Notice tones + Banner strips + AlertList toasts only, under the
  error-surfaces ratchet. Daemon-down and provisioning banners included.

### D7. Incremental transcripts

- Transcript source dirs (per adapter) registered with the watcher alongside
  watch roots; file add/modify/delete enqueues a narrowed `IngestCapture`
  for that file (not a full backfill), consent-gated, debounced/coalesced
  across rapid appends; progress + totals on the uniform stream.
- Boot/tick backfill stays as the safety net (watermark cursors unchanged).
- Isolation rule extends: isolated instances (`SENSEI_INSTANCE=e2e`) never
  watch real `~/.claude`-style dirs. E2E gets a test-mode harness flag that
  points adapters at fixture dirs so transcript progress scenarios can run.

### D8. Sync as jobs

- Sync passes become queue tasks (or emit uniform sync events with task ids
  if queue semantics don't fit): per-persona/per-entity progress, terminal
  states, errors with codes — visible in Now/Queue/Failures like the rest.

### D9. Rollout order (each slice shippable + e2e'd)

1. Envelope + scan filter + terminal fix + scan parse validation (unblocks all
   consumers). 2. Queue observability endpoints. 3. Pipeline totals/terminals
   + metrics linkage. 4. Transcript incremental + harness flag. 5. Sync jobs.
   6. Domain stores. 7. Center surface. 8. Alerting (severity/dedupe/ack/
   proactive). 9. E2E matrix completion.

## Risks / Trade-offs

- Envelope migration touches every SSE consumer at once — mitigated by
  phased slices that migrate one stream family at a time behind the new
  client type, with the old shapes removed in the same slice (no dual-write
  period; daemon+app ship together).
- Watcher load from transcript dirs: adapters enumerate bounded dirs;
  debounce + per-file cursors keep event cost proportional to real changes.
- Queue listings on a hot queue: paginated + snapshot reads, never a live
  cursor into executor internals.

## Migration Plan

Additive endpoints first; stream-shape changes migrate daemon+app together
per slice; no DDL breaks (new columns nullable or new tables). Rollback per
slice by revert.

## Open Questions

- None blocking. Sync-as-queue-tasks vs sync-events decided in slice 5 by
  fit (spec covers both via the progress/terminal requirements, not the
  mechanism).
