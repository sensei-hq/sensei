# Change: The broker keeps the host healthy — janitor, health sentinel, shared build (#444)

## Why
Design §10 (`docs/design/relay-autonomous-queue-and-planning.md:1245-1350`) splits host upkeep from agent
work: lanes should not spend turns on disk space, cleanup or build contention. §10.4 (the merge gate) shipped
with the queue (`crates/senseid/src/relay/gate.rs`). §10.1–10.3 were deferred
(`docs/plan/2026-11-10-relay-autonomous-queue/README.md:38`) and the backlog row still lists them as unbuilt
(`docs/backlog.md:43`). The incident behind them: parallel lanes took free disk from 168 GB to 17 GB and swap
to 40/42 GB, killing a reviewer mid-run (§10.1).

## What is already on develop
- Disk and swap readings: `disk_free_gb()` (statvfs on `/`) and `swap_free_gb()` (macOS `vm.swapusage`)
  at `crates/senseid/src/db/pg_store/relay.rs:1078-1119`.
- Resource leases: `POST /api/relay/resources/request|release` (`crates/senseid/src/api/routes.rs:131-132`,
  store `relay_request_resource` at `pg_store/relay.rs:2080`). One grant per resource, a FIFO queue, a 3600 s
  TTL, and a grant refused below `DISK_FLOOR_GB = 25` / `SWAP_FLOOR_GB = 1` (`crates/senseid/src/relay/mod.rs:1108-1156`).
  The MCP tool `relay_request_resource` names slots like `rust-build` and `e2e` (`crates/mcp/src/lib.rs:1782`).
- The Relay footer already shows the build holder, disk free and swap free from `GET /relay/now`
  `resources` (`pg_store/relay.rs:7495-7505`, `app/src/lib/relay/relay-shell.ts:239-268`).

So the serialized build slot of §10.3 item 1 exists. What is missing: any periodic probe, any watermark,
any claim throttling, any cleanup, broker-executed builds, and a shared build cache.

## What Changes
- **Janitor (§10.2).** A four-stage cleanup cascade run on task `done`/`cancelled`, hourly (new
  `relay_janitor` schedule), and at the warning watermark: prune dangling lane worktrees (clean ones only),
  scavenge stale incremental caches and the gate's leftover verify worktrees, run safe manifest clean targets, compact
  execute logs older than 7 days. Each run is recorded and the footer shows the last cleanup.
- **Health sentinel (§10.1).** A `relay_health` schedule (15 s) classifies the host `normal | warning |
  critical` from disk free and swap used. Warning emits a `health` event and starts the janitor. Critical
  refuses `claim_next`, `claim` and `claim_review` with 503 `relay_host_disk_exhausted`, runs the janitor at
  once, and asks the person when free disk stays under 20 GB. Watermarks are config keys.
- **Shared build (§10.3).** `POST /api/relay/resources/execute` runs a manifest target (never a shell string)
  in the lane's own checkout under the `shared-build` lease, streams output, records exit code, duration and
  peak memory, and releases the slot. Broker builds share one `CARGO_TARGET_DIR` per repository and use
  `sccache` when installed. MCP gains `relay_execute`; the lane guidance says heavy builds take the lease.
- The footer reads the new health fields: disk tone follows the level, a "Claims paused" item shows at
  critical, "Shared build: <lane> (<n> min)" shows while a broker build runs.
- Backlog relay row updated once all three land.

## Impact
- Specs: new capability `relay-host-ops`.
- Code: `crates/senseid/src/relay/` (new `health.rs`, `janitor.rs`, `execute.rs`), `tasks/` (two new
  schedules), `api/handlers/relay.rs` (claim guards, execute route), `pg_store/relay.rs` (now `resources`,
  janitor/execute records), `adapters/manifest/` (Makefile target parsing), `crates/mcp` (one tool + pins),
  `packages/state/relay/types.ts`, `app/src/lib/relay/relay-shell.ts`.
- Data: `database/import/staging/schedules.jsonl` (two rows); no table changes — runs are recorded in
  `relay.leases.props` and `sensei.config`.
- e2e: `app/e2e/tests/relay-janitor.spec.ts`, `relay-health.spec.ts`, `relay-execute.spec.ts`.
- Not here: changing how lanes run their own builds (they keep their worktrees; only broker builds share a
  target dir), the dōjō relay screen, the §10 host page (no mockup).
