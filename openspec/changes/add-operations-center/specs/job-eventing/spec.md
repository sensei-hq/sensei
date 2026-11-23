## ADDED Requirements

### Requirement: Uniform SSE envelope

Every daemon SSE stream SHALL emit one envelope with a shared client type:
`{v, id, ts, entity, action, key, data, done?, error?}`.

#### Scenario: One client type parses every stream

- **WHEN** a client subscribes to the scan, relay, task, or schedule stream
- **THEN** every frame validates against the same envelope (version, cursor
  id, timestamp, entity, action, stable key, typed payload, optional
  terminal/error markers)

#### Scenario: Task lifecycle maps onto add/update/done

- **WHEN** a task is queued, starts, completes, or fails
- **THEN** the stream carries add, update, update with `done:true`, or
  update with `done:true` and a coded error — never a second shape

### Requirement: Entity vocabulary

The envelope SHALL use one entity vocabulary: today's scan and relay sets
extended with `job`, `schedule`, `sync`, `metric`, `transcript`, `index`,
`library`, and `watcher`.

#### Scenario: New domains need no new envelope

- **WHEN** a schedule tick, sync entity, or watcher event emits
- **THEN** it arrives as the uniform envelope with its entity name and a
  documented key

### Requirement: Per-stream daemon-side allowlists

Each stream SHALL forward only its own entities, enforced daemon-side.

#### Scenario: Scan socket stops carrying relay traffic

- **WHEN** a client subscribes to the scan stream
- **THEN** it receives workspace, folder, activity, and assistant frames
  only — relay-family frames stay on the relay stream

### Requirement: Snapshot head plus replay

Each stream SHALL open with a snapshot head and support `Last-Event-ID`
replay from durable rows plus a queue snapshot.

#### Scenario: Reconnect resumes without a blind full re-read

- **WHEN** a client reconnects with its last cursor id
- **THEN** the daemon replays missed frames from durable state instead of
  forcing the client to re-read everything

### Requirement: Heartbeat, keep-alive, and lag surfacing

Every stream SHALL send keep-alive/heartbeat frames and surface lagged
drops as stream-error frames instead of dropping them silently.

#### Scenario: A lagging client learns it lagged

- **WHEN** the broadcast outruns a subscriber
- **THEN** the client receives an explicit stream-error frame naming the
  gap rather than a silent hole

### Requirement: SSE faults reach state

SSE faults SHALL flow through `onFault` into the stores' fault slots and
render like realtime faults do.

#### Scenario: A dead scan socket reads as a fault

- **WHEN** the scan stream errors
- **THEN** the scan state holds the fault and the progress surfaces render
  it, instead of freezing on stale counts
