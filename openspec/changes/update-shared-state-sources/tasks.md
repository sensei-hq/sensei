## Slice 1 — the task view on a source (app, SSE)

Red commit first, then green, per item. Run `cd app && bun run test:unit` after each green: the package has no vitest
config of its own, and `app/vitest.config.ts` includes `../packages/state/**/*.spec{,.svelte}.ts`.

- [x] 1.1 Red: `packages/state/source.spec.ts` — `sseEvents` and `realtimeEvents` typed as `StateStream<E>` for a
      non-relay event type (a `{id, n}` test event): every event reaches the handler, every open calls `onOpen`, the
      teardown unsubscribes and destroys the socket.
- [x] 1.2 Green: add `packages/state/source.ts` (`StateStream<E>`); make `sseEvents`/`realtimeEvents` generic;
      `RelayEvents = StateStream<RelayStreamEvent>`; export `StateStream` from `index.ts`.
- [x] 1.3 Red: `relay/task-state.spec.svelte.ts` — build `RelayTaskState` from a fake `RelayTaskSource`: `retry`
      and `reRead` call `readTask(crew, task)`; `answer` calls `source.answer` with `surface: 'dojo'` when built with
      `{surface: 'dojo'}` and `'desktop'` by default; `connect` subscribes through `source.events`.
- [x] 1.4 Green: `RelayTaskSource` type; `RelayTaskState(reads, source, crewId, taskId, options?)`; drop the
      `RelayApi` field. Add `relayTaskSseSource(api, stream)` in `sources/sse.ts` with a spec in `sources.spec.ts`.
- [x] 1.5 Wire both app task pages (`app/src/routes/(observatory)/relay/[crew]/[task]/+page.svelte`,
      `app/src/routes/(workspace)/workspace/[id]/relay/[crew]/[task]/+page.svelte`) to
      `relayTaskSseSource(relayApi(port), () => relayStream(port, crewId))`; update their page specs (red first if a
      spec asserts the old constructor).
- [x] 1.6 Red then green: `event-manager.spec.ts` — a payload that fails to parse is reported to an `onError`
      handler (default logs with `console.warn` naming the URL) and the next payload still dispatches.
- [x] 1.7 Checks: `cd app && bun run check && bun run test:unit`; `rg -n --no-ignore -g '!node_modules' 'tryGetTask'
      packages app/src` shows only the client, the SSE task source, the two task-page loaders (`+page.ts`, which
      read the first view and stay as they are), fixtures and specs.
- [x] 1.8 Tauri e2e (follow the shared CONTEXT preflight: `pgrep -f "instance e2e"` empty first): `make test-app-e2e`,
      or after the bundle is built (`make app-e2e-build`) `cd app && bunx playwright test --config
      e2e/playwright.config.ts --project=tauri relay-run relay-timeline relay.spec`. The run journey's task-page steps land with no
      reload. Confirm `brew services start sensei` restored the real daemon afterwards.

## Slice 2 — the realtime channel adapter and the dōjō host

- [x] 2.1 Red: `relay/sources/postgres-change.spec.ts` — `fromPostgresChange` maps INSERT→add, UPDATE→update,
      DELETE→remove (data from `old`); a task row carries `task_id`, a lane `lane_id`, a run `run_id`; an unmapped
      table returns `null`. One case per entity compares the mapped event with the SSE fixture's event for the same row
      after `RelayState.apply` (same lanes/runs/items/decisions/messages state either way).
- [x] 2.2 Green: `relay/sources/postgres-change.ts` with `RELAY_TABLES` (table → entity).
- [x] 2.3 Red: `realtimeChannel` spec with a fake structural client — opens one channel per subscribe with one
      `postgres_changes` listener per table, forwards mapped events, reports each status, logs and skips an unmapped
      payload, and the teardown calls `removeChannel`.
- [x] 2.4 Green: `realtimeChannel(client, spec)` returning `{subscribe(on, onStatus)}`; `RelayRealtimeTransport`
      gains `readTask`; `relayRealtimeTaskSource(transport)`; `cannedRealtime` gains a `task` read. Export from
      `index.ts`.
- [x] 2.5 Dōjō alias: add `'@sensei/state': '../packages/state/index.ts'` to `dojo/svelte.config.js`, and
      `'@sensei/state': new URL('../packages/state/index.ts', import.meta.url).pathname` to `dojo/vitest.config.ts`
      (each file's existing `@sensei/ui` form and comment style).
- [x] 2.6 Red then green: `dojo/src/lib/relay/shared-state.spec.svelte.ts` — `RelayState` over
      `relayRealtimeSource` and `RelayTaskState` over `relayRealtimeTaskSource`, both on a transport whose `subscribe`
      is `realtimeChannel` over a fake supabase client. Emitting canned `postgres_changes` payloads moves the lanes,
      decisions and the task view with no re-read; a `SUBSCRIBED` status re-reads; an answer goes out with
      `surface: 'dojo'`.
- [x] 2.7 Checks: `cd dojo && bun run test && bun run check && bun run build`; `cd app && bun run test:unit`.
- [x] 2.8 Tauri e2e regression (package changed): `cd app && bunx playwright test --config e2e/playwright.config.ts
      --project=tauri relay-run relay.spec` green, with the same preflight and daemon restore as 1.8.
- [x] 2.9 Write `packages/state/README.md`: what the package holds, the source contract, the migration order and the
      per-screen checklist (design D5).
- [x] 2.10 `openspec validate update-shared-state-sources --strict`.
- [x] 2.11 On merge to develop: comment the order + checklist on #414; file one issue each for scan, health and
      wizard moving to the package (checklist in the body); comment on #427 that step 4 uses `relayRealtimeSource`,
      `relayRealtimeTaskSource` and `realtimeChannel`; close #414 with `Closes #414` in the final commit and a comment
      linking #427 step 4 for the dōjō screen half.
