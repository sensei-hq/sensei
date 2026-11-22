## 0. Harness unblocks (test-first)

- [x] 0.1 Test-mode harness flag pointing transcript adapters at fixture dirs
- [x] 0.2 Shared fs-fixture helper for watched-root specs (create/modify/delete + cleanup)
- [x] 0.3 SSE capture helper for e2e (subscribe, collect frames, assert shapes)

## 1. Uniform eventing (test-first)

- [x] 1.1 One envelope type + entity/action vocab, daemon + client
- [x] 1.2 Per-stream daemon-side allowlists (incl. scan-socket filter fix)
- [x] 1.3 Snapshot heads + `Last-Event-ID` replay
- [x] 1.4 Keep-alive/heartbeat + lag-drop surfacing (retry stays client-owned)
- [x] 1.5 `sseEvents` onFault wiring + fault rendering
- [x] 1.6 Validated scan parse; type-drift fixes (deferred/kind/id/stage/row)
- [x] 1.7 Live-verified on scratch daemon (envelope/filter/replay/heartbeat/terminal/snapshot); Tauri matrix in slice 9
- [x] 1.8 Dead `Indexed` emitter fixed (`build_connections` → `detect_communities`) with terminal `done` marking

## 2. Queue observability (test-first)

- [x] 2.1 List pending/running/blocked (kind/path/wait/blocked-by) + per-kind depth
- [x] 2.2 Failed-task list with reasons; fix per-repo completed/failed counts
- [x] 2.3 Backoff-pending visibility + nearing-budget signal + per-task abandonments
- [x] 2.4 Schedule skip reasons + pass progress exposure
- [x] 2.5 Live-verified on scratch daemon (listing/feeds/skips); Tauri matrix in slice 9

## 3. Pipeline progress (test-first)

- [x] 3.1 Totals events for transcript waves and metric waves
- [x] 3.2 Terminal `done` frames for remaining pipelines (index `Indexed` fix landed in 1.8); poller fallback deferred to slice 6 (no followers exist yet)
- [x] 3.3 Metrics parent linkage; transcript repair-count surfacing; embed-tail signal
- [x] 3.4 Stage guard + envelope mapping tested (production call sites belong to the pipeline cutover)
- [x] 3.5 Live-verified on scratch daemon (transcript + metric waves end to end); Tauri matrix in slice 9

## 4. Incremental transcripts (test-first)

- [x] 4.1 Watch transcript source dirs; narrowed `IngestCapture` on change
- [x] 4.2 Consent gating + debounce/coalesce across rapid appends
- [x] 4.3 Progress + totals + terminals on the uniform stream (job lifecycle per ingest; wave frames per backfill)
- [x] 4.4 Isolation: never watch real dirs on isolated instances
- [x] 4.5 Live-verified on scratch daemon (incremental/append/consent/isolation); Tauri matrix in slice 9
- [x] 4.6 Symlink-tolerant event→root resolution (FSEvents reports resolved paths)

## 5. Sync as jobs (test-first)

- [x] 5.1 Uniform sync events with wave ids (not queue tasks: plan-before-push ordering forbids fanout)
- [x] 5.2 Per-persona stage frames, wave totals + terminal, coded errors (plan-fetch attribution pre-existed)
- [x] 5.3 Live-verified on scratch daemon (empty wave shapes); Tauri matrix in slice 9

## 6. Domain stores (test-first)

- [x] 6.1 Jobs, schedules, sync stores per the `{source, state, selectors, ui, fault}` contract
- [x] 6.2 State specs drive `apply` with canned events, no DOM
- [x] 6.3 Singleton/page-local audit: fix nav-losing state (consolidation et al.)

## 7. Operations center surface

- [x] 7.1 Now / Queue / Failures / Schedules / Sync sections on the stores
- [x] 7.2 Drill-down (children/attempts), retry, schedule PATCH controls
- [x] 7.3 Rendered specs per section; hardcoded liveness claims wired or removed
- [ ] 7.4 E2E: full-cycle run live in the center (root-add → queue → progress → done) — folds into the slice 9 Tauri matrix (browser e2e is boot-gated)

## 8. Unified alerting (test-first)

- [x] 8.1 Severity mapping + reason codes for every failure class
- [x] 8.2 Dedupe store with success-clear; ack/silence/escalation
- [x] 8.3 Proactive notify beyond assistant-capture
- [x] 8.4 Daemon-down + provisioning banners
- [ ] 8.5 E2E: forced failures alert, aggregate, ack, and clear — folds into the slice 9 Tauri matrix (browser e2e is boot-gated)

## 9. E2E matrix completion (Tauri Playwright, live)

- [x] 9.1 Root-add initiates scan; indexing status updates live
- [x] 9.2 Parent-add consolidates; coverage continues under the parent
- [x] 9.3 File add/modify/delete detected, queued, processed, shown
- [x] 9.4 Transcript/metric/sync progress live end to end
- [x] 9.5 Job-error alert fires with reason and a way back
- [x] 9.6 Full unit + e2e suites green; fabrication/error-surfaces/phone gates clean
