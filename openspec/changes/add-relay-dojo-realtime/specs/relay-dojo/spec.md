## ADDED Requirements

### Requirement: Relay rows in the dōjō are the person's alone
The dōjō SHALL hold published relay rows in `dojo.relay_rows`, keyed by `membership_id`, readable by `authenticated`
only where `dojo.owns_membership(membership_id)` holds, never readable by `anon`, and writable only through the Worker's
relay queue routes. Removal SHALL be an UPDATE that sets `removed = true`, so RLS filters every change a subscriber can
receive.

#### Scenario: The owner reads their rows
- **GIVEN** the Worker stored a `lane` row for membership M
- **WHEN** the person signed in as M selects `dojo.relay_rows`
- **THEN** the row is returned (proved by `database/tests/dojo/relay_rows_access.sql`)

#### Scenario: Another member reads nothing
- **GIVEN** the same row
- **WHEN** a different member of the same tenant, or `anon`, selects `dojo.relay_rows`
- **THEN** no row is returned, and an insert naming membership M by that member is refused

#### Scenario: Rows reach realtime subscribers under RLS
- **GIVEN** the dōjō database is deployed with `scripts/deploy-dojo-db.sh`
- **THEN** `dojo.relay_rows` is a member of `supabase_realtime`
- **AND** a supabase-js channel opened with M's JWT receives an upsert for M within 5 s, while one opened by another
  member receives nothing (proved by `dojo/e2e/relay-rows.spec.ts`)

### Requirement: The daemon publishes shaped relay rows
The daemon SHALL publish each crew's relay reads to its owning memberships as rows of the stream's shape — `crew`,
`lane`, `run`, `run_task`, `task`, `decision`, `message`, `item`, `task_view`, `summary` — produced by the same code that
serves `GET /relay/now`, `GET /relay/queue` and `GET /relay/tasks/{crew}/{task}`, without writing read receipts. It SHALL
publish within 3 s of a relay event and on a 60 s sweep, mark rows it no longer holds as removed, identify the machine
by an opaque `source` (never the `relay.daemon_user` id), and record each push in `sensei.sync_state`
(`entity = relay_sync`).

#### Scenario: A registered lane appears in the dōjō
- **GIVEN** a membership signed in to the local dōjō and an open crew on the desktop
- **WHEN** a lane registers
- **THEN** a `lane` row whose `data` equals that lane's `GET /relay/now` row is in `dojo.relay_rows` within 3 s
  (proved by `app/e2e/tests/relay-dojo-sync.spec.ts`)

#### Scenario: The dōjō assembles exactly what the desktop serves
- **GIVEN** the contract fixture `packages/state/relay/fixtures/dojo-rows.contract.json` written by the Rust test
- **WHEN** `assembleNow` runs over its rows
- **THEN** the result deep-equals the fixture's `RelayNow`

#### Scenario: An unregistered lane is removed
- **WHEN** a published lane unregisters
- **THEN** its row is updated to `removed = true` and no longer appears in an assembled read

#### Scenario: A push that fails is recorded
- **WHEN** the Worker answers a publish with a 5xx
- **THEN** the `relay_sync` sync_state row is `error` with the status, and the next sweep retries

### Requirement: Nothing private leaves the machine
Before publishing, the daemon SHALL run the leak guard over every string in a row and withhold any row that fails,
recording only the finding kinds. It SHALL NOT publish `report_md` unless the crew's `share_reports` is true; an
unshared report SHALL read "Not shared from the desktop" with how to share it, never empty and never as a failure.

#### Scenario: A secret in a summary withholds the row
- **GIVEN** a task summary containing a token the shared secret scanner detects
- **WHEN** the publisher runs
- **THEN** that row is not sent, the `relay_sync` sync_state row is `partial` naming the kind only, and the token's
  value appears in no request body and no log line

#### Scenario: Report bodies stay home by default
- **GIVEN** a crew with `share_reports = false` and a reported task
- **THEN** the published `task_view` row carries no `report_md`
- **AND** the dōjō task view shows "Not shared from the desktop" where the report would be

