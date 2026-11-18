## ADDED Requirements

### Requirement: Janitor cleanup cascade
The broker SHALL run a cleanup cascade in four stages, in order: prune dangling lane worktrees, scavenge
stale incremental caches and the gate's leftover verify worktrees, run allow-listed manifest clean targets, and delete execute
logs older than 7 days. It SHALL never remove a worktree with uncommitted changes or unmerged commits, never
run `make clean` or a whole `cargo clean`, and never touch paths outside known checkouts, the gate's `sensei-gate-*` temp worktrees and
`~/.sensei/relay/exec/`. Only one run SHALL proceed at a time.

#### Scenario: A retired lane's clean worktree is pruned
- **GIVEN** an unregistered lane whose `worktree` is a clean checkout with no unmerged commits
- **WHEN** the janitor runs
- **THEN** the worktree directory is removed and `git worktree list` in the base checkout no longer names it

#### Scenario: A dirty worktree is kept and reported
- **GIVEN** an unregistered lane whose worktree has an uncommitted file
- **WHEN** the janitor runs
- **THEN** the worktree is untouched and the run record lists it under `kept` with the reason `dirty`

#### Scenario: Only allow-listed make targets run
- **GIVEN** a base checkout whose Makefile defines `clean`, `clean-cache` and `build`
- **WHEN** the janitor runs
- **THEN** it runs `make clean-cache` and runs neither `make clean` nor `make build`

#### Scenario: Stale incremental caches go, fresh ones stay
- **GIVEN** `target/debug/incremental` holds one entry modified 30 h ago and one modified 1 h ago
- **WHEN** the janitor runs
- **THEN** the 30 h entry is removed and the 1 h entry remains

### Requirement: Janitor triggers and record
The janitor SHALL run when a task reaches `done` or `cancelled`, on the hourly `relay_janitor` schedule, and
when the health level reaches warning or critical. Each run SHALL record when it ran, why, the GB reclaimed,
and what it kept, and SHALL emit a relay `janitor` event. `GET /relay/now` `resources.last_cleanup` SHALL
carry the latest run, and the Relay footer SHALL show it.

