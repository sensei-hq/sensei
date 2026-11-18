# Tasks: Relay in the dōjō over Supabase realtime (#427)

Each slice merges to develop on its own, green. Red commit (failing test) before the green commit, every time.
Before any e2e run: `pgrep -f "instance e2e"` (wait if live); afterwards confirm `brew services start sensei` restored
the real daemon on :7744. Local dōjō only (`scripts/deploy-dojo-db.sh`); never production.

## 1. Slice A — the dōjō mirror: table, identity, RLS, publication, Worker ingest
- [ ] 1.1 Red: `database/tests/dojo/relay_rows_access.sql` — owner reads, other member and `anon` read nothing,
      a non-owner insert is refused, a `removed` update by the owner is allowed.
- [ ] 1.2 Green: `database/ddl/table/dojo/relay_rows.ddl` (design decision 1) and
      `database/policies/dojo/relay_rows.sql` (select/insert/update on `dojo.owns_membership`, column grants).
- [ ] 1.3 Red then green: `realtime_publication.sql` adds `relay_rows`; a SQL test asserts membership after deploy.
- [ ] 1.4 Red: Worker spec `dojo/src/routes/v1/t/[origin]/[org]/relay/queue/rows/rows-route.spec.ts` — upsert by
      `(membership, source, entity, id)`, `removed` list, 400 on an unknown entity, 403 for a non-member, 413 over the
      body cap (`body-cap.ts`).
- [ ] 1.5 Green: `relay/queue/rows/+server.ts` (POST, `asOwner`).
- [ ] 1.6 Red then green: dōjō Playwright infra — `dojo/e2e/playwright.config.ts`, global setup (check local Supabase,
      `scripts/deploy-dojo-db.sh`, `CF_PAGES=1 bun run build`, `wrangler dev --local`, seed a `sensei.test` relay user's
      membership and a second member), `@playwright/test` as a `dojo` devDependency (the dōjō has none today; only
      `app/` does), `dojo/package.json` script `test:e2e`, Makefile target `test-dojo-e2e`.
- [ ] 1.7 E2E: `dojo/e2e/relay-rows.spec.ts` — POST rows as a daemon double; a supabase-js channel with the owner's
      JWT receives the change within 5 s; the second member's channel receives nothing.

