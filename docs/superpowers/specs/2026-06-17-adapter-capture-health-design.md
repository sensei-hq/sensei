---
name: ACP Adapter Health & Capture Watchdog
description: Daemon-owned per-adapter health checker + auto-resolver, an hourly capture watchdog with circuit breaker + system notifications, and a sensei doctor section that consumes it. The durable fix for silently-broken capture ("circling").
date: 2026-06-17
---

# ACP Adapter Health & Capture Watchdog

## Problem

The sensei↔Claude integration lives in `~/.claude` config that gets wiped by Claude
Code updates/resets, and it fails **silently** — hooks just stop firing and the gap
is discovered weeks later by manually counting `activity.hook_events` rows. This has
caused months of recurring "circling": reinstall, work for a while, silently break,
rediscover, reinstall. See `memory/project_pickup_2026_06_17.md` and `[[project_dogfooding_observability]]`.

The fix is **detection + bounded auto-repair + a loud signal when auto-repair can't
help** — not another manual reinstall.

## Goals

- The **daemon owns** the verification. CLI (`sensei doctor`) and the desktop app are
  thin consumers of a daemon endpoint, so all three agree on one source of truth.
- Generalize to a **per-adapter capability**: every assistant ("ACP") adapter can
  report health and resolve itself. Claude Code is the first concrete adapter.
- An **hourly watchdog** auto-resolves config-side failures, with a **circuit breaker**
  so a failing resolver can't loop forever, and **system notifications** so the user
  learns about both successful auto-repair and give-up states.

## Non-goals

- Fixing issue #31 (`activity.sessions` empty). The watchdog deliberately does **not**
  depend on the sessions table for its "active use" signal.
- A new dashboard UI. The app integration is just calling the new endpoint; rendering
  is out of scope here (follow-up).
- Per-adapter health for assistants other than Claude beyond the trait default
  (MCP-file adapters get a single "configured?" check for free).

## Architecture

Three layers, all owned by the daemon (`crates/senseid`).

### Layer 1 — Adapter capability (the `Assistant` trait)

New module `crates/senseid/src/assistants/health.rs` defines the vocabulary:

```rust
pub enum CheckStatus { Ok, Warn, Fail, Unknown }

pub struct AdapterCheck   { pub id: String, pub label: String, pub status: CheckStatus, pub detail: Option<String> }
pub struct AdapterHealth  { pub adapter_id: String, pub family: String, pub status: CheckStatus, pub checks: Vec<AdapterCheck>, pub resolvable: bool }
pub struct AdapterResolveReport { pub adapter_id: String, pub ok: bool, pub actions: Vec<String>, pub errors: Vec<String> }
```

`CheckStatus` serializes lowercase (matches the existing health-type convention).
`AdapterHealth.status` = worst of its checks (`Fail > Warn > Unknown > Ok`).

Two new methods on the `Assistant` trait:

- `fn config_health(&self) -> Vec<AdapterCheck>` — **pure filesystem probes only**, no
  DB, no network, paths injectable for tests. Default impl = one check derived from the
  existing `is_configured()` (Ok when configured, Fail otherwise).
- `fn resolve(&self) -> AdapterResolveReport` — default impl wraps the existing
  `configure()` (already the clean → marketplace-add → install → verify reinstall flow)
  and maps its `ConfigureResult` into the report.

Adapters never touch `PgStore` — keeping them pure is what makes them unit-testable
(`ClaudeCodeAssistant::configure/remove` already note they can't be mocked because they
shell out; the *health* probes are pure JSON reads precisely to avoid that).

### Layer 2 — Claude adapter override

`ClaudeCodeAssistant::config_health()` returns four checks, each reading a JSON file
(fast + testable — **not** shelling out to the `claude` CLI):

