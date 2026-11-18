# Checkpoint — 2026-11-19 (relay open items — #389 slice 1, the wire)

**Slice:** `add-relay-decision-response-form` slice A (#389): ask with a response schema, answer with
values. Branch `lane/relay-389-s1` (worktree `lanes/relay-open`), PR to develop.

**Done (tasks A1–A6.1, red then green each):**
- `relay/decision_schema.rs`: `validate_response` (the closed subset, refused by keyword and property)
  and `validate_values` (unknown keys, required, type, enum, bounds, maxLength/600, pattern, leak guard).
- `validate_props`: Asked takes `options` (non-empty) or `response`, never both or neither; Answered
  takes `option` or `values`.
- Store: `RelayAskInput.response`, `RelayAnswerInput {option?, values?}`; the answer is checked against
  the stored schema before the write. `relay.now` `open_asked` carries `response`; Now, the stream row,
  the task view, history and the envelope (`EnvelopeAnswer.option?`, `values?`) read it.
- Routes and MCP: `AskBody.response`, answer bodies take `values`; MCP forwards both, `options`/`option`
  are optional tool args.
- `@sensei/state`: `RelayDecision.response`, `RelayAnswer` union, `answer(decision, {option}|{values})`.
- Docs: `lanes-data.md` props table, `lanes.md` §7 decisions.
- e2e: `relay-decision-form.spec.ts` "the broker round-trips a form decision".

**Remaining:** slice B (the form on the Relay screen, live) and slice R (rokkit issue + widen).

**Next command:** `gh pr checks <PR>` for lane/relay-389-s1; then slice B from tasks.md B1.1.

**Open questions:** none blocking.

**Known broken:** none from this branch. The lane DB tests run against the lane template
`sensei_test_relay_389_s1` (`TEST_DATABASE_URL=postgresql://localhost:5432/sensei_test_relay_389_s1`),
built from this tree's DDL (the `relay.now` view changed). Relay e2e specs alone after
`make reset-e2e-db` need a workspace first: include `diagrams-schema` in the run.