### Requirement: The dōjō renders the same Relay screen, live
The dōjō SHALL serve the Relay screen at `/you/relay` (view id `dojo.you.relay`) and the task view at
`/you/relay/[crew_id]/[task_id]` (view id `dojo.you.relay.task`) with the same components as the desktop
(`@sensei/ui/relay`) and `@sensei/state`'s `RelayState` over a supabase-js realtime transport. The transport SHALL map
`postgres_changes` to the stream's `StateEvent`: rows of `lane`, `run`, `decision`, `message` and `item` apply in place
(`removed` → `remove`); other entities re-read. Panels SHALL be remembered through `dojo/src/lib/view-states.svelte.ts`.
The footer SHALL say when the rows were last updated from the desktop; with no rows it SHALL say nothing has synced
yet; a failed read SHALL show an error notice. Desktop-only actions (create, start or cancel a run; backlog edits)
SHALL not be offered on the dōjō.

#### Scenario: The page moves with no reload
- **GIVEN** the dōjō Relay page is open and a lane row is upserted for the signed-in membership
- **THEN** the lane appears in Now within 5 s without a navigation or reload
- **AND** when the row is marked removed, the lane leaves Now the same way (proved by `dojo/e2e/relay-screen.spec.ts`)

#### Scenario: The same components on both surfaces
- **WHEN** the relay components move to `packages/ui/relay`
- **THEN** the desktop journeys `app/e2e/tests/relay-run.spec.ts` and `app/e2e/tests/relay.spec.ts` stay green
- **AND** the dōjō page renders `LaneRow`, `DecisionRow` and `CompletedRow` from `@sensei/ui/relay`

#### Scenario: Nothing synced yet
- **GIVEN** a signed-in member with no published rows
- **THEN** the page says "Not synced from the desktop yet" and how to sign the desktop in, not an empty crew

### Requirement: Answers from the dōjō reach the daemon
A dōjō answer, message or nudge SHALL be written by the Worker as a `dojo.relay_inbox` row with `relay_crew_id`, no
`session_id`, `direction = human_to_agent`, `kind` `decision`, `chat` or `nudge`, and `payload.surface = dojo`, only
for a crew published under the caller's membership. The daemon SHALL read these rows from
`GET relay/queue/inbox?since=` with a cursor persisted in `sensei.sync_state` (`entity = relay_inbox`), apply each
through the same code as `POST /relay/answers` and `POST /relay/messages` with surface `dojo`, then mark it `answered`
(applied) or `superseded` with the refusal reason. The existing `GET relay/inbox` SHALL exclude crew rows.

#### Scenario: Answering a decision on the dōjō
- **GIVEN** a lane asked the person a decision and it shows in the dōjō's Needs you
- **WHEN** the person picks an option on the dōjō
- **THEN** the lane receives the answer through its normal wait within 15 s
- **AND** the decision leaves Needs you on the dōjō and on the desktop without a reload
  (proved by `app/e2e/tests/relay-dojo-answer.spec.ts`)

#### Scenario: A nudge from the dōjō
- **WHEN** the person nudges a waiting lane from the dōjō with a reason
- **THEN** the lane's next message fetch carries the nudge with `surface: dojo`, and the wait row's last nudge reads "you"

#### Scenario: Answered on the desktop first
- **GIVEN** the person answered the decision on the desktop
- **WHEN** a dōjō answer for it is consumed
- **THEN** the inbox row is `superseded` with the daemon's reason and the dōjō decision card shows that reason

#### Scenario: Another member's crew is refused
- **WHEN** a member posts an answer naming a crew published under someone else's membership
- **THEN** the Worker answers 404 and no inbox row is written

#### Scenario: The run inbox is unaffected
- **GIVEN** a crew inbox row exists for membership M
- **WHEN** the daemon calls `poll_inbox` on `GET relay/inbox`
- **THEN** the response contains no row without a `run_id` and decodes cleanly

### Requirement: A run can be followed end to end from the dōjō
With the fake crew driving lanes on the desktop's e2e daemon, a dōjō page SHALL show each step of the run journey —
crew registered, run started, planner decision asked and answered, plan unpacked, tasks claimed, step counters
moving, review, done, run completed — without a reload, within 10 s of each change.

#### Scenario: The journey on both surfaces
- **WHEN** `app/e2e/tests/relay-dojo-run.spec.ts` runs the #426 journey with a dōjō page open beside the desktop
- **THEN** at each step the dōjō page shows the same lane, task and run state as the desktop, with no `reload`,
  no navigation and no retry click, and its footer's updated time advances
