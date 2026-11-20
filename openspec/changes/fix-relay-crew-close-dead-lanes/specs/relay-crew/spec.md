## ADDED Requirements

### Requirement: Lane close counts only live-lease lanes
`POST /api/relay/crews/{id}/close` SHALL refuse with `409 crew has live lanes` only when a lane in the crew other than
the caller holds a live lane lease (a `granted` lane lease whose `expires_at` is after now). A registered lane with no
live lease SHALL NOT block close. The route SHALL stay coordinator-only.

#### Scenario: Dead contributors do not block close
- **GIVEN** a crew with a coordinator lane and two contributor lanes, both contributors registered, and both
  contributors' lane leases past `expires_at`
- **WHEN** the coordinator calls `POST /api/relay/crews/{id}/close`
- **THEN** the answer is `200` with `status: "closed"` and `released` naming both contributor lanes
- **AND** the crew row has `status = 'closed'` and `completed_at` set

#### Scenario: A live contributor still blocks close
- **GIVEN** a crew with a coordinator lane and one contributor lane whose lease is live
- **WHEN** the coordinator calls close
- **THEN** the answer is `409` with error `relay_conflict` and the crew stays `active`

#### Scenario: One live, one dead
- **GIVEN** a crew with a coordinator, one contributor with a live lease and one with an expired lease
- **WHEN** the coordinator calls close
- **THEN** the answer is `409`, and neither contributor is unregistered (the refusal writes nothing)

### Requirement: Close releases the dead lanes it passes over
When the lane close succeeds, every other lane in the crew still registered SHALL be unregistered in the same
transaction through the shared unregister path: `unregistered_at` stamped, its lane lease released, any task it held in
`assigned`, `working`, `rework` or `blocked` moved to `interrupted` with an `interrupted` action, and its resource
grants released to their next waiters. A dead session that calls again SHALL get `410 relay_lane_gone`, and its
reregister SHALL get `409 crew is closed`.

#### Scenario: A revived session finds the crew closed
- **GIVEN** a contributor lane released by the coordinator's close
- **WHEN** that lane heartbeats
- **THEN** the answer is `410` with error `relay_lane_gone`
- **AND WHEN** it reregisters, the answer is `409` naming `crew is closed`

#### Scenario: A held task is interrupted, not lost
- **GIVEN** a dead-lease contributor that still holds a `working` task (the sweeper has not yet run)
- **WHEN** the coordinator closes the crew
- **THEN** the task is `interrupted` with an `interrupted` action naming that lane

#### Scenario: A closed crew's interrupted task is never re-offered
- **GIVEN** a task the coordinator's close interrupted
- **WHEN** the lease sweep runs past the re-offer grace
- **THEN** the task stays `interrupted` with no `reoffered` action, since no lane can claim in a closed crew
- **AND** the sweep sends the closed crew no review nudge

### Requirement: The screen drops a crew closed by its lanes without a reload
The lane close SHALL emit `lane.unregistered` for each lane it released, `task.interrupt` for each task it interrupted,
and `run.completed` for the crew. The relay screen SHALL remove the closed crew's lane rows from the stream with no
reload, and `GET /relay/now` SHALL no longer list the crew.

#### Scenario: Closing an inert crew from the coordinator lane (Tauri e2e)
- **GIVEN** the app is on the relay screen showing a crew with a coordinator lane and a contributor lane
  (`app/e2e/tests/relay-crew-close.spec.ts`)
- **AND** the contributor's lane lease is back-dated past `expires_at` in `sensei_e2e`
- **WHEN** the coordinator calls `POST /api/relay/crews/{id}/close` over the lane API
- **THEN** the answer is `200`, and the crew's lane rows (`[data-lane-row]` for both codes) leave the screen with no
  reload and no navigation
- **AND** `GET /relay/now` does not list the crew
