# Tasks: fix-relay-crew-close-dead-lanes (#398)

One slice. Red commit before each green commit. DB tests run serial (`--test-threads=1`).

## 1. Store: liveness by lease (red, then green)
- [ ] 1.1 Red: store tests beside `close_crew_stamps_completed_at` (`crates/senseid/src/db/pg_store/relay.rs:14130`):
  - `close_crew_passes_over_dead_lease_lanes` — two contributors with leases back-dated past `expires_at`; close
    returns `closed`, `released` holds both lanes, both have `unregistered_at` set and their lane leases `released`.
  - `close_crew_refuses_a_live_lease_lane` — one live, one dead contributor; `Conflict("crew has live lanes")`, and
    the dead lane is still registered (refusal writes nothing).
  - `close_crew_interrupts_a_dead_lanes_task` — dead lane holding a `working` task; after close the task is
    `interrupted` with an `interrupted` action for that lane.
  Fixture note: `fx_two_crews` (`relay.rs:8604`) grants every lane a lease ending at `t0() + 300s`
  (2026-11-06T12:05Z), which is already past by the wall clock that close reads. A "live" lease in these tests must
  be set relative to `current_timestamp` (e.g. `update relay.leases set expires_at = now() + interval '5 minutes'`);
  a "dead" one can keep the fixture time or be back-dated explicitly. `fx_two_crews` has one contributor (A2); the
  two-contributor test adds a second with `fx_lane` + `fx_lease`.
  Commit red (`test(relay): close passes over dead-lease lanes — red (#398)`).
- [ ] 1.2 Green: extract run start's crew-wide live-lane-lease query (`relay_start_run_tx`, `relay.rs:4599-4605`) into
  one helper taking the instant and an optional lane to exclude; run start and `relay_close_crew` both call it (close
  passes `current_timestamp`, excludes the caller). The per-lane copies (`relay.rs:1019`, `2505`, `3776`, `5765`) stay
  as they are. Today `relay_close_crew` reads the crew row without a lock; it now locks it `for update`, refuses on a
  live other lane,
  else locks the other registered lanes `order by id for update` and runs `relay_unregister_tx` for each.
  `RelayCloseOutput` gains `released: Vec<(Uuid, Option<Uuid>)>`.

## 2. Handler: answer and events (red, then green)
- [ ] 2.1 Red: handler tests beside `close_with_live_lane_is_409` (`crates/senseid/src/api/handlers/relay.rs:6306`):
  - `close_with_dead_lane_closes` — contributor lease back-dated; `200`, `released` lists it; `/relay/now` drops the
    crew; the contributor's heartbeat is `410 relay_lane_gone`, its reregister `409` "crew is closed".
  - `close_emits_lane_unregistered_for_released_lanes` — subscribe to the event channel; expect
    `lane.unregistered` for the dead lane before `run.completed`.
  Keep `close_with_live_lane_is_409` as is (it must still pass).
- [ ] 2.2 Green: `close_crew` handler emits `LaneChange::Unregistered` per released lane and `task.interrupt` per
  interrupted task (the person close's loop, `handlers/relay.rs:1762-1767` — extract one helper both call), then
  `run.completed`; answer gains `released: [lane_id…]`.

## 3. Words
- [ ] 3.1 `crates/mcp/src/lib.rs:1774-1775` `relay_close_crew` text: close ignores and releases lanes with no live
  lease. Update any MCP tool-text pin test that fails.
- [ ] 3.2 `docs/prd/modules/03-delivery/21-relay/lanes.md`: under the tool list, note the lane close counts only
  live-lease lanes and releases the rest.

## 4. Tauri e2e (red, then green)
- [ ] 4.1 Red: `app/e2e/tests/relay-crew-close.spec.ts`, following `relay-run.spec.ts` (`RelayCrew` from
  `e2e/lib/relay-crew.ts`, `psqlBin()` from `e2e/lib/psql` as in `relay-conformance.spec.ts:124`):
  close every open crew first (person route); register a coordinator and a contributor; navigate to the relay screen
  once; see both `[data-lane-row]`; back-date the contributor's lane lease in `sensei_e2e`
  (`update relay.leases set expires_at = now() - interval '1 minute' where lane_id = … and kind = 'lane'`); call the
  lane close as the coordinator; expect `200`, both lane rows gone within the stream timeout with no reload, and
  `GET /relay/now` without the crew. Before the store fix it fails on the `409`.
  Run: check `pgrep -f "instance e2e"` first; `make test-app-e2e` (or the single spec per `app/e2e/playwright.config.ts`
  once the bundle is built). After the run confirm `brew services start sensei` restored the real daemon on :7744.
- [ ] 4.2 Green: the spec passes after sections 1–2. The coordinator's own row is not released by close; it should
  leave through `run.completed`: the crew-as-run event carries no row id (see `close_crew_emits_run_completed`,
  handler `relay.rs:6784`), so `RelayState.apply` re-reads Now (`packages/state/relay/relay-state.svelte.ts:295-304`)
  and Now omits closed crews. If the row stays anyway, fix the screen's stream handling so a closed crew leaves,
  with a unit spec first.

## 5. Gate and close
- [ ] 5.1 `cargo fmt --all --check`, `cargo clippy -p senseid -- -D warnings`, `make test-fast`, the serial DB tests for
  `relay`, the app unit specs; fix DeepSource findings in touched code.
- [ ] 5.2 `openspec validate fix-relay-crew-close-dead-lanes --strict`.
- [ ] 5.3 PR to develop; the completing commit says `Closes #398`.