#### Scenario: Cancelling a task cleans up with no refresh
- **GIVEN** the Relay screen is open and an unregistered lane owns a clean worktree
- **WHEN** a task is cancelled (the crew's coordinator lane calls `POST /api/relay/tasks/{id}/cancel`)
- **THEN** the worktree is removed and the footer shows "Cleaned <n> GB · just now" without a manual refresh
  (proved by `app/e2e/tests/relay-janitor.spec.ts`)

#### Scenario: The schedule exists
- **WHEN** the daemon starts on a converged database
- **THEN** `sensei.schedules` holds `relay_janitor` at 3600 s and `SCHEDULABLE` lists it

### Requirement: Host health sentinel
The broker SHALL probe free disk and swap used every 15 s (`relay_health` schedule) and classify the host as
`normal`, `warning` (disk free at or below `relay.health.disk_warn_gb`, default 25, or swap used at or above
`relay.health.swap_warn_pct`, default 60) or `critical` (disk free below `relay.health.disk_critical_gb`,
default 15, or swap used above `relay.health.swap_critical_pct`, default 80). An unknown reading SHALL NOT
raise the level. Critical SHALL take two consecutive critical probes; a lower level SHALL apply at once.
Each level change SHALL emit a relay `health` event, and `GET /relay/now` `resources.health` SHALL carry the
level, readings and watermarks.

#### Scenario: Boundaries classify
- **GIVEN** watermarks 25/15 GB and 60/80 %
- **THEN** 26 GB with 10 % swap is normal, 25 GB is warning, 15 GB is warning, 14 GB is critical, 61 % swap is
  warning, 81 % swap is critical, and an unknown disk with 10 % swap is normal

#### Scenario: Warning shows on the footer live
- **GIVEN** the Relay screen is open
- **WHEN** `relay.health.disk_warn_gb` is set above the machine's free disk
- **THEN** within two probes the footer's disk item turns to the warning tone and a janitor run is recorded,
  without a manual refresh (proved by `app/e2e/tests/relay-health.spec.ts`)

#### Scenario: Watermarks are validated
- **WHEN** a config write sets `relay.health.disk_critical_gb` above `relay.health.disk_warn_gb`, or a
  watermark to a non-integer
- **THEN** `PUT /api/config` answers 400 and nothing is written

### Requirement: Claims pause at critical
The claim routes SHALL refuse while the level is critical: `POST /api/relay/tasks/claim_next`,
`POST /api/relay/tasks/{id}/claim` and `POST /api/relay/tasks/{id}/claim_review` answer 503 with code `relay_host_disk_exhausted` and a message
naming the short reading. Checkpoints, reports, reviews of already-claimed tasks and releases SHALL stay open.
The broker SHALL run the janitor at once and, if free disk is still below `relay.health.recover_gb` (default
20) afterwards, file one open `asked` action of the person in each crew with a live lane; it SHALL not file a
second while one is open, and SHALL answer its own asks when the level falls below critical.

#### Scenario: A claim is refused at critical
- **GIVEN** the level is critical
- **WHEN** a live lane calls `claim_next`
- **THEN** the answer is 503 with `error: "relay_host_disk_exhausted"` and a `message` naming the short reading, and no task changes state

#### Scenario: The person is asked, then the ask clears
- **GIVEN** the Relay screen is open and `relay.health.disk_warn_gb`, `relay.health.disk_critical_gb` and
  `relay.health.recover_gb` are set above the machine's free disk, warn above critical
- **WHEN** two probes pass
- **THEN** the footer shows "Claims paused" and Needs you shows the broker's free-space ask, without a refresh
- **WHEN** the watermarks are reset to their defaults
- **THEN** the next probe clears "Claims paused", the ask leaves Needs you, and `claim_next` answers 200
  (proved by `app/e2e/tests/relay-health.spec.ts`)

### Requirement: Broker-executed builds
`POST /api/relay/resources/execute {lane_id, manifest, target, timeout_ms?}` SHALL run a target the manifest
parser lists for the lane's own checkout, under the `shared-build` resource lease, and answer an event stream
of `line` events (secret-scanned, each at most 600 chars) and one `done` event with `exit_code`, `duration_ms`
and `max_rss_kb`. The lease `props` SHALL keep that summary, the full output SHALL go to
`~/.sensei/relay/exec/<lease_id>.log`, and the lease SHALL be released on exit, error or disconnect. A target
the manifest does not list, a manifest outside the lane's checkout, or an unknown lane SHALL be refused before
anything runs. Cargo targets SHALL run with `CARGO_TARGET_DIR` set to the main checkout's `target/shared`, and
with `RUSTC_WRAPPER=sccache` when `sccache` is installed.

#### Scenario: A build runs and the footer shows it
- **GIVEN** the Relay screen is open and a lane whose worktree has a Makefile target `build` that sleeps 3 s
- **WHEN** the lane calls execute with `manifest: "Makefile"`, `target: "build"`
- **THEN** the footer shows "Shared build: <lane code> (<n> min)" while it runs and "No build running" after, without a
  refresh, and the stream ends with `done` and `exit_code: 0` (proved by `app/e2e/tests/relay-execute.spec.ts`)

#### Scenario: A second build queues
- **GIVEN** lane A's execute holds `shared-build`
- **WHEN** lane B calls execute
- **THEN** B's build starts only after A's `done`, and both lease rows record their summaries

#### Scenario: An unlisted target is refused
- **WHEN** a lane calls execute with `target: "rm -rf /"` or a target the Makefile does not define
- **THEN** the answer is 422 `relay_invalid` and no process starts

### Requirement: Lanes are told to share builds
The MCP server SHALL expose `relay_execute`, which calls the execute route, collects the stream and returns the
`done` summary plus the last 600 chars of output. The lane guidance SHALL tell lanes to run heavy builds
through `relay_execute` or inside a `shared-build` lease.

#### Scenario: The tool is listed
- **WHEN** an MCP client lists tools
- **THEN** `relay_execute` is present with `manifest` and `target` arguments, and the tool-list pins count it
