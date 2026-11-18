## Slice A — the wire: ask with a schema, answer with values (daemon + DDL + MCP + state types)

Red commit first for every numbered group; green in the next commit.

### A1. Props and validation (red first)
- [ ] A1.1 Red: `relay/mod.rs` unit tests — `validate_props(Asked)` accepts `response` and refuses options+response,
  neither, empty `options` alone, bad field id, duplicate id, >12 fields, `select` without options, unknown kind;
  `validate_props(Answered)` accepts `values` without `option`
- [ ] A1.2 Red: `validate_values(response, values)` unit tests — unknown key, required missing/empty, wrong kind per
  kind, `select` off-list, text >600 chars, leak-guard hit (kinds only), absent optional stays absent
- [ ] A1.3 Green: implement in `crates/senseid/src/relay/mod.rs` (Asked at `:584`, Answered at `:605`, new
  `validate_response` + `validate_values` beside them)

### A2. Store (red first; `--test-threads=1`, `make test-template` after the view edit)
- [ ] A2.1 Red: `pg_store/relay.rs` tests — ask with `response` stores `props.response`; answer with `values`
  stores `props.values` and resumes blocks; answer with `option` on a form decision → Validation; answer with
  `values` on an option decision → Validation; invalid values → Validation, nothing written
- [ ] A2.2 Green: `RelayAskInput.response: Option<RelayDecisionResponse>`, `RelayAnswerInput` becomes
  `option: Option<String>, values: Option<Value>`; ask (`:5031`), answer (`:5200`, replace the option check at
  `:5269-5277`)

### A3. Reads (red first)
- [ ] A3.1 Red: pgTAP in `database/tests/relay/relay_now.sql` — `open_asked` payload carries `response` (null for an
  option decision)
- [ ] A3.2 Green: `database/ddl/view/relay/now.ddl:82-88` adds `'response', a.props->'response'`
- [ ] A3.3 Red: store tests — `now_decision_json` (`:1338`) row, the `relay_task_view` open decision
  (`:7893-7945`, shaped separately), task history (`:1240-1259`) and the lane envelope answers (`:6259-6304`)
  carry `response` / `values`; the envelope still reads (no decode error) after a form answer, with
  `EnvelopeAnswer.option` null
- [ ] A3.4 Green: shape them; the envelope query decodes `props->>'option'` as `Option<String>` and selects
  `props->'values'`; `EnvelopeAnswer` (`relay/envelope.rs:44`) gains `values`, `option` becomes `Option`

### A4. HTTP + MCP (red first)
- [ ] A4.1 Red: handler tests in `api/handlers/relay.rs` — ask with `response` round-trips on `GET /relay/now`;
  the #418 stream `add` row equals the Now row incl. `response`; `POST /relay/answers` with `values` → 200 and the
  `remove` event; bad values → 422 naming the field
- [ ] A4.2 Green: `AskBody` gains `response`; `LaneAnswerBody` / `PersonAnswerBody` gain `values`, `option` optional
- [ ] A4.3 Red: `crates/mcp/src/lib.rs` tests — `relay_ask_decision` forwards `response`; `relay_answer_decision`
  forwards `values`; the `missing_required` pin (`:4200-4208`) drops `options` (ask) and `option` (answer): the
  daemon enforces one-of
- [ ] A4.4 Green: forwarding at `:616-638`, tool schemas at `:1647-1672` (`options`/`option` move to optional,
  `response`/`values` added as optional)

### A5. State types (red first)
- [ ] A5.1 Red: `packages/state/relay` specs — `RelayDecision.response`, `RelayAnswer` union; `RelayState.answer` /
  `RelayTaskState.answer` send `{option}` or `{values}`; `relaySseSource`, `relayTaskSseSource`,
  `relayRealtimeSource` and `relayRealtimeTaskSource` pass the body through (`sources.spec.ts`)
- [ ] A5.2 Green: `types.ts:216-235,640-648`, `relay-state.svelte.ts:444`, `task-state.svelte.ts` answer (posts via
  its `RelayTaskSource.answer`, the seam #414 adds; this change lands after #414); DecisionRow callers keep passing
  `{option}`

### A6. Docs + proof
- [ ] A6.1 `lanes-data.md` props table (`asked.response`, `answered.values`) and `lanes.md` decisions paragraph
- [ ] A6.2 e2e `app/e2e/tests/relay-decision-form.spec.ts` test "the broker round-trips a form decision": a lane asks
  with `response`; `GET /relay/now` shows it; the person posts `values`; the asking lane's envelope reads them back.
  And `relay-run.spec.ts` option-pick journey stays green
- [ ] A6.3 Gates: `cargo fmt --all --check`, clippy `-D warnings`, `make test-fast`, senseid DB tests, `bun run test`
  in `packages/state`; ratchets untouched

## Slice B — the form on the Relay screen, live

### B1. Form mapping (red first)
- [ ] B1.1 Red: `packages/state/relay/decision-form.spec.ts` — `toDecisionForm` per kind (schema type, layout,
  `required` list, `select` options, `default` seeds data); `toAnswerValues` drops empty optional fields
- [ ] B1.2 Green: `packages/state/relay/decision-form.ts`, exported from `packages/state/index.ts`

### B2. Component (red first)
- [ ] B2.1 Red: `DecisionRow.spec.svelte.ts` + harness — a form decision draws one field per schema field
  (`data-decision-field="<id>"`), Answer is refused inline while a required field is empty (no write), sends
  `{values}` once on Answer, disables while busy (double press = one write), shows the refusal via `Notice`
  (`data-decision-error`) and keeps the values; an option decision renders as today
- [ ] B2.2 Green: `DecisionRow.svelte` branches on `decision.response`; a `DecisionForm.svelte` beside it uses
  `@rokkit/forms` `FormRenderer` with the selector's schema/layout; `onanswer` takes a `RelayDecisionChoice`
- [ ] B2.3 Check the rendered rokkit DOM first; key fields by our wrapper if `[data-form-field]` cannot be addressed
  by scope
- [ ] B2.4 Wire `NeedsYou.svelte:25,47-53`, `RelayScreen.svelte:163-166,253` (`answer(id, option)`) and
  `RelayTaskScreen.svelte:89-95` to the new `onanswer` taking a `RelayDecisionChoice`
- [ ] B2.5 `catalogue.ts` entry `DecisionRow/form` (all five kinds); phone width stacks fields (`md:` for wider)

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

## Close
- [ ] C1 `openspec validate add-relay-decision-response-form --strict`
- [ ] C2 Close #389 with the e2e evidence (merged to develop). Design review of the card (iris-deneb) is listed in the
  closing comment as a follow-up the user runs