| check id | source | Fail when |
|---|---|---|
| `marketplace` | `~/.claude/settings.json` → `extraKnownMarketplaces["sensei-marketplace"]` (and/or `~/.claude/plugins/known_marketplaces.json`) | absent |
| `plugin` | reuse `verify_plugin_installed(installed_plugins.json, "sensei")` | not recorded |
| `enabled` | `~/.claude/settings.json` → `enabledPlugins["sensei@sensei-marketplace"] == true` | missing/false |
| `hooks` | the installed-plugin `installPath` exists **and** its `hooks/hooks.json` declares `PreToolUse` + `PostToolUse` forwarders | installPath missing / hooks file absent or unparseable / required events not declared ("hooks errors") |

Each probe is a free function taking the relevant path(s) so it can be unit-tested
against tempdir fixtures, mirroring `verify_plugin_installed`'s existing tests.

`ClaudeCodeAssistant::resolve()` = the default (calls `configure()`), reinstalling the
marketplace + plugin and reporting any issues — exactly the agreed behavior.

### Layer 3 — daemon orchestration (has the DB)

The **events-flowing** check needs `activity.hook_events`, which adapters can't reach.
The daemon computes it and merges it into the Claude adapter's `AdapterHealth`:

- New `pg_store::latest_hook_event_ts(family: &str) -> Result<Option<i64>, String>` —
  `SELECT max(ts) FROM activity.hook_events WHERE assistant_family = $1`. `hook_events`
  already carries `assistant_family`, so freshness is per-family.
- Pure helper `capture_freshness(last_ts, now_ms, window_hours, exclude_weekends) -> AdapterCheck`:
  - `None` (zero events ever) → **Warn**, `"never captured"` (distinct from stale — a
    fresh/never-wired install must not masquerade as a stale failure).
  - newest within window → **Ok**.
  - newest older than the window measured in **business time** (Saturday/Sunday do not
    count toward elapsed) → **Fail**, `"stale: last event Nh ago"`.
- `business_elapsed_hours(from_ms, to_ms, exclude_weekends) -> f64` — pure, the testable
  core of the weekend exclusion.
- Orchestrator `assistants::health_report(pg, cfg) -> Vec<AdapterHealth>` = every
  configured adapter's `config_health()`, plus (for the `claude` family) the appended
  freshness check.

#### "Active use" / window rationale (from brainstorming)

A developer works ~daily; in a normal 9–5 at least one tool event fires per hour, with
occasional 2–3h lulls. If **not a single event** lands in a working day, capture is
broken. So: default window **24h**, **weekends excluded**, user can tighten. The act of
a human running `doctor` or the app polling is itself the "active use" signal — no
dependency on the (broken) sessions table.

### Endpoints

So the CLI and the app are thin consumers of one daemon-owned check:

- `GET /api/assistants/health` → `{ status, adapters: Vec<AdapterHealth> }`.
- `POST /api/assistants/resolve` `{ "adapter_id": "claude-code" }` → `AdapterResolveReport`.
  Also clears any circuit-breaker suspension for that adapter (explicit manual retry).

Registered next to the existing `/api/assistants/{detect,families,configure,remove}`.

### Hourly watchdog + circuit breaker

A `tokio::spawn` loop in `build_full_app`, mirroring the existing log-retention and
federation loops (sleep-based, 1h interval). Per tick, per **configured** adapter that
is not Suspended:

