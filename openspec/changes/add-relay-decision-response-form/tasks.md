## Slice A — the wire: ask with a schema, answer with values (daemon + DDL + MCP + state types)

Red commit first for every numbered group; green in the next commit.

### A1. The subset checker and props (red first)
- [x] A1.1 Red: `crates/senseid/src/relay/decision_schema.rs` unit tests for `validate_response` — accepts a
  three-property schema (string+enum, integer with minimum/maximum, boolean) with and without a layout;
  refuses, each naming the keyword and property: no properties, >12 properties, a bad property name, `required`
  naming an unknown property, `type: "array"` (pick-several), `items`, `format` (e.g. `"textarea"`), a nested
  `object`, `$ref` / `oneOf`, `enum` on a non-string, duplicate or >50 enum members, `maxLength` 0 or >600,
  `minimum > maximum`, a non-finite bound, a pattern that does not compile, a pattern containing `(?`, a
  `default` that breaks its own rules; and in the layout: a scope naming no property, a property twice,
  `renderer`, `override`, `readonly`, `options`, `min`/`max`, any other key
- [x] A1.2 Red: `validate_values(response, values)` unit tests — unknown key; required absent / null / `""`; wrong
  type per type (`integer` given 2.5, `number` given `"3"`, `boolean` given `"true"`); enum miss; below
  `minimum` / above `maximum`; over `maxLength`; over 600 with no `maxLength`; `pattern` miss (and an unanchored
  match passes); leak-guard hit (kinds only); an absent optional stays absent (no default filled)
- [x] A1.3 Red: `relay/mod.rs` unit tests — `validate_props(Asked)` accepts `response` and refuses
  options+response, neither, empty `options` alone, and an out-of-subset `response` (delegates to A1.1);
  `validate_props(Answered)` accepts `values` without `option`
- [x] A1.4 Green: `decision_schema.rs` (pure, `serde_json::Value` + the `regex` crate already in
  `crates/senseid/Cargo.toml:62`; no new dependency), wired from `validate_props` (Asked at
  `relay/mod.rs:584`, Answered at `:605`)

### A2. Store (red first; `--test-threads=1`, `make test-template` after the view edit)
- [x] A2.1 Red: `pg_store/relay.rs` tests — ask with `response` stores `props.response`; ask with an
  out-of-subset `response` (an `array` property) → Validation and no action or block written; answer with
  `values` stores `props.values` and resumes blocks; answer with `option` on a form decision → Validation; answer with
  `values` on an option decision → Validation; invalid values → Validation, nothing written
- [x] A2.2 Green: `RelayAskInput.response: Option<RelayDecisionResponse>`, `RelayAnswerInput` becomes
  `option: Option<String>, values: Option<Value>`; ask (`:5031`), answer (`:5200`, replace the option check at
  `:5269-5277` with: option decision → option check; form decision → `validate_values` against the stored
  `asked.props.response`, before the write)

### A3. Reads (red first)
- [x] A3.1 Red: pgTAP in `database/tests/relay/relay_now.sql` — `open_asked` payload carries `response` (null for an
  option decision)
- [x] A3.2 Green: `database/ddl/view/relay/now.ddl:82-88` adds `'response', a.props->'response'`
- [x] A3.3 Red: store tests — `now_decision_json` (`:1338`) row, the `relay_task_view` open decision
  (`:7893-7945`, shaped separately), task history (`:1240-1259`) and the lane envelope answers (`:6259-6304`)
  carry `response` / `values`; the envelope still reads (no decode error) after a form answer, with
  `EnvelopeAnswer.option` null
- [x] A3.4 Green: shape them; the envelope query decodes `props->>'option'` as `Option<String>` and selects
  `props->'values'`; `EnvelopeAnswer` (`relay/envelope.rs:44`) gains `values`, `option` becomes `Option`

### A4. HTTP + MCP (red first)
- [x] A4.1 Red: handler tests in `api/handlers/relay.rs` — ask with `response` round-trips on `GET /relay/now`;
  ask with `type: "array"` → 422 naming the property;
  the #418 stream `add` row equals the Now row incl. `response`; `POST /relay/answers` with `values` → 200 and the
  `remove` event; bad values → 422 naming the field
- [x] A4.2 Green: `AskBody` gains `response`; `LaneAnswerBody` / `PersonAnswerBody` gain `values`, `option` optional
- [x] A4.3 Red: `crates/mcp/src/lib.rs` tests — `relay_ask_decision` forwards `response`; `relay_answer_decision`
  forwards `values`; the `missing_required` pin (`:4200-4208`) drops `options` (ask) and `option` (answer): the
  daemon enforces one-of
- [x] A4.4 Green: forwarding at `:616-638`, tool schemas at `:1647-1672` (`options`/`option` move to optional,
  `response`/`values` added as optional)

### A5. State types (red first)
- [x] A5.1 Red: `packages/state/relay` specs — `RelayDecision.response` (`{schema, layout?}`), `RelayAnswer` union; `RelayState.answer` /
  `RelayTaskState.answer` send `{option}` or `{values}`; `relaySseSource`, `relayTaskSseSource`,
  `relayRealtimeSource` and `relayRealtimeTaskSource` pass the body through (`sources.spec.ts`)
