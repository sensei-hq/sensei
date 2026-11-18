## ADDED Requirements

### Requirement: One stream contract for every state source

`@sensei/state` SHALL export a domain-neutral `StateStream<E>`: subscribe with an event handler and an open handler,
and get a teardown back. The SSE and realtime adapters SHALL produce it for any event type, and every open (the first
and each reconnect) SHALL call the open handler.

#### Scenario: A non-relay event type uses the same stream
- **WHEN** a spec builds `sseEvents` over a fake socket typed for a test event `{id, n}` and fires two events and two opens
- **THEN** the handler receives both events, the open handler runs twice, and the teardown unsubscribes and destroys the socket

#### Scenario: A realtime SUBSCRIBED counts as an open
- **WHEN** a realtime transport reports `SUBSCRIBED`, then `CHANNEL_ERROR`, then `SUBSCRIBED`
- **THEN** the open handler runs exactly twice

### Requirement: The task view reads and answers through a source

`RelayTaskState` SHALL be built from a `RelayTaskSource` (`readTask`, `answer`, `events`) and an optional surface, and
SHALL NOT depend on the daemon HTTP client. An answer SHALL carry the surface the state was built with (`desktop` by
default). The app's task pages SHALL build it with the SSE task source.

#### Scenario: The dōjō surface answers as the dōjō
- **WHEN** a `RelayTaskState` built with `{surface: 'dojo'}` answers an open decision
- **THEN** the source receives the answer with `surface: 'dojo'` and the state re-reads the task through `readTask`

#### Scenario: The desktop task page still moves live
- **WHEN** the Tauri e2e `app/e2e/tests/relay-run.spec.ts` drives a fake crew through a task's checkpoints
- **THEN** the task page shows each new step and state change with no reload, and `relay-timeline.spec.ts` stays green

### Requirement: Supabase changes map to the daemon's event shape

The package SHALL map a `postgres_changes` payload on a relay table to a `StateEvent` the states apply as they apply
the daemon's: INSERT → add, UPDATE → update, DELETE → remove. A table row SHALL map to the id key the daemon names
that entity by, with none of the table's columns, so the state re-reads it. A table declared screen-shaped SHALL carry
its row (from `new`, or `old` on DELETE) plus that id key. A payload on an unmapped table SHALL be reported, not
dropped silently.

#### Scenario: A relay table row re-reads what it names
- **WHEN** a spec maps a row with the columns of `relay.items`, `relay.runs` or `relay.lanes` and applies it to `RelayState`
- **THEN** the state re-reads the queue (item) or Now (run, lane), the item shows in the backlog, the run keeps its task counts, and no table column lands on the lane

#### Scenario: A screen-shaped table and the daemon leave the same state
- **WHEN** a spec applies a lane, run, item, decision and message to one `RelayState` through SSE fixture events and to another through `postgres_changes` payloads on tables declared screen-shaped
- **THEN** both states hold the same rows and the selectors return the same views

#### Scenario: A task change reaches the task view
- **WHEN** an UPDATE on the `tasks` table for the viewed task is mapped and applied to `RelayTaskState`
- **THEN** the event carries `task_id` and the state re-reads that task

### Requirement: A realtime channel adapter over a structural client

The package SHALL provide `realtimeChannel(client, spec)` that opens one channel per subscription, under its own
topic, with a `postgres_changes` listener per relay table, forwards mapped events and channel statuses, and removes
the channel on teardown. A channel that fails, times out or closes SHALL be logged with the supabase-js error and
reported to the state as a fault. It SHALL accept any client with the supabase-js `channel` / `on` / `subscribe` /
`removeChannel` shape and SHALL add no package dependency.

#### Scenario: Two subscribers share a client and a crew
- **WHEN** two subscribers on one real supabase-js client subscribe to the same crew topic
- **THEN** both open, each on its own channel, and neither throws

#### Scenario: A dropped channel is a fault
- **WHEN** the channel reports `CHANNEL_ERROR` with an error
- **THEN** it is logged with the topic and the error, the state's `streamFault` holds the reason, and the next `SUBSCRIBED` clears it

#### Scenario: Teardown removes the channel
- **WHEN** a subscriber built on `realtimeChannel` with a fake client is torn down
- **THEN** the fake client's `removeChannel` is called once with the channel it opened, and later payloads reach no handler

### Requirement: The dōjō hosts the shared relay state

The dōjō SHALL import `@sensei/state` (aliased like `@sensei/ui`) and SHALL build `RelayState` and `RelayTaskState`
over the realtime source in a spec, with no DOM and no daemon.

#### Scenario: The dōjō state moves on canned realtime payloads
- **WHEN** `dojo/src/lib/relay/shared-state.spec.svelte.ts` emits canned `postgres_changes` payloads through a fake supabase client
- **THEN** on screen-shaped tables the lanes and decisions update without a re-read, a row from today's relay tables re-reads, the task view re-reads its own task, a `SUBSCRIBED` status triggers one re-read, and `bun run build` in `dojo/` succeeds with the alias

### Requirement: The migration order and checklist are recorded

`packages/state/README.md` SHALL record the order in which domains move into the package and a per-screen checklist,
and the same text SHALL be posted on #414.

#### Scenario: A reader finds the next domain to move
- **WHEN** a contributor opens `packages/state/README.md`
- **THEN** it lists relay as done, then scan, health, wizard, and dōjō domain states in order, with the six-item checklist each screen must meet
