## ADDED Requirements

### Requirement: Decision response schema
A relay decision SHALL carry either a non-empty `options` list (option-pick) or a `response` (form), and never both or neither. A `response` SHALL be `{schema, layout?}` where `schema` is a flat JSON Schema subset: one `type: "object"` with 1 to 12 `properties`, each named by `^[a-z][a-z0-9_]{0,39}$` and of type `string`, `number`, `integer`, `boolean` or `array`; an optional `required` list naming those properties; `enum` (pick one) on a string; `minimum` and `maximum` on a number or integer; `maxLength` (at most 600) and `pattern` on a string; `x-multiline: true` on a string without an `enum` (long text, drawn as a textarea, whose `maxLength` may go to 4000); on an `array` (pick several, drawn as a MultiSelect) `items` of `{type?: "string", enum}` and an optional `uniqueItems: true`; and the annotations `title`, `description` and `default`. `layout` SHALL be an optional rokkit vertical layout whose elements carry only `scope` (naming a schema property), `label`, `description` and `props.placeholder`, and which names every property exactly once (rokkit draws only the layout's elements). Every property SHALL be answerable: each `enum` member (a pick-several's `items.enum` included) SHALL be non-empty and pass the property's own `maxLength`, `pattern` and leak guard, and an integer's `minimum` and `maximum` SHALL hold a whole number between them. The daemon SHALL refuse with 422, before any write, a `response` that uses anything outside this subset, including an array without `items.enum`, `minItems` / `maxItems`, `uniqueItems: false`, `items` off an array, `x-multiline` other than `true` or beside an `enum`, `format`, nested objects, schema composition, a layout `renderer` or `override`, and a `pattern` that does not compile, uses `(?`, or uses a construct the browser's `RegExp` (with the `u` flag) reads differently (`\w`, `\d`, `\s`, `\b`, `\A`, `\z`, hex escapes, POSIX or nested classes, class set operations); the message SHALL name the property and the keyword. There SHALL be no sensei-specific field format: the schema the lane sends is the schema `@rokkit/forms` draws. A `response` sent without a `layout` SHALL be stored with one the daemon adds before the write: a vertical layout of `scope`-only elements listing the properties in the order the lane wrote them, since jsonb stores object keys in its own order and an array keeps the lane's.

#### Scenario: A lane asks with a schema
- **WHEN** a lane posts `POST /api/relay/decisions` with `asked_of: "person"`, no options, and a `response` whose schema has a required string with `enum`, an integer with `minimum` and `maximum`, and a boolean
- **THEN** the daemon stores `asked.props.response` (the schema as sent, with a layout listing the properties in the order the lane wrote them) and `GET /relay/now` lists the decision with that `response` and empty `options`

#### Scenario: A lane asks to pick several and for long text
- **WHEN** a lane asks with a property of `type: "array"`, `items: {type: "string", enum: [...]}` and `uniqueItems: true`, and a string with `x-multiline: true` and `maxLength: 2000`
- **THEN** the daemon stores the response and the Relay screen draws a MultiSelect and a textarea for them

#### Scenario: A pick-several outside the subset is refused
- **WHEN** a lane asks with an array without `items.enum`, or with `minItems`, or `x-multiline` beside an `enum`, or a long text `maxLength` over 4000
- **THEN** the daemon answers 422 naming that property and keyword, and no decision or task block is recorded

#### Scenario: A layout cannot choose a control
- **WHEN** a lane asks with a layout element carrying `renderer: "textarea"`, or a schema property carrying `format: "textarea"`
- **THEN** the daemon answers 422 naming the keyword, and no decision is recorded

#### Scenario: A form no answer can satisfy is refused
- **WHEN** a lane asks with a layout that leaves a property out (or lists no elements), an `enum` member that fails its own `pattern` or `maxLength`, trips the leak guard or is empty, or integer bounds of 0.2 to 0.8
- **THEN** the daemon answers 422 naming the property, and no decision or task block is recorded

#### Scenario: Options and response together are refused
- **WHEN** a lane asks with both `options` and `response`
- **THEN** the daemon answers 422 and no decision or task block is recorded

#### Scenario: A schema-less decision is unchanged
- **WHEN** a lane asks with `options` and `recommended` and no `response`
- **THEN** the decision behaves exactly as before: Now lists it with `response: null`, and the option-pick journey in `app/e2e/tests/relay-run.spec.ts` passes

### Requirement: Structured answer validated by the daemon before storing
Answering a form decision SHALL carry `values` (an object keyed by property name) and no `option`; answering an option decision SHALL carry `option` and no `values`. The daemon SHALL validate every value against the schema stored with the decision before writing, whatever the client already checked: unknown keys, a required property absent, null or empty (a pick-several with no member is empty), a value of the wrong type (`integer` SHALL be a whole number), a string not in its `enum`, a number outside `minimum`/`maximum`, a string over its `maxLength` (or over 600 characters when none is set, 4000 for long text), a pick-several that is not an array of its `items.enum` members or lists one twice, a string not matching its `pattern`, and text tripping the leak guard SHALL each be refused with 422 naming the property. A refused answer SHALL record nothing and leave the decision open. An absent optional property SHALL stay absent; the daemon SHALL NOT fill it from `default`.

#### Scenario: The person answers with valid values
- **WHEN** the person posts `POST /relay/answers` with `decision_id`, `values` satisfying the schema, and `surface: "desktop"`
- **THEN** the daemon records one `answered` action with `props.values`, resumes the tasks the ask blocked, and emits the decision `remove` event

#### Scenario: A required property is missing
- **WHEN** the answer's `values` omit a required property
- **THEN** the daemon answers 422 with a message naming that property, records no `answered` action, and the decision stays on Now

#### Scenario: A value breaks a constraint the client skipped
- **WHEN** a caller posts `values` directly (no form) with an integer above its `maximum`, or a string that does not match its `pattern`
- **THEN** the daemon answers 422 naming that property and rule, and records nothing

#### Scenario: A pick-several answer is checked member by member
- **WHEN** a caller posts `values` with a pick-several holding a member off its list, a member twice, or a string in place of the array
- **THEN** the daemon answers 422 naming that property, and records nothing

#### Scenario: The lane reads the values back
- **WHEN** the asking lane reads its envelope after the person answered a form decision
- **THEN** the envelope's answer carries `values` equal to what the person sent and `option: null`

### Requirement: Form decisions render as a validated form
The Relay screen SHALL render a decision that carries a `response` as a form drawn by `@rokkit/forms` `FormRenderer` from the decision's schema and layout (the lane's layout, or the one the daemon stored in the lane's property order; an element without a `label` or `description` takes the property's `title`, else its name in words, and its `description`), one control per property, inside the existing decision card, in both Needs you and the task view. It SHALL validate inline before sending: a missing required field (a pick-several with no member included), or a value breaking a bound, `maxLength` (or the 600 / 4000 cap when none is set) or `pattern`, SHALL show its message and send nothing; the inline check SHALL read `maxLength` and the caps (in characters) and `pattern` as the daemon does, so it never refuses a value the daemon would take. Answer SHALL send the values once (a second press while the write is in flight sends nothing) and SHALL show the daemon's refusal in place without clearing the entered values. A decision with `options` SHALL render as option buttons, as today.

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

#### Scenario: Pick-several and long text answered live
- **WHEN** the person picks two members of a pick-several, types two lines of long text and presses Answer
- **THEN** the card leaves Needs you without navigation and the lane envelope reads the exact members and text (proved by `relay-decision-form.spec.ts` "a pick-several and long-text decision is answered live")

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