- [x] A5.2 Green: `types.ts:216-235,640-648`, `relay-state.svelte.ts:444`, `task-state.svelte.ts` answer (posts via
  its `RelayTaskSource.answer`, the seam #414 adds; this change lands after #414); DecisionRow callers keep passing
  `{option}`

### A6. Docs + proof
- [x] A6.1 `lanes-data.md` props table (`asked.response`, `answered.values`) and `lanes.md` decisions paragraph,
  including the subset (what is accepted, what is refused and why); the MCP `relay_ask_decision` description
  states the subset in one line
- [x] A6.2 e2e `app/e2e/tests/relay-decision-form.spec.ts` test "the broker round-trips a form decision": a lane asks
  with `response`; `GET /relay/now` shows it; the person posts `values`; the asking lane's envelope reads them back.
  And `relay-run.spec.ts` option-pick journey stays green
- [x] A6.3 Gates: `cargo fmt --all --check`, clippy `-D warnings`, `make test-fast`, senseid DB tests, `bun run test`
  in `packages/state`; ratchets untouched

## Slice B — the form on the Relay screen, live

### B1. Form mapping (red first)
- [ ] B1.1 Red: `packages/state/relay/decision-form.spec.ts` — `toDecisionForm` passes the schema through
  unchanged; uses the lane's layout when sent; else builds a vertical layout in `properties` order labelled
  from `title` (else the name in words) with `description` as help; seeds `data` from `default`;
  `toAnswerValues` drops empty optional properties and keeps required ones
- [ ] B1.2 Green: `packages/state/relay/decision-form.ts`, exported from `packages/state/index.ts`

### B2. Component (red first)
- [ ] B2.1 Red: `DecisionRow.spec.svelte.ts` + harness — a form decision draws one control per schema property
  (`data-decision-field="<name>"`; an `enum` string draws a select, a boolean a checkbox, a number a number
  input), Answer is refused inline while a required field is empty or a `pattern` / bound fails (no write), sends
  `{values}` once on Answer, disables while busy (double press = one write), shows the refusal via `Notice`
  (`data-decision-error`) and keeps the values; an option decision renders as today
- [ ] B2.2 Green: `DecisionRow.svelte` branches on `decision.response`; a `DecisionForm.svelte` beside it uses
  `@rokkit/forms` `FormRenderer` with the selector's schema/layout; `onanswer` takes a `RelayDecisionChoice`
- [ ] B2.3 Check the rendered rokkit DOM first; key fields by our wrapper if `[data-form-field]` cannot be addressed
  by scope
- [ ] B2.4 Wire `NeedsYou.svelte:25,47-53`, `RelayScreen.svelte:163-166,253` (`answer(id, option)`) and
  `RelayTaskScreen.svelte:89-95` to the new `onanswer` taking a `RelayDecisionChoice`
- [ ] B2.5 `catalogue.ts` entry `DecisionRow/form` (string, string+enum, number with bounds, integer, boolean;
  one with a lane layout, one without); phone width stacks fields (`md:` for wider)

### B3. Live e2e (Tauri Playwright)
- [ ] B3.1 `relay-decision-form.spec.ts` test "a form decision arrives with no refresh": a lane asks with a
  three-field schema while the screen is open; the card and its fields appear in Needs you without navigation
- [ ] B3.2 Same spec "required fields refuse inline": Answer with the required field empty shows the field message
  and the card stays (no write: the lane envelope has no answer)
- [ ] B3.3 Same spec "answering resolves with no refresh": fill, Answer; the card leaves Needs you without navigation;
  the lane envelope reads the exact `values`
- [ ] B3.4 Same spec "the task view answers too": a task-bound form decision answered from
  `/workspace/<id>/relay/<crew>/<task>` leaves with no refresh
- [ ] B3.5 `relay-run.spec.ts` (option-pick) and `relay-conformance.spec.ts` unregressed; `zz-a11y.spec.ts` stays at
  0 serious on the Relay screen with a form decision showing
- [ ] B3.6 Gates: `bun run check`, `bun run test` in `app/`, `make test-app-e2e` relay specs green; evidence DOM dumps
  under `test-results/relay-decision-form/`

## Slice R — widen the subset when rokkit draws pick-several and long text

- [ ] R1 Raise an issue on sensei-hq/rokkit (with the sensei use case, #389): `FormRenderer` maps
  `type: "array"` with `items.enum` to `MultiSelect` (today `ArrayEditor`, `@rokkit/forms` `dist/lib/elements.js:37`),
  and honours a schema-level long-text hint that draws its `InputTextArea` (`dist/lib/renderers.js:52`) without a
  layout `renderer`. Link the issue from #389. Done when the issue exists; this change does not wait on it.
- [ ] R2 When rokkit ships R1: bump `@rokkit/forms` (and `@rokkit/*` peers) in `app/` and `packages/`; red tests
  first in `decision_schema.rs` (array of unique enum members accepted, off-list member / duplicate / wrong type
  refused; the long-text hint accepted with its cap) and in `decision-form.spec.ts` / `DecisionRow.spec.svelte.ts`
  (MultiSelect and textarea drawn); then widen D1/D3 and the `relay-decisions` spec. Re-read
  `resolveInputType` in the new release so no other keyword starts choosing a control. Track as its own issue
  if #389 has closed by then.

## Close
- [ ] C1 `openspec validate add-relay-decision-response-form --strict`
- [ ] C2 Close #389 with the e2e evidence (merged to develop). Design review of the card (iris-deneb) is listed in the
  closing comment as a follow-up the user runs
