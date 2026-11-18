# Change: Make the dōjō plane provable in production — push readiness, a test push, and one evidence command (#314)

## Why
#314 (G-OPS-1) is done when an enrolled daemon syncs repositories, metrics and graph to
dojo.sensei-hq.com, a relay run appears there, and a push notification arrives on a phone, with
evidence recorded on the issue. The code paths exist and were verified on the **local** dōjō
(#299, #301 comments of 2026-10-21). What blocks a production check is that two of the three
legs cannot be observed without reading Worker logs or the production database:

- **Push is silent when it does not work.** `vapidFromEnv` returns null when any VAPID value is
  missing and `sendRelayPushFromEnv` only logs a warning (`dojo/src/lib/server/relay-push-env.ts:27-47`).
  Nothing says whether production has the keys (the 2026-07-18 runbook left them unset:
  `docs/history/plan/relay-p4-prod-runbook.md`). A send's outcome (`attempted`, `delivered`) is
  returned and dropped (`dojo/src/lib/server/relay-push-send.ts:302-350`); no row records it.
- **A push needs a real crash, approval, decision or stall** to fire
  (`eventForSignal`, `relay-push-send.ts:52-60`). There is no way to ask "does push reach my
  phone?" directly.
- **Sync and relay evidence is scattered.** The daemon already answers it —
  `GET /api/dojo/memberships` (each connection's `dojo_url`), `GET /api/dojo/sync-state`
  (`crates/senseid/src/api/routes.rs:611`, per-entity verdicts by persona) and `GET /api/runs`
  (`routes.rs:492`; `dojo_session_id` set by `publish_run`,
  `crates/senseid/src/tasks/handlers/publish_run.rs:138-154`) — but nothing puts the three
  together for one target dōjō, so "verified in production" is a manual SQL session. Two traps
  make a naive count wrong: `sensei.sync_state` is keyed `(entity, entity_key, direction)` with no
  host (`database/ddl/table/sensei/sync_state.ddl`), so rows synced to the local dōjō stay `synced`
  after the daemon is pointed at production; and `dojo_session_id` is stored from the *first*
  membership that acks (`publish_run.rs:138-154`), so it does not say which host holds the run.

## What Changes
- **dōjō push readiness and outcome (slice 1).**
  - `GET /version` (public) also reports `push: { configured: boolean }` — whether all three VAPID
    values are present. No key material.
  - Each send records its outcome on the subscription: `last_push_at`, `last_push_status`
    (HTTP status from the push service) and `last_push_error` (≤600 chars).
  - `GET /v1/you/push/status` returns the caller's subscriptions (platform, enabled, last outcome)
    and `configured`. Never the endpoint or keys.
  - `POST /v1/you/push/test` sends a zero-knowledge test notification to the caller's own enabled
    subscriptions, bypassing event prefs and quiet hours (the person asked), and returns
    `{attempted, delivered, disabled}`. With VAPID unset it answers 503 `push_not_configured`,
    not a silent success.
  - The inbox "Notify me" control: when on, offers **Send a test** and shows the last delivery;
    when the dōjō has no VAPID key, says so instead of a button that fails.
  - `release.yml`'s deploy-worker step warns when the live `/version` reports
    `push.configured = false`.
- **One evidence command (slice 2).** `sensei dojo evidence [--host dojo.sensei-hq.com] [--since 24h]`
  reads the local daemon and the target dōjō (read-only) and prints a markdown block for the issue:
  dōjō build and push readiness; the daemon's sync target (`GET /api/auth/status` → `dojo`, which
  must be the host — the daemon syncs to one dōjō at a time); connections that point at the host;
  per persona slot of those connections, counts of rows `synced` since `--since` for
  `dojo_sync_plan`, `repository`, `repository_metric`, `repository_graph`; runs with
  `dojo_session_id` set whose publish could only have reached the host (workspace bound to a
  connection on the host, or no other enabled connection). Counts only by default (the issue is
  public); repository keys only with `--show-keys`. Exit 0 only when every leg passes.
- **Runbook (tasks.md §4)** for the user's production steps: set VAPID secrets, sign the daemon in
  to dojo.sensei-hq.com, approve keychain, run a relay run, send a test push to the phone, paste
  the evidence.

Not here: `dojo_memberships.sync_status` never written (#184 — the evidence reads `sync_state`
instead); relay crews in the dōjō (#427 — #314's relay leg is the existing run bridge,
`publish_run`); Team-ID signing (#282 — the runbook uses `sensei auth approve`).

## Impact
- Specs: `dojo-push` (new), `dojo-production-evidence` (new).
- Code: `database/ddl/table/dojo/push_subscriptions.ddl`, `dojo/src/lib/server/relay-push-send.ts`,
  `relay-push-env.ts`, `dojo/src/routes/version/+server.ts`, new `dojo/src/routes/v1/you/push/`
  (`status`, `test`), `dojo/src/lib/relay/relay-notify.svelte.ts` + its component,
  `.github/workflows/release.yml`; `crates/cli/src/main.rs` + new `crates/cli/src/dojo_evidence.rs`.
- Production: nothing deploys from this change. The user runs the release and the runbook.
