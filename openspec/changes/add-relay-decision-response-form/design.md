# Design: relay decision response forms (#389)

## Context

What exists on develop (`c25d1a930`), grounded:

| Piece | Where | Today |
|---|---|---|
| Ask body | `crates/senseid/src/api/handlers/relay.rs:804-816` (`AskBody`) | `options`, `recommended`, `blocks`, `asked_of`; no schema |
| Answer bodies | `relay.rs:818-831` (`LaneAnswerBody`, `PersonAnswerBody`) | `option` required, `note` optional |
| Ask store | `crates/senseid/src/db/pg_store/relay.rs:5031` | writes `asked.props {options, blocks, asked_of, context?, recommended?}` |
| Answer store | `pg_store/relay.rs:5200`; option check `:5269-5277`; props `:5281` | refuses an unknown option; writes `answered.props {option, surface}` |
| Props check | `crates/senseid/src/relay/mod.rs:584` (Asked), `:605` (Answered) | allow-list; `options` and `option` required |
| Now view | `database/ddl/view/relay/now.ddl:82-92` (`open_asked`) | payload has `options`, `recommended`; no schema |
| Decision row | `pg_store/relay.rs:1338` (`now_decision_json`) | shapes Now (`relay_now`, `:7277`) and the #418 stream row (`relay_decision_row`, `:6937`) |
| Task view decision | `pg_store/relay.rs:7893-7945` (`relay_task_view`) | its own query on `relay.actions` and its own `options` shaping, separate from `now_decision_json`; no schema |
| History | `pg_store/relay.rs:1240-1259` | asked: options…; answered: option + surface |
| Lane envelope | `pg_store/relay.rs:6259-6304`, `crates/senseid/src/relay/envelope.rs:44` | the query decodes `a.props->>'option'` into a non-null `String`; `EnvelopeAnswer.option: String` |
| Live | `relay.rs:1526` (ask → add event), `relay.rs:1860` (person answer → remove event) | decision add/remove already stream (#418) |
| MCP | `crates/mcp/src/lib.rs:616-638` (forward), `:1647-1672` (tool schemas), `:4200-4208` (`missing_required` pin) | forwards `options`/`option` only; `options` is required on ask and `option` on answer |
| State | `packages/state/relay/types.ts:216-235`, `:640-648`; `relay-state.svelte.ts:444`; `task-state.svelte.ts:69`; exports in `packages/state/index.ts` | `answer(decision, option: string)`; `RelayState` answers through its source, `RelayTaskState` through `api.tryAnswerDecision` (`client.ts:77`) directly (after #414: through its `RelayTaskSource`) |
| UI | `app/src/lib/relay/DecisionRow.svelte:18,22,46`; `NeedsYou.svelte:25,47-53`; `RelayScreen.svelte:163-166,253`; `RelayTaskScreen.svelte:89-95` | option buttons + Answer; `onanswer` is `(option: string)` on the row and `(id, option)` on NeedsYou/RelayScreen |
| e2e | `app/e2e/tests/relay-run.spec.ts:250-265` | option-pick ask → answer → card leaves |

Nothing of #389 is built. The live chain it depends on (#378/#418) is merged.

## Goals / Non-Goals

- Goals: a decision may carry a field schema; the person (or an asked lane) answers with structured values in
  one validated write; both surfaces render it from the same state; arrival and resolution are live.
- Non-goals: free-text chat; multi-step or conditional forms; the dōjō transport (#427).

## Decisions

### D1. Wire shape: a field list, not raw JSON Schema

`response = {fields: [Field]}` with `Field = {id, label, kind, required?, options?, default?, help?}`.
`kind ∈ text | textarea | number | boolean | select`. `options = [{id, label}]` only for `select`.
Limits: 1–12 fields; `id` matches `^[a-z][a-z0-9_]{0,39}$` and is unique; `label` ≤ 120 chars.

Why: the brief names exactly these attributes. A closed list is small enough to validate fully in Rust,
and the client maps it to rokkit's JSON-Schema-plus-layout in one pure function. Raw JSON Schema would let a
lane send keywords the daemon cannot check and rokkit cannot draw.

### D2. Options and response are exclusive

A decision carries `options` (non-empty) or `response`, not both, and not neither. `recommended` belongs to
options only. Mixed "pick one and also fill in" is a `select` field plus other fields.

### D3. Answer shape

Answer carries `option` for an option decision, `values` for a form decision; the other key is refused.
`values` is an object keyed by field id. Validation, all before any write (single transaction as today):
unknown key → 422; required missing or empty → 422; `number` not finite → 422; `boolean` not bool → 422;
`select` not an option id → 422; `text`/`textarea` > 600 chars or tripping `guard_markdown`
(`relay/mod.rs:349`) → 422 naming kinds only. Optional fields absent stay absent (D83: null = not reported,
never filled with a default). A field's `default` seeds the form only; the daemon never substitutes it.

The refusal message names the first failing field id and rule, e.g. `values.path is required`.

### D4. Storage: props, no new table or enum

`asked.props.response` and `answered.props.values` ride the existing `relay.actions.props` jsonb
(`database/ddl/table/relay/actions.ddl:18`). No new enum values, so no apply hook. The only DDL edit is the
`relay.now` view payload adding `'response', a.props->'response'`.

### D5. Client mapping and validation

`toDecisionForm(response)` in `packages/state/relay/decision-form.ts` returns `{schema, layout, data}`:
`text`→`{type:'string'}`, `textarea`→`{type:'string'}` with layout `props.type:'textarea'`,
`number`→`{type:'number'}`, `boolean`→`{type:'boolean'}`, `select`→`{type:'string', enum}` with layout
`options`. Required fields go in the object's `required` list. Inline validation uses rokkit `validateAll`;
the daemon re-validates (the client is never trusted). `toAnswerValues(response, data)` drops empty optional
fields so absent stays absent.

### D6. One answer method for both sources

`RelayAnswer` becomes `{decision_id, note, surface} & ({option} | {values})`. `RelayState.answer` and
`RelayTaskState.answer` take `RelayDecisionChoice = {option: string} | {values: Record<string, unknown>}`.
`RelayTaskState` answers through its `RelayTaskSource.answer` (added by #414, `update-shared-state-sources`, which
lands first); its body type is `RelayAnswer`, so it follows. `relayTaskSseSource`, `relaySseSource`,
`relayRealtimeSource` and `relayRealtimeTaskSource` pass it through untouched, so the dōjō gets form answers for free
when its transport lands (#427).

### D7. Live path reuses #418

No new event. The ask handler already emits the decision `add` with `now_decision_json`'s row
(`relay.rs:1526`); that row gains `response`. The person answer already emits `remove` (`relay.rs:1860`).
The lane answer path emits a task event only (unchanged; a lane-asked decision is not on Needs you).

## Decisions defaulted (user may override)

1. **Field list over JSON Schema on the wire** (D1). Alternative: accept a JSON Schema subset. Chosen for
   full daemon-side validation and a closed client mapping.
2. **Options and response are mutually exclusive** (D2). Alternative: allow both and answer with either.
   Chosen to keep one answer shape per decision.
3. **Crew-level (no task) decisions use the same schema.** The brief leaves this to Design; nothing in the
   current code distinguishes them (`RelayAskInput.task_id` is optional, `pg_store/relay.rs:644`), so one
   shape serves both.
4. **Visual: the existing decision card** (warning-tinted, `DecisionRow.svelte:36-39`) hosts the form under
   the question; Answer stays the dark button at the end. The Design review (iris-deneb) of this delta is a
   follow-up the user runs; the build does not wait on it.
5. **No `note` field in the form UI.** The wire keeps `note` (unchanged); the form does not add a free-text
   note box, since free text is out of scope.
6. **Limits:** 12 fields, 600 chars per text value (the summary cap, `relay/mod.rs:364`).

## Risks / Trade-offs

- rokkit `FormRenderer` markup is the library's; e2e selectors use our own `data-decision-field="<id>"`
  wrapper via the `child` snippet only if `FormRenderer`'s `[data-form-field]` cannot be keyed by scope. Task
  2.3 checks the rendered DOM first.
- An older daemon drops an unknown `response` (`AskBody` has no `deny_unknown_fields`) and today accepts an
  empty `options` list (`relay/mod.rs:586-587` only requires the array; nothing else checks it is non-empty). The MCP and daemon ship together in
  one release, so installed pairs agree; this change also makes an empty `options` without `response` a 422
  (D2), closing the empty-card case.

## Migration Plan

Additive. No backfill: existing `asked` rows have `options` and keep working. Rollback is not planned
(additive props are inert to older readers).
