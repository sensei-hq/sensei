# Checkpoint — 2026-11-18 (relay open items — #414 slice 1 built)

**Slice:** `update-shared-state-sources` slice 1 (#414): the task view on a source + the shared
`StateStream` contract. Branch `lane/relay-414-s1` (worktree `lanes/relay-open`), PR to develop.

**Done (tasks 1.1–1.8):**
- `packages/state/source.ts`: `StateStream<E>`; `sseEvents` / `realtimeEvents` are generic;
  `RelayEvents = StateStream<RelayStreamEvent>`.
- `RelayTaskState(reads, source, crewId, taskId, { surface })` reads and answers through a
  `RelayTaskSource`; `relayTaskSseSource(api, stream)` in `sources/sse.ts`. Both app task pages use it.
- `EventManager` reports a payload that fails to parse through `onError` (default `console.warn`
  naming the URL) and keeps dispatching; a throwing handler is no longer swallowed.
- Tests: `source.spec.ts`, `task-state.spec.svelte.ts`, `sources.spec.ts`, `event-manager.spec.ts`
  (each red then green). `relay-run.spec.ts` task-page step now asserts no full reload.

**Remaining:** slice 2 (tasks 2.1–2.11): `fromPostgresChange`, `realtimeChannel`,
`relayRealtimeTaskSource`, the dōjō alias and shared-state spec, the package README, validate, close.

**Next command:** review the slice 1 PR, merge into develop, then start slice 2 on
`lane/relay-414-s2` from develop.

**Open questions:** none blocking.

**Known broken:** none from this branch. `make install-debug` cleans `app/src-tauri/target`, so run
`make app-e2e-build` (not install-debug alone) before a single e2e spec.
