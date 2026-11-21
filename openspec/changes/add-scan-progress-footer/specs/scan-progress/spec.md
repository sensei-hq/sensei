## ADDED Requirements

### Requirement: Passive scan observation

The app SHALL observe daemon indexing activity without triggering scans,
through the shared scan state.

#### Scenario: Footer attaches while the daemon is busy

- **WHEN** the footer mounts and the task queue is non-idle
- **THEN** the shared state reports `scanning` and no scan is triggered

#### Scenario: Footer attaches while the daemon is idle

- **WHEN** the footer mounts and the task queue is idle
- **THEN** the shared state stays `idle` (never `done`)

#### Scenario: Passive observation never completes a run

- **WHEN** the queue drains while only passively observed
- **THEN** the shared state returns to `idle` and the wizard scan gate stays closed

#### Scenario: Setup still owns its run

- **WHEN** the setup scan page starts a scan
- **THEN** unscanned roots are triggered and the run can reach `done` as today

### Requirement: Shared ScanProgress component

The app SHALL render indexing progress through one shared `ScanProgress`
component with `mini`, `band`, and `full` variants, every variant reading
the same shared state.

#### Scenario: Variants agree

- **WHEN** two variants render simultaneously
- **THEN** every count shown in both is identical (one state, no second read)

#### Scenario: Static variant per mount

- **WHEN** a screen mounts a variant
- **THEN** the variant prop is fixed for that mount; runtime switching happens only through shared visibility state, never the prop

### Requirement: Mini variant in sidebar footers

Each section sidebar SHALL show the `mini` variant in its footer while
indexing is active.

#### Scenario: Mini appears with live counts

- **WHEN** the shared state becomes `scanning`
- **THEN** each sidebar footer shows completed files and queued tasks with progress

#### Scenario: Mini hides while idle

- **WHEN** the shared state is `idle`
- **THEN** the mini variant renders nothing (footers keep their current content)

#### Scenario: Mini expands to the band

- **WHEN** the user activates the mini expand control
- **THEN** the band becomes visible (shared `expanded` state)

### Requirement: Band variant in the root layout

The root layout SHALL show the `band` variant — a full-width, small-height
bar with stats and latest activity — when expanded while indexing is active.

#### Scenario: Band shows on expand

- **WHEN** the user expands from mini while the state is `scanning`
- **THEN** a full-width bar shows workspace stats and the latest activity

#### Scenario: Band collapses

- **WHEN** the user dismisses the band
- **THEN** it hides and mini remains the visible progress

#### Scenario: Band hides when the run ends

- **WHEN** the shared state leaves `scanning` with no error
- **THEN** the band disappears regardless of `expanded`

#### Scenario: Band links to the full view

- **WHEN** the band is visible
- **THEN** it offers the setup scan route as the full view, with the `?force=1`
  setup re-entry once setup is complete

### Requirement: Full variant reuse

The setup scan page SHALL render the shared component's `full` variant with
no visual or behavioural change.

#### Scenario: Setup scan looks and behaves as today

- **WHEN** the user runs the setup scan
- **THEN** stats bar, workspace cards, tasks panel, summary card, retry and
  re-scan behave exactly as before (same testids)

### Requirement: Unreachable daemon

The footer SHALL say when the daemon stops answering mid-run instead of
freezing on stale counts.

#### Scenario: Daemon unreachable mid-run

- **WHEN** the queue poll fails while progress is visible
- **THEN** mini and band show the daemon is unreachable and retrying, not stale counts

### Requirement: Scan state in @sensei/state

Scan state SHALL live in `@sensei/state` per the package checklist (closes
#496): entities as reactive contexts, source injected at construction, the
stream as a `StateStream` that re-reads on every open, pure selectors with
unit specs.

#### Scenario: State specs drive events without DOM

- **WHEN** the scan specs run
- **THEN** they construct the state with a canned source, drive `apply`
  with canned events, and assert on data

#### Scenario: One shared instance in the app

- **WHEN** the footer, a sidebar, and the setup page render together
- **THEN** all three read one module-level instance built over the daemon SSE source
