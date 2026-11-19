# Checkpoint — 2026-11-19 (relay open items — #389 slice 2, the form on the Relay screen)

**Slice:** `add-relay-decision-response-form` slice B + C1 (#389): a form decision on Needs you and the
task view, live with no refresh. Branch `lane/relay-389-s2` (worktree `lanes/relay-open`), from develop
after PR #500 (slice A) merged.

**Done (B1–B3, C1; red then green):**
- `@sensei/state` `relay/decision-form.ts`: `toDecisionForm` (schema through, lane layout else one built
  from the schema, data from `default`) and `toAnswerValues` (empty optional fields dropped, D83).
- `DecisionForm.svelte` (FormRenderer, `data-decision-field`); `DecisionRow` sends `{values}` once and
  keeps them on a refusal; NeedsYou, RelayScreen and RelayTaskScreen pass a `RelayDecisionChoice`.
- e2e `relay-decision-form.spec.ts` 6/6 plus relay-run, conformance, zz-a11y, timeline, sprint, gate green.
- Review r1 (green `65b9b64d6`): task view keyed by decision; required yes/no seeds `false`; field ids
  carry the decision id. e2e 7/7 with relay-run.
- Review r2 fix (red `20bfe00c2`, green `e07d85712`): a pattern means the same to daemon and form. The
  daemon refuses at ask time what the browser reads differently (`check_pattern_dialect`: `\w \d \s \b`,
  `\A \z`, hex, POSIX/nested classes, set ops); the form checks patterns itself with a `u` RegExp
  (since r4, `toPatternMatcher`), not rokkit's plain one. e2e `relay-decision-form` 6/6 again.
- Review r3 (bump `cb5128d4d`, refactor `4e9cfc681`, red `c04eb76a9`, green `76329d33a`): `@rokkit/*` ^1.16.0 in
  app/ and dojo/ (packages/ has no rokkit dep); DecisionCheckbox (rokkit#190) and the NaN handling
  (rokkit#193) removed; Select labels now name their trigger (rokkit#194). maxLength counted in characters
  as the daemon does (`textMisses`/`withoutTextRules`; no native UTF-16 `maxlength`). The daemon refuses
  every keyword 1.16 draws a control from (`x-multiline`, `renderer`, `options`, `min`, `max`, arrays).
  e2e `relay-decision-form` 6/6 on 1.16 (axe passes with rokkit's own checkbox); app 3842+, dojo 2411 unit green.
- Review r4 (red `34f3197b8`, green `51bb94543`): the inline pattern check no longer runs a lane's
  pattern on the browser's backtracking RegExp (`^(a+)+$` on 29 chars took 7s). `toPatternMatcher` parses
  the daemon's dialect and runs a Thompson NFA, linear like Rust `regex`; outside the dialect or past
  10,000 states it leaves the check to the daemon. 56k-case fuzz vs `u` RegExp: 0 diffs. e2e 6/6.
- Review r5 (merge `c34ef7181`): develop merged in; all PR CI green; e2e wizard+relay 25/25.

- Review r6 (red `5e3f730c4`, green `8f999f1ba`): a form without a layout keeps the lane's field order.
  The MCP forwards `properties` unsorted (`preserve_order`); the daemon stores a scope-only layout in that
  order before the jsonb write (`with_lane_order`); the form labels elements from the schema. e2e
  relay-decision-form 6/6 + relay-run green; senseid relay 457, mcp 109.
**Remaining:** CI green on 501; C2 (close #389 on merge). Slice R (R1 rokkit issue, R2 widen) needs its
own issue once #389 closes. Design review of the card (iris-deneb) is a user follow-up.

**Next command:** `gh pr checks 501`.
**Open questions:** none blocking.
**Known broken:** none from this branch. relay-run on a fresh sensei_e2e needs setup-wizard first.