## 2. Slice B — the publisher
- [ ] 2.1 DDL first: `relay.crews.share_reports boolean not null default false`; `sensei.sync_entity` gains
      `relay_sync`, `relay_inbox` with `database/apply/before/sensei_sync_entity_relay.sql` registered in
      `design.yaml`; `sensei.task_execution_kind` gains `relay_sync` with
      `database/apply/before/sensei_task_kind_relay_sync.sql` (the `sensei_task_kind_classify_turn_intent.sql`
      pattern); `dbd reconcile` (twice if dbd#121).
- [ ] 2.2 Red: DB-backed test that `relay_now_shape` writes no `delivered`/`read` actions; green: split
      `relay_now` into `relay_now_shape` + the receipt step the handler keeps (existing `now_*` handler tests stay green).
- [ ] 2.3 Red: pure `relay_sync::rows_of(now, queue, task_views, share_reports)` tests — one row per entity, ids,
      `crew_id`, `section` on `task`, no `resources`, no `report_md` when not shared.
- [ ] 2.4 Red: leak-guard test — a secret in any string leaf withholds the row and returns kinds only.
- [ ] 2.5 Green: `crates/senseid/src/tasks/handlers/relay_sync.rs` + `TaskKind::RelaySync` (its `KindInfo`, the
      `TaskKind::ALL` list and its pinned count in `tasks/mod.rs` tests); opaque `source` (decision 2); membership
      routing by factoring `resolve_run_memberships`' rule to take a `workspace_id` (it takes a `Run` today);
      removed-row diff; sync_state writes.
- [ ] 2.6 Red then green: `DojoClient::upsert_relay_rows` against an axum fake Worker (the `client.rs` test pattern);
      `dojo-protocol` `RelayQueueRows` wire type with a DDL-strings test for the entity list.
- [ ] 2.7 Red then green: trigger — a relay `StateEvent` marks the user dirty, publish after 1 s quiet; 60 s sweep.
- [ ] 2.8 Contract: the Rust test writes `packages/state/relay/fixtures/dojo-rows.contract.json`; red TS spec
      `packages/state/relay/assemble.spec.ts`; green `assembleNow` / `assembleQueue` / `assembleTaskView`.
- [ ] 2.9 E2E: Playwright project `relay-dojo` in `app/e2e/playwright.config.ts` + Makefile `test-relay-dojo-e2e`
      (local Supabase + wrangler up; e2e persona and membership seeded in `sensei_e2e`; session in a dedicated Keychain
      slot, removed at teardown — read `crates/senseid/src/api/handlers/auth.rs` `live_access_token` for the slot
      format first). `app/e2e/tests/relay-dojo-sync.spec.ts`: register lanes with the fake crew; rows appear in local
      Supabase (`app/e2e/lib/psql.ts`) within 3 s; an unregistered lane goes `removed`; no `report_md`.

## 3. Slice C — the dōjō screen on shared components
- [ ] 3.1 Red: a dōjō component spec importing `LaneRow` from `@sensei/ui/relay` fails to resolve.
- [ ] 3.2 Green: move the relay view components and their specs/fixtures from `app/src/lib/relay` to
      `packages/ui/relay`; replace every app-only import per design decision 12 (`$lib/relay-api.js`, `$lib/api.js`,
      `$lib/glyphs`, `$lib/glyph-vocabulary`, `$lib/open-external.js`; `catalogue.ts` and `lane-view-store.ts` stay in
      the app); aliases in both
      `svelte.config.js` (the dōjō's `@sensei/state` alias is already there from #414). App unit specs and `relay-run.spec.ts`, `relay.spec.ts`
      stay green (DRY: delete the app copies, no re-export shims).
- [ ] 3.3 Red: `packages/state/relay/sources/relay-rows.spec.ts` — `fromRelayRow` maps a `dojo.relay_rows`
      `postgres_changes` payload to a `StateEvent` (decision 4), `removed` → `remove`; `realtimeChannel` with
      `map: fromRelayRow` forwards those events (SUBSCRIBED re-read and channel errors are #414's, already covered).
- [ ] 3.4 Green: `packages/state/relay/sources/supabase.ts` — the supabase-js transport (reads `dojo.relay_rows` +
      `assemble*`, channel on `dojo.relay_rows` filtered by membership, writes to the Worker routes), built on #414's
      `realtimeChannel` with `map: fromRelayRow`; its `readTask` is `assembleTaskView`, so the task route uses
      `relayRealtimeTaskSource(transport)` with `{surface: 'dojo'}` (design decision 5; no `RelayApi` adapter).
- [ ] 3.5 Red then green: `RelayState` hides desktop-only actions when `surface === 'dojo'`; selectors spec.
- [ ] 3.6 Green: routes `dojo/src/routes/(app)/you/(inbox)/relay/+page.svelte|ts` and `[crew_id]/[task_id]`; view
      ids `dojo.you.relay`, `dojo.you.relay.task` through `view-states.svelte.ts`; footer freshness; "Not synced from
      the desktop yet"; error notice on a failed read; inbox nav entry.
- [ ] 3.7 E2E: `dojo/e2e/relay-screen.spec.ts` — seeded rows render Now, Queue, Crew and a task view; an upsert
      and a removal move the page with no reload; light/dark × wide/phone screenshots; an unshared report reads
      "Not shared from the desktop".

## 4. Slice D — answers back
- [ ] 4.1 Red: `database/tests/dojo/relay_inbox_crew_rows.sql` — a crew row needs `relay_crew_id` and no
      `session_id`; insert only for a crew published under the caller's membership.
- [ ] 4.2 Green: `relay_inbox.ddl` (nullable `session_id`, `relay_crew_id`, check); in
      `database/policies/dojo/relay_inbox.sql` an insert policy `relay_inbox_queue_own`, `relay_crew_id` added to the
      `authenticated` insert column grant, and an update policy for the consumed route (own, `human_to_agent`,
      crew row, `pending`) — the existing `relay_inbox_reply_own` allows only `agent_to_human` rows.
- [ ] 4.3 Red: Worker specs — `relay/queue/answers` and `relay/queue/messages` write the row (404 for a foreign
      crew); `relay/queue/inbox?since=` returns crew rows only; `relay/queue/inbox/[id]/consumed` sets `answered` or
      `superseded`; existing `relay/inbox` GET excludes crew rows.
- [ ] 4.4 Green: those routes; the supabase transport's `answer` / `send` call them.
- [ ] 4.5 Red: daemon tests — the consumer applies a decision once through the `person_answer` core with surface
      `dojo`; a nudge through the `person_message` core; a refusal is marked `superseded` with its reason; the cursor
      persists in sync_state; rows for another machine's crews are skipped; `poll_inbox` still decodes.
- [ ] 4.6 Green: extract the cores of `person_answer` / `person_message` (events still emitted), the consumer in
      `relay_sync`, `DojoClient` queue inbox calls; 5 s poll while a crew is open and a membership is signed in.
- [ ] 4.7 Red then green: dōjō decision card shows a superseded row's reason (relay_inbox subscription).
- [ ] 4.8 E2E: `app/e2e/tests/relay-dojo-answer.spec.ts` (project `relay-dojo`) — the fake crew asks; a chromium page
      on the local dōjō shows it; the test answers there; the driver's wait receives it; Needs you clears on both
      surfaces with no reload; a dōjō nudge reaches the lane; an answer already given on the desktop shows its reason.

## 5. Slice E — the run journey watched from the dōjō
- [ ] 5.1 E2E: `app/e2e/tests/relay-dojo-run.spec.ts` — the #426 journey with a dōjō page open; at each step assert
      the dōjō shows the desktop's lane/task/run state within 10 s, no reload, footer time advancing.
- [ ] 5.2 Docs: `docs/features/06-relay.md` and `docs/journeys/relay.md` (this repo has no `docs/guide` or
      `docs/llms`) on the dōjō Relay screen and `share_reports`.
- [ ] 5.3 `openspec validate add-relay-dojo-realtime --strict`; close #427 with the e2e evidence.

## 6. Runbook for the user (production, not done by the workflow)
- [ ] 6.1 Deploy the dōjō DDL to production through the release's `dojo-prod` job (approved by a person).
- [ ] 6.2 Deploy the Worker; sign the desktop in to production; open `/you/relay` on a phone; run a crew; check the
      page moves with no refresh and an answer from the phone reaches the lane. Record the evidence on #314.
