## ADDED Requirements

### Requirement: Transcript sources watched

Transcript source dirs (per adapter) SHALL be watched alongside watch
roots, and file add/modify/delete SHALL enqueue a narrowed `IngestCapture`
for that file — not a full backfill.

#### Scenario: A new session ingests on arrival

- **WHEN** a new transcript file appears in a watched source dir
- **THEN** an ingest task for that file is queued promptly, and its parse,
  upsert, and downstream analysis emit progress on the uniform stream

#### Scenario: An edited transcript reprocesses incrementally

- **WHEN** a transcript file changes
- **THEN** only units past the per-file cursor are parsed and processed,
  with totals and terminal frames for the incremental unit

### Requirement: Consent-gated and debounced

Change-driven ingest SHALL respect consent (fail closed) and debounce
rapid appends into one unit of work.

#### Scenario: Rapid turns coalesce

- **WHEN** a transcript appends many turns in seconds
- **THEN** they coalesce into one ingest unit per file, and consent
  withdrawn mid-flight stops the ingest with a coded refusal

### Requirement: Backfill stays the safety net

Boot and scheduled backfill SHALL remain, watermark-driven, as the net
under change-driven ingest.

#### Scenario: A missed change is caught by the sweep

- **WHEN** a change evades the watcher (restart, overflow, new adapter dir)
- **THEN** the next backfill sweep ingests it via the same per-file cursors,
  emitting the same progress shapes

### Requirement: Isolation extends to transcript watching

Isolated instances SHALL never watch real transcript dirs; e2e SHALL get a
test-mode flag pointing adapters at fixture dirs.

#### Scenario: Transcript e2e runs against fixtures

- **WHEN** the e2e harness enables transcript test mode
- **THEN** ingest and change-driven scenarios run against fixture dirs only,
  and the default isolated path still ingests nothing
