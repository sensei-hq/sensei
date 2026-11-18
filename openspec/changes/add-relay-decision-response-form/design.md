# Design: relay decision response forms (#389)

## Context

What exists on develop (`c25d1a930`; ask/answer bodies and `validate_props` re-checked on `f880251b8`), grounded:

| Piece | Where | Today |
|---|---|---|
| Ask body | `crates/senseid/src/api/handlers/relay.rs:805-817` (`AskBody`) | `options`, `recommended`, `blocks`, `asked_of`; no schema |
| Answer bodies | `relay.rs:819-831` (`LaneAnswerBody`, `PersonAnswerBody`) | `option` required, `note` optional |
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
| Regex | `crates/senseid/Cargo.toml:62` (`regex = "1"`) | already a senseid dependency |
| JSON Schema crate | `Cargo.lock:4655-4670` | only `schemars` (a schema *generator*, pulled in transitively); no validator crate |
| rokkit forms | `app/package.json:33` (`@rokkit/forms` `^1.15.7`); the filters band already draws a `FilterSchema` through it (`app/src/lib/relay/relay-shell.ts:282-292`) | see "rokkit facts" below |

rokkit facts (`@rokkit/forms` 1.15.7, `dist/lib/`), which fix the subset's edges:
- `resolveInputType` picks `props.renderer` first, then any `format` other than `text`/`number`
  (`elements.js:63-65`). A schema `format` or a layout `renderer` therefore chooses the control, so the subset
  refuses both: the lane cannot pick a control the daemon does not check.
- A number with both `min` and `max` (layout spelling) becomes a range slider (`elements.js:50`); the JSON
  Schema spelling `minimum` / `maximum` stays a number input and is still validated (`validation.js:98,110`).
- `pattern` is compiled with `new RegExp` (`validation.js:35`); `maxLength` is checked at `validation.js:48`.
- `type: "array"` maps to `ArrayEditor` (`elements.js:37`, `renderers.js:36`), not `MultiSelect` (which exists
  in `@rokkit/ui`, `dist/components/index.js:11`). A `textarea` renderer exists (`renderers.js:52`) but is
  reachable only through `renderer` / `format`, which the subset refuses. Hence the rokkit issue (R1).

Nothing of #389 is built. The live chain it depends on (#378/#418) is merged.

## Goals / Non-Goals

- Goals: a decision may carry a flat JSON Schema subset (plus an optional rokkit layout); the person (or an asked lane) answers with structured values in
  one validated write; both surfaces render it from the same state; arrival and resolution are live.
