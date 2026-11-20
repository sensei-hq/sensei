# Tasks

## 1. Slice 1 — dōjō push readiness, outcome and test push
1.1 Red: `dojo/src/routes/version/version.spec.ts` asserts `push.configured` is true with all three
    VAPID values and false when any is missing. Green: `/version` reads `vapidFromEnv()`
    (`dojo/src/lib/server/relay-push-env.ts`).
1.2 Red: SQL test `database/tests/dojo/push_subscription_outcome.sql` — columns `last_push_at`,
    `last_push_status`, `last_push_error` (check ≤600) exist; the owner reads them, another user
    does not; `authenticated` cannot update them. Green: edit
    `database/ddl/table/dojo/push_subscriptions.ddl` (+ column comments), `dbd reconcile`, policies.
1.3 Red: `relay-push-send.spec.ts` — a 201 records `last_push_status=201` and `last_push_at`; a 500
    records status and error; a throw records the error; 404/410 still disables. Green: today
    `PushSender` returns only the HTTP status (`relay-push-send.ts:201-224`), so widen it to
    `{status, error}` (the response body, truncated to 600) and write the outcome per subscription
    in `sendRelayPush` (service role), errors logged, never thrown. Reads need no policy change:
    `authenticated` has a table-wide `select` grant and a column-scoped `update` grant
    (`database/policies/dojo/push_subscriptions.sql`), so the new columns are readable, not writable.
1.4 Red: route spec for `GET /v1/you/push/status` — returns `configured` and the caller's
    subscriptions with `platform, enabled, last_push_at, last_push_status`; no `web_push` field;
    401 unauthenticated; a read failure is 500, never `[]`. Green: implement.
1.5 Red: route spec for `POST /v1/you/push/test` — 503 `push_not_configured` without VAPID; 409
    `no_subscription` with none enabled; otherwise sends the fixed test payload ignoring prefs and
    quiet hours and returns `{attempted, delivered, disabled}`. Green: add a `test` signal to
    `buildRelayPushPayload`/`sendRelayPush` (bypass `shouldPush` only for it) and the route.
1.6 Red: `relay-notify.spec.svelte.ts` — `on` view offers "Send a test" and shows the last delivery;
    a `no-vapid-key` dōjō renders the status "Notifications aren't set up on this dōjō" with no
    button; a failed test renders a failure Notice with its remedy. Green: state + component.
1.7 `release.yml` deploy-worker: add a step after "The live dōjō serves this tag" (that step
    `exit 0`s on a match, so the check cannot be appended inside it) that emits `::warning::` when
    `push.configured` is false.
1.8 Verify on the LOCAL dōjō with Playwright: subscribe with a stub push endpoint
    (`127.0.0.1:<port>` recording requests), click Send a test, see "Delivered just now"; the stub
    received exactly one request with an encrypted body; `/version` shows `configured: true`; with
    VAPID removed from `.dev.vars` the inbox shows the not-set-up status. Record the run in the PR.
1.9 `bun run test` (dojo), `bun run check`, `database/tests/run.sh dojo` green; PR to develop.

## 2. Slice 2 — `sensei dojo evidence`
2.1 Red: unit tests in `crates/cli/src/dojo_evidence.rs` over a pure `assess(auth_status,
    memberships, sync_state, runs, version, host, since) -> Evidence`: fails when the daemon's sync
    target (`auth_status.dojo`) is not the host; picks connections whose `dojo_url` host equals
    `--host` and joins sync rows on their `persona_slot`; counts rows in state `synced` with
    `synced_at` within `since` for `dojo_sync_plan`, `repository`, `repository_metric`,
    `repository_graph` (a `synced` row older than `since` does not count); counts runs with
    `dojo_session_id` created within `since` whose `workspace_id` is in a host connection's
    `bound_workspaces`, or that are unbound while every enabled connection is on the host; marks
    each leg pass/fail; a missing leg names itself.
2.2 Red: render test — markdown block has counts only, no repository keys, no emails; with
    `show_keys` it lists keys. Green: implement `assess` + `render`.
2.3 Red: integration test against a stub daemon + stub dōjō (axum test servers): exit 0 when all
    legs pass, 1 with the failing leg named; a daemon read error is an error exit, never "0 rows".
    Green: wire `sensei dojo evidence [--host] [--since] [--show-keys]` in `crates/cli/src/main.rs`
    (the daemon address comes from the existing global `--daemon-url` / `SENSEI_DAEMON_URL`).
2.4 Live: run against the local daemon enrolled to the local dōjō (`--host 127.0.0.1:5173`) and
    confirm counts match `GET /api/dojo/sync-state`.
2.5 `cargo fmt --all --check`, `cargo clippy -p sensei-cli -- -D warnings`, `cargo test -p sensei-cli`
    green; docs: `docs/spec/pipeline/dojo-lifecycle.md` (the daemon-side dōjō doc; this repo has no
    `docs/guide/`) describes the command. PR to develop.

## 3. Close-out (local)
3.1 Comment on #314 with what shipped and link the runbook below. Leave it open.

## 4. Runbook — production (the user's steps; nothing here may be run by an agent)
1. Release a version carrying slices 1–2 (`make bump`, PR to main, `make release`, approve
   `dojo-prod`). The deploy-dojo job adds the `push_subscriptions` columns.
2. Set VAPID secrets on the dōjō Worker (`docs/history/plan/relay-p4-prod-runbook.md` step 2).
   Check: `curl -s https://dojo.sensei-hq.com/version` shows `"push":{"configured":true}`.
3. Install the release; `sensei auth approve` (keychain, until #282); in the app, Settings → Dōjō →
   Connection, sign in to dojo.sensei-hq.com.
4. Wait one sync cycle; start a run (`start_run`) in `sensei` and let it publish.
5. On the phone, open dojo.sensei-hq.com/you, turn on Notify me, press Send a test; confirm the
   notification arrives. Then raise a decision from a run and confirm it also arrives.
6. Run `sensei dojo evidence` and paste its block, plus "push arrived on phone at <time>", on #314,
   then close it (also update #184/#299/#301 comments if their production legs are met).
