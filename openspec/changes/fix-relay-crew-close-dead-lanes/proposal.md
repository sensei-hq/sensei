# Fix: a relay crew with dead lanes can close (#398)

## Why
`POST /api/relay/crews/{id}/close` (the coordinator's lane route) refuses with `409 crew has live lanes` while any
other lane in the crew has `unregistered_at is null`, whatever its lease says
(`crates/senseid/src/db/pg_store/relay.rs:6047-6061`). A session that dies never unregisters — nobody holds its token
any more — and no path evicts it. The sweeper expires the dead lane's lease and interrupts its task
(`relay.rs:6503-6569`) but leaves the lane row registered. So the crew can never close from the lane side.
Observed on crew `cf9d`: both contributors gone, leases expired, tasks interrupted, close still 409.

Wherever Relay judges whether a lane can act, liveness is the lane's lease, not a column
(`database/ddl/table/relay/lanes.ddl:43` comment). The same "granted lane lease past now" check guards the resource
waiter grant (`relay.rs:1019-1023`), assign (`relay.rs:2505-2509`), the lane's own claim path (`relay.rs:3776-3780`),
run start (`relay.rs:4599-4605`, joined to registered lanes of the crew) and handover (`relay.rs:5765-5768`). Now lists
registered lanes but marks each one's presence by its lease (`docs/prd/modules/03-delivery/21-relay/lanes.md:65-70`).
Close is the one gate that counts the registration flag alone.

The issue's 2026-11-12 comment fixes the direction: register defaults to contributor and sprint runs complete
themselves, so the fix is "close tolerates dead-lease lanes", not a coordinator evict.

## What is already done (not in scope)
- The person's close, `POST /relay/crews/{id}/close` (#419, `relay.rs:6082-6142`), releases every lane with
  `unregistered_at is null` — dead or alive — and closes the crew. It needs no coordinator and already tolerates dead
  lanes. It stays as is (it refuses while a planning or active run is open; the person cancels the run first).
- Archiving closed crews is #393, a separate change.

## What Changes
- The lane close counts only lanes holding a live lane lease (`kind = 'lane'`, `state = 'granted'`,
  `expires_at > current_timestamp`), besides the caller. A lane with no live lease does not block close.
- In the same transaction, close unregisters each dead-lease lane through the shared `relay_unregister_tx`
  (`relay.rs:1976`): `unregistered_at` stamped, lease released, any held task interrupted, resource grants freed.
  No lane row is left registered in a closed crew except the caller, who unregisters next as today.
- The close handler (`crates/senseid/src/api/handlers/relay.rs:1733-1749`) emits `lane.unregistered` for each lane it
  released and `task.interrupt` for each task it interrupted, as the person close does (`relay.rs` handler
  1762-1767), then the existing `run.completed`.
- A dead session that comes back finds `410 relay_lane_gone` on its next call and `409 crew is closed` on reregister —
  the contract `lanes.md:60-61` already states for a crew the person closed.
- The MCP `relay_close_crew` description (`crates/mcp/src/lib.rs:1774-1775`) and `lanes.md` say that close ignores and
  releases lanes without a live lease.
- A lane with a live lease still blocks close with `409 crew has live lanes`.

## Impact
- Capability: `relay-crew`.
- Code: `crates/senseid/src/db/pg_store/relay.rs` (`relay_close_crew`, `RelayCloseOutput`, and
  `relay_start_run_tx`, whose crew-wide live-lease query becomes the shared helper),
  `crates/senseid/src/api/handlers/relay.rs` (`close_crew`), `crates/mcp/src/lib.rs` (tool text),
  `docs/prd/modules/03-delivery/21-relay/lanes.md`.
- No DDL change. No new route. Wire shape of the close answer gains `released` (lane ids), additive.
- Tests: store tests beside `close_crew_stamps_completed_at` (`relay.rs:14130`), handler tests beside
  `close_with_live_lane_is_409` (handler `relay.rs:6306`), one Tauri e2e spec `app/e2e/tests/relay-crew-close.spec.ts`.
