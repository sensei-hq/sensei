# Checkpoint — 2026-11-18 (relay open items — #389 slice 2, the form on the Relay screen)

**Slice:** `add-relay-decision-response-form` slice B + C1 (#389): a form decision on Needs you and the
task view, live with no refresh. Branch `lane/relay-389-s2` (worktree `lanes/relay-open`), from develop
after PR #500 (slice A) merged.

**Done (B1–B3, C1; red then green):**
- `@sensei/state` `relay/decision-form.ts`: `toDecisionForm` (schema through, lane layout else one built
  from the schema, data from `default`) and `toAnswerValues` (empty optional fields dropped, D83).
- `DecisionForm.svelte`: `@rokkit/forms` FormRenderer, each field in `data-decision-field` via the
  `child` snippet; `collect()` checks inline with `validateAll`. `DecisionRow` sends `{values}` once and
  keeps them on a refusal; NeedsYou, RelayScreen and RelayTaskScreen pass a `RelayDecisionChoice`.
- The yes/no draws native (`DecisionCheckbox.svelte`): rokkit's custom checkbox fails axe
  `aria-hidden-focus` — raised as sensei-hq/rokkit#190.
- Catalogue: `DecisionRow/form`, `DecisionRow/form-layout`. Fields stack at phone width (`md:` 2 cols).
- e2e `relay-decision-form.spec.ts` 6/6 (arrives, refuses inline, resolves, task view, + slice A's two);
  relay-run, relay-conformance, zz-a11y, relay, relay-timeline, relay-sprint, relay-gate all green.
  Evidence DOM dumps under `app/test-results/relay-decision-form/`. Real brew daemon restored on :7744.
- Review r1 fixes (PR #501, red `f0608f0e7`, green `65b9b64d6`, e2e `fd0e98b74`, docs `0ddd36400`):
  task view `{#key decision.id}`; cleared number → unset (rokkit#193); required yes/no seeds `false`;
  field ids prefixed by decision id (Select label gap: rokkit#194); arrival e2e closes its crew; inline
  refusal proven by a fetch counter; D6/spec/ask tool say no-layout order is jsonb's. e2e 7/7 with relay-run.
- Gates: app `bun run check` 0 errors; `bun run test:unit` all green; `openspec validate --strict` valid.

**Remaining:** C2 (close #389 on merge, with the e2e evidence; design review of the card by iris-deneb
is a follow-up the user runs). Slice R (R1 rokkit issue for pick-several + long text, R2 widen) is not
built and needs its own issue once #389 closes.

**Next command:** `gh pr checks 501`.

**Open questions:** none blocking. A form with no lane layout lists fields in jsonb's order (D6 amended);
keeping the lane's order would need an order array on the wire and in `relay.now`.

**Known broken:** none from this branch.
