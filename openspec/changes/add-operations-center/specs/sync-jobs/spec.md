## ADDED Requirements

### Requirement: Sync passes are first-class jobs

Sync passes SHALL surface as jobs with identities, progress, terminal
states, and coded errors — visible wherever queue jobs are.

#### Scenario: A sync pass shows live progress

- **WHEN** a sync pass runs across personas and entities
- **THEN** its per-entity progress, terminal states, and any errors appear
  in the same Now/Queue/Failures views as queue tasks

### Requirement: Sync errors carry codes and remedies

Sync failures SHALL keep their per-entity error, attempt time, and remedy,
and plan-fetch failures SHALL stay attributed per persona.

#### Scenario: A failing persona names its cause

- **WHEN** one persona's plan fetch fails
- **THEN** the failure is attributed to that persona with its error and
  remedy, without failing the whole pass dishonestly
