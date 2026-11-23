## ADDED Requirements

### Requirement: File tasks record per-phase timings

`process_file` SHALL record `parse_ms`, `world_load_ms`,
`compile_wait_ms`, `resolve_ms`, and `write_ms` splits through a
task-local outbox (one `TaskContext` serves every worker, so a context
field would mix concurrent tasks); the executor SHALL persist them to
`task_executions.phases_ms` (nullable JSONB) on completion. Phases sum
to no more than wall time; absent phases read as missing keys, never
zero.

#### Scenario: A slow file attributes itself

- **WHEN** a file takes 80s with a 70s compile wait
- **THEN** its execution row carries
  `{"compile_wait_ms": 70000, ...}` and the breakdown is queryable

### Requirement: Phase spans aid live debugging

Each phase SHALL run under a debug-level tracing span naming the file,
so a wedged scan shows its phase distribution in the log without a
profiler. Spans cost nothing when disabled.

#### Scenario: A stuck scan names its phase

- **WHEN** debug logging is on during a slow file
- **THEN** the log shows which phase each in-flight file sits in
