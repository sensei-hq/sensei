## ADDED Requirements

### Requirement: Decision response schema
A relay decision SHALL carry either a non-empty `options` list (option-pick) or a `response` schema (form), and never both or neither. A `response` SHALL be a list of 1 to 12 fields, each with a unique `id` matching `^[a-z][a-z0-9_]{0,39}$`, a `label`, a `kind` of `text`, `textarea`, `number`, `boolean` or `select`, an optional `required` flag, `options` when and only when the kind is `select`, an optional `default`, and optional `help`. The daemon SHALL refuse a malformed schema with 422 before any write.

#### Scenario: A lane asks with a schema
- **WHEN** a lane posts `POST /api/relay/decisions` with `asked_of: "person"` and a `response` of three valid fields and no options
- **THEN** the daemon stores `asked.props.response` and `GET /relay/now` lists the decision with that `response` and empty `options`

#### Scenario: Options and response together are refused
- **WHEN** a lane asks with both `options` and `response`
- **THEN** the daemon answers 422 and no decision or task block is recorded

#### Scenario: A schema-less decision is unchanged
- **WHEN** a lane asks with `options` and `recommended` and no `response`
- **THEN** the decision behaves exactly as before: Now lists it with `response: null`, and the option-pick journey in `app/e2e/tests/relay-run.spec.ts` passes

### Requirement: Structured answer in one validated write
Answering a form decision SHALL carry `values` (an object keyed by field id) and no `option`; answering an option decision SHALL carry `option` and no `values`. The daemon SHALL validate every value against the stored schema before writing: unknown keys, missing or empty required fields, a value of the wrong kind, a `select` value that is not one of its options, and text over 600 characters or tripping the leak guard SHALL each be refused with 422 naming the field. A refused answer SHALL record nothing and leave the decision open. An absent optional field SHALL stay absent; the daemon SHALL NOT fill it from the field's `default`.

#### Scenario: The person answers with valid values
- **WHEN** the person posts `POST /relay/answers` with `decision_id`, `values` satisfying the schema, and `surface: "desktop"`
- **THEN** the daemon records one `answered` action with `props.values`, resumes the tasks the ask blocked, and emits the decision `remove` event

#### Scenario: A required field is missing
- **WHEN** the answer's `values` omit a required field
- **THEN** the daemon answers 422 with a message naming that field, records no `answered` action, and the decision stays on Now

#### Scenario: The lane reads the values back
- **WHEN** the asking lane reads its envelope after the person answered a form decision
- **THEN** the envelope's answer carries `values` equal to what the person sent and `option: null`

### Requirement: Form decisions render as a validated form
The Relay screen SHALL render a decision that carries a `response` as a form drawn with `@rokkit/forms` from the schema, one control per field, inside the existing decision card, in both Needs you and the task view. It SHALL validate inline before sending: a missing required field SHALL show its message and send nothing. Answer SHALL send the values once (a second press while the write is in flight sends nothing) and SHALL show the daemon's refusal in place without clearing the entered values. A decision with `options` SHALL render as option buttons, as today.

#### Scenario: Required field refused inline
- **WHEN** the person presses Answer on a form decision with a required field empty
- **THEN** the field shows its validation message, no answer is posted, and the card stays (proved by `app/e2e/tests/relay-decision-form.spec.ts` "required fields refuse inline")

#### Scenario: A daemon refusal is shown in place
- **WHEN** the daemon refuses the answer (for example the decision was already answered on the dōjō)
- **THEN** the card shows the refusal reason via its error notice and keeps the entered values

### Requirement: Form decisions are live with no refresh
A form decision SHALL appear on the open Relay screen when it is asked, and SHALL leave it when it is answered, without navigation or reload, through the existing relay stream decision `add` / `remove` events whose row carries `response`.

#### Scenario: Arrival with no refresh
- **WHEN** a lane asks a form decision of the person while the Relay screen is open
- **THEN** the card and its fields appear in Needs you without navigation (proved by `relay-decision-form.spec.ts` "a form decision arrives with no refresh")

#### Scenario: Resolution with no refresh
- **WHEN** the person fills the form and presses Answer
- **THEN** the card leaves Needs you without navigation and the lane envelope reads the exact values (proved by `relay-decision-form.spec.ts` "answering resolves with no refresh")

#### Scenario: Answered from the task view
- **WHEN** a task-bound form decision is answered from the task view
- **THEN** the card leaves the task view without navigation (proved by `relay-decision-form.spec.ts` "the task view answers too")

### Requirement: Both surfaces share the answer path
The decision answer in `@sensei/state` SHALL take either `{option}` or `{values}` and pass it unchanged through both the daemon SSE source and the dōjō realtime source, so the dōjō answers form decisions through the same state updater once its transport lands.

#### Scenario: The realtime seam forwards values
- **WHEN** `RelayState.answer` is called with `{values}` over `relayRealtimeSource(cannedRealtime(...))`
- **THEN** the canned transport records one `answer` write whose body carries those `values` and no `option`

#### Scenario: The task view's source forwards values
- **WHEN** `RelayTaskState.answer` is called with `{values}` over `relayRealtimeTaskSource(cannedRealtime(...))`
- **THEN** the canned transport records one `answer` write whose body carries those `values` and no `option`
