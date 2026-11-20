# Tasks: relay host ops (#444)

Three slices, each mergeable to develop on its own, each red-first (a failing-test commit before the code that
passes it) and each ending with `cargo fmt --all --check`, `cargo clippy -- -D warnings`, `make test-fast`,
the relay DB suite (`--test-threads=1`) and its e2e spec green.

## Slice 1 — Janitor (§10.2)

### 1. Makefile targets
- [ ] 1.1 Red: `adapters/manifest/make.rs` tests — `parse_make_targets` lists rule targets, skips variable
  assignments (`X := y`), pattern rules (`%.o:`), special targets (`.PHONY`) and recipe lines; categories via
  `command_category::categorise`.
- [ ] 1.2 Green: implement the pure parser returning `DiscoveredCommand` (no registry change).

### 2. Cascade (pure plan + executor seam)
- [ ] 2.1 Red: `relay/janitor.rs` tests for `plan_cleanup(inputs) -> Vec<Step>`: a clean unmerged-free
  retired worktree → `RemoveWorktree`; dirty → `Keep{dirty}`; unmerged → `Keep{unmerged}`; a registered
  lane's worktree → nothing; Makefile with `clean`, `clean-cache`, `build` → only `make clean-cache`;
  `pnpm-lock.yaml` → `pnpm store prune`; incremental entries 30 h / 1 h → only the 30 h one; exec logs 8 d /
  1 d → only the 8 d one; a `$TMPDIR/sensei-gate-*` dir 30 h old → removed, 1 h old → kept; any other temp-dir path → never.
- [ ] 2.2 Green: `plan_cleanup` + a `JanitorRunner` trait (`GitJanitorRunner` real, fake for tests) that runs
  steps and measures reclaimed bytes with `disk_free_gb` before/after.
- [ ] 2.3 Red: tempdir integration test — a real git repo with a retired clean worktree and a dirty one; the
  real runner removes the first, keeps the second, and `git worktree list` agrees.
- [ ] 2.4 Green: make it pass; one run at a time via `pg_advisory_lock(hashtext('relay_janitor'))`.

### 3. Triggers, record, event
- [ ] 3.1 Red: store test — a janitor run writes `sensei.config` key `relay.janitor.last_run`
  (`{at, reason, reclaimed_gb, kept:[{path, reason}]}`); `relay_now` `resources.last_cleanup` reads it (null
  when never run; a read failure is an error, never null).
- [ ] 3.2 Green: write + read.
- [ ] 3.3 Red: handler tests — every path to `done`/`cancelled` spawns a janitor run (fake runner counts) and
  emits `StateEvent::relay("janitor")`: task cancel, run cancel, manual merge, approval with no branch, and
  the gate's `done`.
- [ ] 3.4 Green: spawn the janitor on its own task after each of: handlers `cancel` (`relay_cancel_task`,
  `pg_store/relay.rs:5617`), `cancel_run` (`relay_cancel_run`), `merge` (`relay_merge_task`, :5517), `review`
  (`relay_submit_review`, :3012 — approval with nothing to merge and Task 0 unpack end at `done`), and the
  gate outcome in `relay_lease_sweep.rs` `pass` (`relay_gate_outcome`, :3521).
- [ ] 3.5 Red: `schedule.rs` pin — `SCHEDULABLE` and `sensei.schedules` both hold `relay_janitor`.
- [ ] 3.6 Green: add `relay_janitor` to `SCHEDULABLE` (`tasks/schedule.rs:23`), a row in
  `database/import/staging/schedules.jsonl` (3600 s), `tasks/relay_janitor.rs` spawn via
  `ticker::run_scheduled`, register in `api/server.rs` beside `relay_lease_sweep::spawn` (line 566);
  `dbd import` the new row locally.

### 4. Footer
- [ ] 4.1 Red: `relay-shell.spec.ts` — `resources.last_cleanup` renders "Cleaned 3 GB · 10 min ago"; null
  renders nothing extra; `relay-shell-input.spec.ts` maps the field. Types in
  `packages/state/relay/types.ts` (`RelayResources.last_cleanup`) and fixtures.
- [ ] 4.2 Green: `relay-shell.ts` footer + input mapping.

### 5. e2e
- [ ] 5.1 `app/e2e/tests/relay-janitor.spec.ts`: mint a crew + lane over HTTP (pattern of
  `relay-sprint.spec.ts`), create a temp git repo with a clean worktree registered as a second lane's
  `worktree`, unregister that lane, open Relay, cancel a task over HTTP (task cancel is coordinator-only:
  `relay_cancel_task` checks `relay_crew_of_coordinator`, so mint a coordinator lane); assert the worktree directory is gone
  and the footer shows "Cleaned" with no refresh. Run with `make test-app-e2e` (check
  `pgrep -f "instance e2e"` first; confirm `brew services start sensei` restored the real daemon after).

## Slice 2 — Health sentinel (§10.1)

### 6. Classifier and probe
- [ ] 6.1 Red: `relay/health.rs` table test for `classify` at every boundary in the spec scenario, unknown
  readings, and the two-probe rise to critical / immediate fall.
- [ ] 6.2 Green: `classify`, `Watermarks`, `HostProbe` trait, `StatvfsProbe` reusing `disk_free_gb` and a new
  `swap_used_pct` beside `swap_free_gb` (`pg_store/relay.rs:1078-1119`; move both readers into `health.rs`
  so there is one home).
