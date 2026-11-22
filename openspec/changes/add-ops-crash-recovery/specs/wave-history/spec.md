## ADDED Requirements

### Requirement: Wave runs are logged

Every wave SHALL leave a row: `WaveTotals` opens it (wave, key, parent,
total), the terminal closes it with settled counts (`done`).

#### Scenario: A finished wave is inspectable later

- **WHEN** a transcript backfill completes and the app reloads
- **THEN** the Pipelines section still shows the wave with its settled/total
  counts and done state — not just waves open right now

### Requirement: Crashed waves are marked abandoned

Waves left open by a dead session SHALL close as `abandoned` at boot, never
linger open and never silently vanish.

#### Scenario: A crash mid-wave reads honestly

- **WHEN** the daemon dies mid-sync-pass and restarts
- **THEN** the pass's wave row reads `abandoned` with its last known
  totals — the UI never shows it as still running

### Requirement: Recent wave history is served

`GET /api/tasks/waves` SHALL serve recent runs (open + closed, filterable
by wave, capped with exact totals), and the Pipelines/Sync sections SHALL
render history beside live waves.

#### Scenario: History beside live state

- **WHEN** the user opens Pipelines after a quiet hour
- **THEN** the last completed waves render with counts and done state, above
  the "no open wave" line — history and live state in one glance, never
  confused