1. Compute `AdapterHealth`.
2. **Auto-resolve only when a config-side check (`marketplace`/`plugin`/`enabled`/`hooks`)
   is `Fail`.** A pure `events`-stale-but-config-OK state is logged + notified as a
   *warning* but **not** auto-reinstalled (reinstalling a correctly-wired plugin is churn
   that won't restore a session that merely needs a restart).
3. If a resolver ran: re-check.
   - now healthy → notify **"capture issue auto-resolved"** (info), stay `Active`.
   - still failing → notify **"capture broken — auto-repair failed, manual action needed"**
     (critical), transition the adapter to **`Suspended { reason, since }`** so subsequent
     ticks skip it (no infinite hourly reinstall).

Circuit-breaker state is **in-memory** (an `Arc<Mutex<HashMap<adapter_id, WatchdogState>>>`
threaded into the loop and the resolve handler). It resets on:
- daemon restart (a natural "try again"), or
- an explicit `POST /api/assistants/resolve` (or `sensei doctor --fix`).

This is why notifications are load-bearing: once Suspended, the daemon goes quiet, so the
notification is the only signal the user gets.

### Notifications

New thin module `crates/senseid/src/notify.rs`:

```rust
pub trait Notifier: Send + Sync {
    fn notify(&self, level: NotifyLevel, title: &str, body: &str);
}
```

- Production impl backed by **`notify-rust`** (cross-platform: macOS / Linux dbus /
  Windows toast) — a new daemon dependency, chosen over hand-rolling `osascript` to stay
  cross-platform and reuse a well-tested library.
- `NoopNotifier` for tests so unit tests never fire real OS notifications.
- The watchdog holds `Arc<dyn Notifier>`, injected in `build_full_app`.

### DB logging

Reuse the existing structured logger (`sensei_logger::Logger` → `public.logs`, already
wired in `build_full_app`). The watchdog logs: each check verdict, every resolution
attempt, and its outcome (resolved / gave-up-and-suspended). **No new DDL** — current
state is computed live by `GET /api/assistants/health`; the breaker state is in-memory.
(A dedicated `activity.adapter_health` history table is a possible follow-up if the app
later wants queryable history; explicitly out of scope here.)

### CLI `sensei doctor`

`crates/cli/src/doctor.rs` keeps its existing bootstrap-deps section, then adds a
**Capture / Assistants** section:

- `GET http://127.0.0.1:<port>/api/assistants/health`, render a per-adapter, per-check
  coloured table using the existing `green/red/yellow/dim` helpers.
- Daemon unreachable → render that as its own finding ("daemon down — cannot verify
  capture"), not a crash.
- `sensei doctor --fix` → `POST /api/assistants/resolve` for each failing+resolvable
  adapter, then re-fetch and re-render.

## Error handling

- Every probe degrades to `Unknown`/`Fail` with a `detail`, never panics — a corrupt
  `settings.json` reports `"enabled: settings.json unparseable"`, it does not abort the
  sweep.
- The hourly loop wraps each adapter independently; one adapter's failure can't kill the
  loop or starve the others.
- Notifications and DB log writes are best-effort (logged-and-ignored on failure); they
  must never abort the watchdog.

## Testing (TDD)

Pure functions are the bulk of the coverage:

- `capture_freshness`: none → Warn; within window → Ok; beyond window → Fail; boundary.
- `business_elapsed_hours`: Fri-4pm → Mon-10am excludes the weekend (≈18h, under 24h →
  Ok); same span with `exclude_weekends=false` (≈66h → Fail); intra-week spans.
- Claude probes (`marketplace`/`plugin`/`enabled`/`hooks`) against tempdir JSON fixtures:
  present/absent/false/malformed, mirroring the existing `verify_plugin_installed` tests.
- `AdapterHealth.status` = worst-of-checks aggregation.
- Watchdog policy (with a fake `PgStore`-free health fn + `NoopNotifier` + a stub
  resolver): config-Fail → resolves; resolve-then-pass → notifies + stays Active;
  resolve-then-still-fail → notifies + Suspends; Suspended → skipped next tick; pure
  events-stale → notify-only, never resolves.
- Endpoint smoke tests via the existing `test_app()` router harness.

## Out of scope / follow-ups

- App UI surface for the new endpoint.
- `activity.adapter_health` history table (if queryable history is wanted later).
- Linux/Windows notification verification on real hardware (the crate is cross-platform;
  the user's daemon runs on macOS).

## Relationship to the silent-error audit

`sessions.rs:173` `let _ = state.pg.insert_hook_event(...)` swallows capture-insert
failures and returns 200 — the exact bug that lets capture die invisibly. Fixing it
(log on failure, still return 200 so hooks never block) is the first item of the separate
codebase-wide silent-error audit (backlog §2 follow-up), tracked and executed after this
feature ships.
