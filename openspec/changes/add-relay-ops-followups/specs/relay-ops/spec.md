## ADDED Requirements

### Requirement: The sentinel classifies host health every 15 seconds
The daemon SHALL probe disk free space and swap use every 15 seconds and
classify: Normal (disk > 25 GB and swap < 60%), Warning (disk ≤ 25 GB or
swap ≥ 60%), Critical (disk < 15 GB or swap > 80%). A failed probe SHALL
count as Normal (a blind sentinel must not halt work) and log once.

#### Scenario: Watermarks
- **WHEN** disk reads 20 GB free and swap 50%
- **THEN** the verdict is Warning
- **WHEN** disk reads 40 GB and swap 85%
- **THEN** the verdict is Critical

### Requirement: Warning warns and cleans; Critical throttles and escalates
On Warning the daemon SHALL emit SSE `health_warning` and trigger an
async janitor run. On Critical both claim endpoints SHALL answer 503
`relay_host_disk_exhausted`, the janitor SHALL run synchronously, and
when cleanup recovers < 20 GB the daemon SHALL ask the person
(`open_asked`, `asked_of: person`) to free storage.

#### Scenario: Critical throttles claims
- **GIVEN** disk under 15 GB free
- **WHEN** a lane claims next
- **THEN** it answers 503 `relay_host_disk_exhausted`

### Requirement: The janitor reclaims in four stages on three triggers
Task done/cancelled, an hourly pass, and the warning watermark SHALL each
run the cascade: prune dangling worktrees, scavenge stale incremental /
scratch dirs, run manifest clean targets (`make clean-temp`, `pnpm store
prune`, scoped `cargo clean -p`), and compact 7-day-old log attachments
(keeping action metadata). Each stage SHALL report bytes reclaimed; a
stage that fails SHALL log and continue the cascade.

#### Scenario: Done task prunes its worktree
- **GIVEN** a task whose lane unregistered with its worktree on disk
- **WHEN** the task completes
- **THEN** the worktree directory is gone and the reclaim is reported

### Requirement: Heavy builds serialize on the shared-build lease
Before a broker-executed build, the daemon SHALL hold the machine-wide
`shared-build` resource lease (queuing behind another build); lanes MAY
offload builds via `POST /api/relay/resources/execute` (manifest +
target + worktree), whose streamed output, memory and duration land in
`props` and whose slot releases on completion. Worktrees SHALL share
`CARGO_TARGET_DIR`, `sccache`, and the pnpm store.

#### Scenario: Two builds serialize
- **GIVEN** lane A holds the shared-build lease
- **WHEN** lane B executes a build
- **THEN** B waits for A's release, and its output, memory and duration
  are recorded
