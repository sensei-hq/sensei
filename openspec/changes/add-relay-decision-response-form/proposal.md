# Change: Relay decisions carry a response schema, answered through a form

Issue: #389. Brief: `coord/briefs/relay-structured-response.md` (outside the repo, in the coordination folder).

## Why

A relay decision today is option-pick only. A lane that needs fields back (a path, a count, a yes/no plus a
reason) has no surface, so the exchange falls back to someone carrying the answer by hand. The first live run
(2026-11-09) hit this.

## What Changes

- **Wire (ask).** `POST /api/relay/decisions` and MCP `relay_ask_decision` accept an optional `response`
  (MCP `options` stops being a required tool argument): a
  list of fields, each `{id, label, kind, required, options?, default?, help?}`. `kind` is one of `text`,
  `textarea`, `number`, `boolean`, `select`. A decision carries either `options` (option-pick) or `response`
  (form), never both. Schema-less decisions are unchanged.
- **Wire (answer).** `POST /relay/answers` (person) and `POST /api/relay/decisions/{id}/answer` (lane, MCP
  `relay_answer_decision`) accept `values` (an object keyed by field id) in place of `option` for a form
  decision. The daemon validates the values against the stored schema in one write: required fields present,
  kinds match, `select` values name an option, text values pass the 600-char cap and the leak guard. A refusal
  names the field and why (422), and records nothing.
- **Props.** `asked.props` gains `response`; `answered.props` gains `values`, and `option` becomes required
  only when the ask has options. `validate_props` enforces both shapes.
- **Reads.** The `relay.now` view's `open_asked` payload, the stream's decision row (#418), the task view's
  open decision (shaped separately in `relay_task_view`), the task history, and the lane envelope's answers
  all carry `response` / `values`.
- **State.** `@sensei/state` gains the `RelayDecisionField` / `RelayDecisionResponse` types, a pure
  `toDecisionForm(response)` selector that builds the rokkit `schema` + `layout` + initial `data`, and an
  answer method that takes `{option}` or `{values}`. The desktop SSE source and the dōjō realtime seam share
  it (F8 posture).
- **UI.** `DecisionRow` renders a form decision with `@rokkit/forms` `FormRenderer` (same schema/layout path
  as the filters band, #384), validates inline before sending, sends once (single-flight), and shows the
  daemon's refusal in place. Option-pick rendering is unchanged.
- **Live.** A form decision arrives in Needs you and on the task view with no refresh, and answering removes
  it with no refresh (the #418 stream already carries add/remove; the row just gains `response`).
- **Docs.** `docs/prd/modules/03-delivery/21-relay/lanes-data.md` (props table) and `lanes.md` (decisions)
  describe `response` and `values`.

Not breaking: every existing field keeps its meaning; new fields are optional on the wire.

## Impact

- Affected specs: `relay-decisions` (new capability delta).
- Affected code:
  - `database/ddl/view/relay/now.ddl` (open_asked payload), `database/tests/relay/relay_now.sql`
  - `crates/senseid/src/relay/mod.rs` (validate_props, new `validate_response` / `validate_values`)
  - `crates/senseid/src/db/pg_store/relay.rs` (ask/answer inputs, store, `now_decision_json`, the
    `relay_task_view` decision, history, envelope answers), `crates/senseid/src/relay/envelope.rs`
  - `crates/senseid/src/api/handlers/relay.rs` (AskBody, LaneAnswerBody, PersonAnswerBody)
  - `crates/mcp/src/lib.rs` (tool schemas + forwarded keys)
  - `packages/state/relay/{types.ts,relay-state.svelte.ts,task-state.svelte.ts,decision-form.ts}`,
    `packages/state/index.ts` (exports)
  - `app/src/lib/relay/{DecisionRow.svelte,DecisionForm.svelte,NeedsYou.svelte,RelayScreen.svelte,RelayTaskScreen.svelte,catalogue.ts}`
  - `app/e2e/tests/relay-decision-form.spec.ts` (new)
- Out of scope: free-text chat; the dōjō's supabase transport (#427 epic) — this change keeps the dōjō seam
  (`relayRealtimeSource`) type-compatible only.
- Depends on #414 (`update-shared-state-sources`): `RelayTaskState` answers through the `RelayTaskSource` that change
  adds, so #414 lands first.
