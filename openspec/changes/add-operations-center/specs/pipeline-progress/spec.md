## ADDED Requirements

### Requirement: Totals for every pipeline

Every pipeline SHALL emit `FolderQueued`-style totals (units expected) when
a wave starts.

#### Scenario: Transcript and metric waves state their size

- **WHEN** a transcript backfill or metric computation wave starts
- **THEN** it emits its total units (files, groups) so progress bars have a
  denominator from the first frame

### Requirement: Terminal frames end every pipeline

Pipelines SHALL complete on terminal `done` frames, and the dead index
`Indexed` emitter SHALL be fixed to the real terminal kind.

#### Scenario: Index completion arrives on the stream

- **WHEN** the terminal index task finishes a folder
- **THEN** a folder `update` with `done:true` arrives and the workspace
  flips without any poller deciding completion

### Requirement: Wave aggregation linkage

Wave children SHALL link to their parent so task-follow aggregation works
for every pipeline.

#### Scenario: A metric wave aggregates its groups

- **WHEN** a metric computation fans out per-group tasks
- **THEN** each child links its parent and the wave's settled/total counts
  aggregate through the same follow path as transcript waves

### Requirement: Dropped counts surface

Known-dropped counts SHALL surface instead of vanishing: transcript repair
counts, per-file turn counts, and embed-tail follow-ups.

#### Scenario: A backfill reports what it repaired

- **WHEN** a transcript backfill repairs sessions
- **THEN** the repaired count is emitted on the stream, not computed and
  dropped

### Requirement: Uniform stage protocol

Pipeline stages SHALL use the `StageStarted`/`StageCompleted`/`StageFailed`
events in production, not only in tests.

#### Scenario: Long stages narrate themselves

- **WHEN** a pipeline stage starts and ends
- **THEN** stage frames name the stage and its outcome so a follower sees
  phase transitions, not silence between totals and terminals