- [ ] 6.3 Red: config tests — `relay.health.*` keys accept positive integers (per key, in
  `config_value_is_valid`, `api/handlers/config.rs:68`) and refuse the rest with 400; a PUT that leaves
  critical at or above warn (body merged over stored values and defaults) is refused with 400 in
  `set_config_handler`, nothing written.
- [ ] 6.4 Green: validation.

### 7. Sentinel task and state
- [ ] 7.1 Red: `tasks/relay_health.rs` tests with a fake probe — a level change writes `SharedState` health
  and emits `StateEvent::relay("health")`; an unchanged level emits nothing; warning and critical spawn the
  janitor (fake counts).
- [ ] 7.2 Green: schedule `relay_health` (15 s) in `SCHEDULABLE` + `schedules.jsonl`; spawn in `server.rs`.
- [ ] 7.3 Red: `relay_now` test — `resources.health` carries `{level, disk_free_gb, swap_used_pct,
  watermarks, checked_at}`.
- [ ] 7.4 Green.

### 8. Throttle and escalation
- [ ] 8.1 Red: handler tests — at critical, `claim_next`, `claim`, `claim_review` answer 503
  `relay_host_disk_exhausted`; `checkpoint`, `report`, `release` still succeed; at warning all claims
  succeed. `RelayError::HostExhausted` code/status pins in `relay/mod.rs` tests.
- [ ] 8.2 Green: the variant (`relay/mod.rs:1028`, status 503 in `status` :1068, code in `code` :1087, a
  message arm in the handler `err` mapper `api/handlers/relay.rs:85`) and the guard in the three handlers
  (`claim_next` :418, `claim_task` :444, `claim_review` :1396).
- [ ] 8.3 Red: store tests — critical with free disk below `recover_gb` after the janitor files one person ask
  per crew with a live lane; a second critical probe files none; a probe below critical answers the open asks.
- [ ] 8.4 Green: ask writer reusing the gate's ask insert (`pg_store/relay.rs:3706-3735`, extract a shared
  helper), self-answer path, `decision_asked` events.

### 9. Footer
- [ ] 9.1 Red: `relay-shell.spec.ts` — disk tone success/warning/danger by level; "Claims paused" item at
  critical; unknown readings stay muted.
- [ ] 9.2 Green.

### 10. e2e
- [ ] 10.1 `app/e2e/tests/relay-health.spec.ts`: open Relay; `PUT /api/config` `relay.health.disk_warn_gb`
  above real free disk → disk item turns warning within 45 s, no refresh; set warn, critical and recover above
  free disk, warn highest (the pair rule refuses critical ≥ warn) → "Claims paused" and the ask in Needs you; `claim_next` over HTTP answers 503; `DELETE
  /api/config/relay.health.*` → both clear and `claim_next` answers 200. Always reset the keys in `afterEach`.

## Slice 3 — Shared build (§10.3)

### 11. Execute
- [ ] 11.1 Red: `relay/execute.rs` tests — request validation (unknown lane, manifest path escaping the lane
  checkout, target not listed, shell metacharacters) refuses before spawn; cargo targets get
  `CARGO_TARGET_DIR=<main checkout>/target/shared` and `RUSTC_WRAPPER` only when `sccache` resolves via
  `sensei_bootstrap::util::which_binary`.
- [ ] 11.2 Green: resolver + env builder.
- [ ] 11.3 Red: DB-backed handler test — execute on a temp repo Makefile target streams `line` events then
  `done{exit_code:0}`, writes the log file, stores the summary in the lease props and releases the lease; a
  second lane's execute waits until the first is done; a failing target reports its exit code; a disconnect
  releases the lease.
- [ ] 11.4 Green: route `POST /api/relay/resources/execute` in `api/routes.rs` beside line 131, SSE response,
  supervised child (extract `agent_spawn`'s timeout/kill/reap into a shared helper; `run_agent` buffers to
  exit and cannot stream), `libc::wait4` for `ru_maxrss` normalised to KiB (bytes on macOS), the queued
  lease polled until granted, secret scan (`crate::secret_scan::scan`) on lines, `emit("resources.request"/
  "resources.release")` as the request/release handlers do so the footer updates live.
- [ ] 11.5 Janitor stage 4 now has real logs: extend the 2.1 test to a log written by execute.

### 12. MCP and guidance
- [ ] 12.1 Red: MCP tool-list pins and dispatch test for `relay_execute` (`crates/mcp/src/lib.rs:754`, 1782).
- [ ] 12.2 Green: the tool; guidance text tells lanes to build through it or a `shared-build` lease; both
  `SKILL.md` copies updated if they describe resources.

### 13. Footer and e2e
- [ ] 13.1 Footer already shows the build holder; red/green only if `resourceName('shared-build')` needs a
  label change ("Shared build").
- [ ] 13.2 `app/e2e/tests/relay-execute.spec.ts`: a lane with a temp-repo worktree whose Makefile `build`
  sleeps 3 s; start execute over HTTP without awaiting; the footer shows "Shared build: <code>" then "No build
  running" with no refresh; the stream ends `done` `exit_code: 0`.

### 14. Close-out
- [ ] 14.1 Update the relay row in `docs/backlog.md` (§10.1–10.3 built), the plan README line 38, and the
  design §10 with a "Deviations" note for the defaulted decisions.
- [ ] 14.2 `openspec validate add-relay-host-ops --strict`; comment on #444 with the commits and e2e runs;
  final commit says `Closes #444`.