- Non-goals: pick-several and long-text fields until rokkit draws them (D9); free-text chat; multi-step or conditional forms; the dōjō transport (#427).

## Decisions

### D1. Wire shape: a flat JSON Schema subset plus an optional rokkit layout

`response = {schema, layout?}`. The lane writes what `FormRenderer` draws; sensei adds no field format of its own.

`schema` (all of these, nothing else):
- `type: "object"`, `properties` (1-12 entries), optional `required` (names from `properties`, no duplicates).
- Each property name matches `^[a-z][a-z0-9_]{0,39}$`.
- Each property: `type` ∈ `string | number | integer | boolean`, plus only the keywords its type allows:
  - `string`: `enum` (1-50 unique strings, each ≤ 120 chars; pick one), `maxLength` (1-600), `pattern`
    (≤ 200 chars; must compile with the `regex` crate and must not contain `(?`, so inline flags and named
    groups, which JS spells differently, are refused).
  - `number` / `integer`: `minimum`, `maximum` (finite; `minimum ≤ maximum`).
  - `boolean`: no constraint keywords.
  - every type: the annotations `title` (≤ 120), `description` (≤ 300) and `default` (must itself pass the
    property's rules). Annotations never change what an answer must be.
- Anything else is refused by name: `format`, `items`, `type: "array"` or `"object"` inside a property,
  `$ref`, `oneOf`/`anyOf`/`allOf`, `additionalProperties`, `minLength`, `exclusiveMinimum`, and so on. The
  refusal names the keyword and the property: `response.schema.properties.files.type: array is not supported`.

`layout` (optional): `{type: "vertical", elements: [...]}` where each element is `{scope: "#/<name>", label?,
description?, props?: {placeholder?}}`, `scope` names a schema property, each property appears at most once,
`label`/`placeholder` ≤ 120 chars and `description` ≤ 300. `renderer`, `override`, `readonly`, `options`,
`min`/`max` and any other layout key are refused (they choose controls or values the daemon does not check).

Why: the user chose this on 2026-11-18. One shape end to end (lane → daemon → rokkit) with no mapping layer,
and a closed subset small enough to check completely in Rust.

### D2. Options and response are exclusive

A decision carries `options` (non-empty) or `response`, not both, and not neither. `recommended` belongs to
options only. Mixed "pick one and also fill in" is a string property with `enum` plus other properties.

### D3. Answer shape and server-side validation

Answer carries `option` for an option decision, `values` for a form decision; the other key is refused.
`values` is an object keyed by property name. Validation, all before any write (single transaction as today),
against the schema stored on the `asked` action:
- unknown key → 422; a `required` property absent, null or `""` → 422;
- `string` not a string, `number` not a finite number, `integer` not a whole number, `boolean` not a bool → 422;
- `enum` miss, `minimum` / `maximum` out of range, over `maxLength` (or over 600 when the schema sets none),
  `pattern` not matching (unanchored, as JSON Schema says) → 422;
- text tripping `guard_markdown` (`relay/mod.rs:349`) → 422 naming kinds only.

Optional properties absent stay absent (D83: null = not reported, never filled with a default). A `default`
seeds the form only; the daemon never substitutes it. The refusal names the first failing property and rule,
e.g. `values.path is required`.

### D4. Storage: props, no new table or enum

`asked.props.response` and `answered.props.values` ride the existing `relay.actions.props` jsonb
(`database/ddl/table/relay/actions.ddl:18`). No new enum values, so no apply hook. The only DDL edit is the
`relay.now` view payload adding `'response', a.props->'response'`.

### D5. The checker: hand-written, in one module

`crates/senseid/src/relay/decision_schema.rs` owns `validate_response(&Value)` (ask time) and
`validate_values(&Value /*response*/, &Value /*values*/)` (answer time), both pure. `validate_props(Asked)`
calls the first; the answer store calls the second against the stored `asked.props.response`.

Why hand-written: the workspace has no JSON Schema validator (`Cargo.lock` has only `schemars`, a generator),
and a general validator would accept the whole spec, so the subset check would have to be written anyway. The
closed subset is a few dozen lines over `serde_json::Value` plus the `regex` crate senseid already uses
(`crates/senseid/Cargo.toml:62`). Compiled patterns are not cached (≤ 12 per answer, ≤ 200 chars each).

### D6. Client mapping and validation

`toDecisionForm(response)` in `packages/state/relay/decision-form.ts` returns `{schema, layout, data}`: the
schema unchanged; the lane's layout when sent, else a vertical layout with one element per property in the
order the daemon returns `properties`, labelled from `title` (else the property name in words) with
`description` as help; and `data` seeded from each `default`, with a required boolean that has no default seeded
`false` (its unchecked box reads as no, so an untouched box answers no rather than failing `required`). The
returned order is not the lane's: the daemon keeps it in memory, but `relay.actions.props` is jsonb, which stores
object keys shorter-first, then by bytes. A lane that needs a set order sends a layout (an array, so its order
survives). Inline validation uses rokkit `validateAll` (`required` honoured from the
object's list, per the rokkit README). `toAnswerValues(schema, data)` drops empty optional properties so absent
stays absent. The daemon re-validates (D3).

### D7. One answer method for both sources

`RelayAnswer` becomes `{decision_id, note, surface} & ({option} | {values})`. `RelayState.answer` and
`RelayTaskState.answer` take `RelayDecisionChoice = {option: string} | {values: Record<string, unknown>}`.
`RelayTaskState` answers through its `RelayTaskSource.answer` (added by #414, `update-shared-state-sources`, which
lands first); its body type is `RelayAnswer`, so it follows. `relayTaskSseSource`, `relaySseSource`,
`relayRealtimeSource` and `relayRealtimeTaskSource` pass it through untouched, so the dōjō gets form answers for free
when its transport lands (#427).

### D8. Live path reuses #418

No new event. The ask handler already emits the decision `add` with `now_decision_json`'s row
(`relay.rs:1526`); that row gains `response`. The person answer already emits `remove` (`relay.rs:1860`).
The lane answer path emits a task event only (unchanged; a lane-asked decision is not on Needs you).

### D9. Pick-several and long text wait for rokkit

Refused by D1 today. Task R1 raises a sensei-hq/rokkit issue: FormRenderer maps `type: "array"` with
`items.enum` to `MultiSelect`, and a schema-level long-text hint maps a string to its textarea. When rokkit
releases it, task R2 bumps `@rokkit/forms` in `app/` and `packages/`, widens D1 to `array` + `items.enum`
(answer: an array of unique enum members, with `minItems` / `maxItems` if rokkit honours them) and the
long-text hint (its own `maxLength` cap, decided with the hint), and adds red tests for both first. No sensei
override in the meantime.

## Decided by the user (2026-11-18)

1. **A flat JSON Schema subset plus an optional rokkit layout, no custom field format** (D1). Rendered by
   `@rokkit/forms` `FormRenderer`. Subset: one object; `string`/`number`/`integer`/`boolean`; `enum` (pick
   one); `required`; `minimum`/`maximum`; `maxLength`; `pattern`. Replaces the earlier default (a sensei field
   list).
2. **The daemon enforces the subset** at ask time (422 for anything outside it) and **validates the answer
   against the schema before storing**, not only in the client (D3, D5).
3. **Pick-several and long text extend rokkit, not sensei** (D9): a sensei-hq/rokkit issue, then a rokkit bump
   that widens the subset. Until then the subset check refuses them.
4. **Options and response are mutually exclusive** (D2): a decision has buttons or a form, never both.
   Schema-less decisions keep option-pick unchanged.
5. **Crew-level (no task) decisions use the same shape.** `RelayAskInput.task_id` is optional
   (`pg_store/relay.rs:644`), and nothing distinguishes them.
6. **Visual: the existing decision card** (warning-tinted, `DecisionRow.svelte:36-39`) hosts the form under
   the question; Answer stays the dark button at the end. The Design review (iris-deneb) of this delta is a
   follow-up the user runs; the build does not wait on it.

## Decisions defaulted (user may override)

1. **Annotations allowed:** `title`, `description` and `default` ride the schema (D1); they label and seed the
   form and never change validation.
2. **The checker is hand-written** (D5) rather than a JSON Schema crate.
3. **`pattern` dialect:** must compile with Rust `regex` and contain no `(?`, so the daemon and the browser's
   `RegExp` agree on what it means.
4. **No `note` field in the form UI.** The wire keeps `note` (unchanged); the form does not add a free-text
   note box, since free text is out of scope.
5. **Limits:** 12 properties, 50 enum members, 600 chars per text value (the summary cap, `relay/mod.rs:364`).

## Risks / Trade-offs

- rokkit `FormRenderer` markup is the library's; e2e selectors use our own `data-decision-field="<name>"`
  wrapper via the `child` snippet only if `FormRenderer`'s `[data-form-field]` cannot be keyed by scope. Task
  B2.3 checks the rendered DOM first.
- A future rokkit release could change what a schema keyword draws. The subset check (D1) pins the keywords;
  R2 re-reads `resolveInputType` on every rokkit bump that touches forms.
- An older daemon drops an unknown `response` (`AskBody` has no `deny_unknown_fields`) and today accepts an
  empty `options` list (`relay/mod.rs:586-587` only requires the array; nothing else checks it is non-empty). The MCP and daemon ship together in
  one release, so installed pairs agree; this change also makes an empty `options` without `response` a 422
  (D2), closing the empty-card case.
- The daemon's `regex` and the browser's `RegExp` are different engines. Refusing `(?` covers the syntax that
  differs in practice; a pattern both accept but read differently is possible in theory, and the daemon's
  answer is the one that counts.

## Migration Plan

Additive. No backfill: existing `asked` rows have `options` and keep working. Rollback is not planned
(additive props are inert to older readers).
