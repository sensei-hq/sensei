# Finish the shared state seam: task source, realtime channel, dōjō host (#414)

## Why

#414 asks for one shared `@sensei/state` package with source adapters injected at construction, so the relay screen
runs off the same state on the desktop (daemon SSE) and the dōjō (Supabase realtime).

Most of it is on develop (#421, #484):

- `packages/state` exists with `ReactiveStageContext` (`packages/state/stage.svelte.ts:8`), `EventManager`
  (`packages/state/event-manager.ts:10`), the six relay entity states, the composite `RelayState`
  (`packages/state/relay/relay-state.svelte.ts:119`), the selectors, and canned-event specs.
- `RelaySource` (`relay-state.svelte.ts:52`) is the adapter interface. The SSE source is `relaySseSource`
  (`packages/state/relay/sources/sse.ts:18`); the realtime source is `relayRealtimeSource` over a
  `RelayRealtimeTransport` interface with a canned double (`packages/state/relay/sources/realtime.ts:28,44,79`).
- The app's relay pages build `RelayState` from `@sensei/state` over `relaySseSource`
  (`app/src/routes/(observatory)/relay/+page.svelte:13`), and `RelayTaskState` over `relayApi` plus `sseEvents`
  (`.../relay/[crew]/[task]/+page.svelte:13`; the workspace task page at line 14 is the same).
- Scan already uses the shared `ReactiveStageContext` (`app/src/lib/scan-state.svelte.ts:8`).

Three gaps stop the dōjō from using it, and the issue's migration record does not exist:

1. **The task view is tied to the daemon.** `RelayTaskState` reads and answers through `RelayApi`, the daemon HTTP
   client (`packages/state/relay/task-state.svelte.ts:29,53,70,122`), and hard-codes `surface: 'desktop'` on every
   answer (`task-state.svelte.ts:74`). The dōjō has no `RelayApi`.
2. **The realtime adapter is a seam only.** Nothing maps Supabase `postgres_changes` to a `StateEvent`; the dōjō's
   `relay-realtime.ts` only calls a coalesced refresh (`dojo/src/lib/relay-realtime.ts:142-182`).
3. **The dōjō cannot import the package.** `dojo/svelte.config.js` and `dojo/vitest.config.ts` alias
   `@sensei/glyphs`, `@sensei/sdlc-figures` and `@sensei/ui`, not `@sensei/state`.

The stream contract is also relay-named (`RelayEvents = RelaySource['events']`, `relay-state.svelte.ts:71`), so the
next domains (health, wizard) have no common shape to adopt.

## What Changes

- Add a domain-neutral stream contract at the package root: `StateStream<E>` (subscribe with an event handler and an
  open handler; returns the teardown). `RelayEvents` becomes `StateStream<RelayStreamEvent>`.
- Add `RelayTaskSource { readTask, answer, events }`. `RelayTaskState` takes a task source and a surface instead of
  `RelayApi` + events. The SSE side gets `relayTaskSseSource(api, stream)`; the realtime transport gains `readTask`.
- Add the realtime channel adapter: a pure `fromPostgresChange` mapper (`{eventType, new, old}` →
  `{action, entity, data}`, the daemon event's shape) and `realtimeChannel(client, spec)`, which gives a transport its
  `subscribe(on, onStatus)` from a structural Supabase client (no supabase-js dependency in the package).
- Alias `@sensei/state` in the dōjō (svelte.config.js, vitest.config.ts) and prove, in a dōjō spec, that `RelayState`
  and `RelayTaskState` built over `relayRealtimeSource` move on canned `postgres_changes` payloads.
- Record the migration order and the per-screen checklist in `packages/state/README.md` and on #414.

Not in this change (owned by epic #427): the dōjō relay route, the `relay.*` publisher, RLS, the realtime publication,
and the transport's reads and writes against the dōjō tables. #427 step 4 consumes the adapter built here.

## Impact

- Affected code: `packages/state/{index.ts, source.ts (new), README.md (new)}`,
  `packages/state/relay/{relay-state.svelte.ts, task-state.svelte.ts, sources/sse.ts, sources/realtime.ts,
  sources/postgres-change.ts (new)}` and their specs; the two app task pages; `dojo/svelte.config.js`,
  `dojo/vitest.config.ts`, a new dōjō spec.
- No daemon, DDL, or UI markup change. App behaviour is unchanged; the relay Tauri e2e suite is the regression proof.
- Issues: closes #414 (decision defaulted in design.md); unblocks #427 step 4.
