---
type: design
---

# API / daemon — module

Behind-the-scenes design for the daemon that backs every feature doc —
[Observatory](../features/03-observatory.md), [Project](../features/04-project.md),
[Setup](../features/01-setup.md), [Config](../features/02-config.md),
[Governance](../features/05-governance.md), [Relay](../features/06-relay.md).
The feature docs say what the user sees; this says how the engine underneath
runs it. See also [`../architecture/daemon.md`](../architecture/daemon.md) and
[`../architecture/data.md`](../architecture/data.md) for the deeper layer specs
— this doc is the practical map for touching the code.

## The binary

- Crate: `crates/senseid` (bin `senseid`). One Axum HTTP server over one
  Postgres pool, port **7744**, single mode (no dev/prod split).
- Entry: `src/api/server.rs` builds the router; `src/api/routes.rs` is the full
  route table (`.route(...)` calls) — the fastest way to find any endpoint.
- `src/api/state.rs` — `SharedState`/`AppState`: `PgStore`, `TaskQueue`,
  `Arc<Gateway>`, a `broadcast::Sender<StateEvent>` for SSE, and the
  capture-watchdog `BreakerMap`.
- Clients: app (Tauri sidecar), cli, mcp — all HTTP to `:7744`. No client talks
  to Postgres directly.

## Route surface

- Handlers live under `src/api/handlers/` — one file per domain: `workspace.rs`
  (repos/projects), `observatory.rs` (solutions/summary), `project_detail.rs`
  (ftr/drift/patterns/libraries/memories/recommendations), `gateway.rs` /
  `gateway_routers.rs` / `gateway_chains.rs` / `gateway_image.rs`,
  `scan_events.rs`, `playbook.rs`, `runs.rs`, `instruments.rs`, `sessions.rs`,
  `mcp.rs` / `mcp_manifests.rs` / `mcp_servers.rs`, `logs.rs`,
  `scheduled_tasks.rs`, `tool_signals.rs`, `query.rs`, `dojo.rs`.
- `src/api/routes.rs` is grouped by domain in the same order — read it top to
  bottom to see the whole surface at once.

## SSE event stream

- Single shape for every stream: `StateEvent { action, entity, data }`
  (`src/api/events.rs`) — mirrors the app's `StateEvent<T>` client interface.
  `action` ∈ Add/Update/Remove/Set; `entity` routes the payload to the right
  client-side state context.
- Transport: `broadcast::Sender<StateEvent>` on `SharedState`, fanned out over
  `/api/scan/events` (`handlers/scan_events.rs`) — carries scan/index progress
  *and* assistant-part registration events on the same channel (see
  [setup-and-config.md](setup-and-config.md#folder-scan)).
- Any task that wants to push a live update sends into `event_tx`; the app's
  SSE client dispatches by `entity`.

## The gateway (routing inference + actions)

- Consumed as the `gateway-embedded` git dependency (sibling repo
  `sensei-hq/gateway`, formerly in-tree `crates/gateway/`); daemon wraps it in
  `Arc<Gateway>` on `SharedState`.
- Config loaders: `src/api/gateway_init.rs` (boot-time load) and
  `src/api/gateway_config_loader.rs` (table-driven from the `gateway.*` schema:
  routers → models → named chains — `classify`, `reasoning`, `embed`,
  `insight-copy`, `image`). Hot-reloads via `Arc<RwLock<GatewayConfig>>`, no
  daemon restart.
- Selection is 3-tier (exact adapter+model → named chain → capability), each
  candidate gated 4 ways (router-enabled+key → supports-capability →
  breaker-not-open → within-budget); budget rule is "never block, always
  degrade" — drop external → local-only → Noop. See
  [`../architecture/daemon.md#gateway-internals`](../architecture/daemon.md#gateway-internals)
  for the full mechanics; this doc only maps the code.
- Local-first: embedded gemma / all-minilm always available offline; cloud legs
  are router-gated additions, never a hard dependency.
- API/routes: `handlers/gateway.rs` (`infer`/`embed`/`consensus`,
  `/api/gateway/status`), `handlers/gateway_routers.rs` (router/provider/model
  listing, key set/clear), `handlers/gateway_chains.rs` (chain role + members),
  `handlers/gateway_image.rs` (image generation chain).
- Auth: cloud provider API keys are BYOK, stored in the macOS Keychain via
  `src/gateway_keys/mod.rs` (`set_key`/`get_key`/`delete_key`, shells to
  `security`) — never in Postgres or the DDL config tables. Provider/router
  definitions (no secrets) live in `src/gateway_routers/mod.rs` +
  `gateway.routers`/`gateway.models` tables. No OAuth flow for gateway
  providers today — key-based only; OAuth exists only for Dōjō membership
  (`src/dojo/memberships.rs`, `handlers/dojo.rs`) and is a separate concern.
- Persistence: `GatewayStore` trait implemented on Postgres —
  `gateway.inference_calls` + `gateway.execution_traces` (the execution trace
  records the `SkipReason` per gate for observability).

## The core loop (capture → graph → analyze → learn → deliver)

- Driven by `src/tasks/` (`TaskQueue`, `TaskKind` executor) — scheduled
  (analyzer ticks) and event-triggered (watcher, hooks, HTTP) work, with
  barriers so post-processing waits for children to settle. Full flowchart:
  [`../architecture/daemon.md#the-task-system`](../architecture/daemon.md#the-task-system).
- Capture: `src/assistants/` (adapter health + hourly watchdog,
  `BreakerMap` on `SharedState`), `src/transcript/` (session/turn
  reconstruction), `src/watcher/` (FSEvents).
- Graph: `src/indexer/`, `src/languages/` + `src/adapters/` (per-language IR →
  nodes/edges), `src/ir.rs` (the adapter IR types).
- Analysis/learning: `src/analysis/` (signals, patterns, recommendations),
  `src/collective/` (consolidation/consensus callers of the gateway),
  `src/instruments/` (tool-usage stats), `src/federation/` (cross-repo).
- `src/playbook.rs` — the recommend-and-confirm engine behind Intake
  (front-door). `src/run_watchdog.rs` — per-task hang detection.
- Storage: `src/db/` (`PgStore` — the one Postgres access layer; nothing else
  touches the DB directly). Schema/conventions: [`../architecture/data.md`](../architecture/data.md).

## Installer / bootstrap

- `src/installer/` + `src/model_provision.rs` — brew-tap install, embedded
  model provisioning at first boot. Health-gate design (probing → auto-fixing
  → all-green) lives in `crates/bootstrap`, documented in
  [setup-and-config.md](setup-and-config.md#health-gate-bootstrap) — the
  daemon is one of the six gates it probes, not the prober itself.

## Not built / future

- No OAuth for gateway cloud providers (key-only today); Dōjō-side OAuth is
  unrelated and already shipped.
- Relay driver code exists (`src/relay_drivers/`) but the daemon-coordinator
  supervision path is still forming — see [`../features/06-relay.md`](../features/06-relay.md)
  and the vacation-run/Beta-Relay design notes for current status.
- Silent-error audit across handlers/tasks is a standing follow-up (no `.ok()`
  swallowing allowed — always `tracing::warn!`).
