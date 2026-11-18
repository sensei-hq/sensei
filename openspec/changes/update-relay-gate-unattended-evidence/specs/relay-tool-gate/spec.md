## ADDED Requirements

### Requirement: Gate reply reading
The daemon SHALL read a person's reply to a relay tool gate as approved, declined or unclear. Only a `verdict`
of `approve` (any case) or `approve: true` SHALL count as approved. A `verdict` of `deny` (any case) or
`approve: false` SHALL count as declined, and a decline SHALL win over an approval in the same reply. Every other
reply, including free text, an empty object and a non-object, SHALL count as unclear.

#### Scenario: Explicit approval
- **WHEN** the reply is `{"verdict":"approve"}` or `{"approve":true}`
- **THEN** the reply is read as approved and the tool call is allowed

#### Scenario: Explicit decline
- **WHEN** the reply is `{"verdict":"deny"}`, `{"approve":false}` or `{"verdict":"approve","approve":false}`
- **THEN** the reply is read as declined and the tool call is denied

#### Scenario: Garbled or free-text reply
- **WHEN** the reply is `{}`, `{"text":"ok"}`, `{"verdict":"maybe"}` or a bare string
- **THEN** the reply is read as unclear

### Requirement: Unclear replies fail closed for unattended hard-blocks
An unclear reply SHALL be decided the same way as no reply: in an unattended run a hard-block call SHALL be
denied with its reason; in an interactive session, and for an allow-list gate that is not a hard-block, the call
SHALL be allowed (D28).

#### Scenario: Unattended deploy with a garbled reply
- **GIVEN** an unattended run (the hook body has `unattended: true`)
- **WHEN** a `make deploy` call's gate is answered with `{"text":"sure"}`
- **THEN** `/hook/gate` answers `deny` with a reason that names the deploy and says the reply was not a clear approve or deny

#### Scenario: Interactive deploy with a garbled reply
- **GIVEN** an interactive session (no `unattended` flag)
- **WHEN** a `make deploy` call's gate is answered with `{"text":"sure"}`
- **THEN** `/hook/gate` answers `allow`

### Requirement: Gate outcomes in the run's activity
When the `/hook/gate` body carries a `run_id` that names an existing run, the daemon SHALL append the gate's
outcome to that run's `activity.run_events`: `gate_raised` when the gate is put to a phone, `gate_passed` with
`outcome: "approved"` on approval, and `flagged` with `outcome` `declined`, `denied` or `allowed` otherwise.
`detail` SHALL carry the tool, the hard-block category when there is one, the outcome, the no-answer cause when
there is one, and the reason, and SHALL NOT carry the tool input or command text. An ungated call SHALL NOT be
recorded. A failure to record SHALL be logged and SHALL NOT change the gate's answer.

#### Scenario: Unattended deploy with no dōjō enrolled is recorded as denied
- **GIVEN** a run created with `POST /api/runs` and no dōjō membership
- **WHEN** the e2e spec `app/e2e/tests/relay-gate.spec.ts` posts an unattended `make deploy` to `/hook/gate` with that `run_id`
- **THEN** the answer is `deny`, and `GET /api/runs/{id}` lists a `flagged` event with `outcome: "denied"`, `category: "deploy"` and the reason, and no command text

#### Scenario: Interactive deploy is recorded as allowed
- **WHEN** the same spec posts a `make deploy` without `unattended` and with the `run_id`
- **THEN** the answer is `allow`, and the run lists a `flagged` event with `outcome: "allowed"`

#### Scenario: Ordinary call is not recorded
- **WHEN** an unattended `cargo test` is posted with the `run_id`
- **THEN** the answer is `allow` and no gate event (`detail.via` = `"hook_gate"`) is appended to the run

#### Scenario: Bad run id does not change the answer
- **WHEN** an unattended `make deploy` is posted with a `run_id` that is not a uuid or names no run
- **THEN** the answer is the same `deny` as without a `run_id`, the response is 200, and the failed record is logged

### Requirement: The plugin names the run
The sensei plugin's gate hook SHALL send `run_id` with each `/hook/gate` call: the value of `SENSEI_RUN_ID` for a
step the daemon spawned, else the run drive's run id for a driven session. An interactive, undriven session SHALL
send no `run_id`.

#### Scenario: Daemon-spawned step
- **GIVEN** `SENSEI_RUN_ID` is set in the session's environment
- **WHEN** the gate hook calls `/hook/gate`
- **THEN** the body carries `unattended: true` and `run_id` equal to `SENSEI_RUN_ID`

#### Scenario: Driven session
- **GIVEN** the run drive is on for the session with run id R and `SENSEI_RUN_ID` is unset
- **WHEN** the gate hook calls `/hook/gate`
- **THEN** the body carries `unattended: true` and `run_id` equal to R

#### Scenario: Interactive session
- **WHEN** neither `SENSEI_RUN_ID` nor a run drive is present
- **THEN** the body carries no `run_id`
