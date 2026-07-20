---
name: front-door — decisions
updated: 2026-07-20
---

# front-door — decisions

> Append-only. One entry per decision: date · decision · why · alternatives.
> The chunk's anti-rework memory — never re-derive a settled choice.

## Decisions

- **2026-07-19 — Persist-on-confirm.** The app form records the `playbook_run`
  row on confirm, not on preview. Why: `preview: true` has to be free to call
  repeatedly (every recommendation the user sees before deciding) without
  writing garbage rows — persistence has to be a deliberate, single act tied
  to the user's actual confirm.

- **2026-07-19 — Session-less app form.** The app path passes the app's
  active session id if one exists, else `null`; the run is still recorded but
  is advisory only. Why: the app isn't a coding session, so there's no FTR
  signal to attribute back to it — attributing a fake or absent outcome would
  corrupt the learning loop. The CLI/agent path, which runs inside a live
  coding session, is what feeds §9 learning; the app tells the user what
  sensei would do without pretending to train on it.

- **2026-07-19 — Auto-select thresholds: `Risk::Low` + `n >= 10` + `ftr >=
  0.8`.** Deliberately stricter than the reweight/learn thresholds. Why:
  auto-select means skipping human oversight entirely, which demands more
  evidence than merely nudging a rule's priority — a rule can be reweighted
  on thinner evidence because a human still confirms every run either way;
  auto-select removes that check, so the bar for trusting it is higher.
  High-risk chunks are excluded outright, not just held to a higher bar.

- **2026-07-19 — Reweight target FTR = 0.5 (fixed, not self-referential).**
  The bounded reweight nudges a rule's priority toward a fixed target FTR of
  0.5, not toward the rule set's own running mean. Why: a self-referential
  target (e.g. "the current global average FTR") would let the whole system
  drift as it reweights itself — a fixed target keeps the adjustment
  meaningful and comparable across passes instead of chasing a moving goal.

## Open decisions

- Whether to add a documentation/product-definition `intent` value (and, if
  so, whether it needs a matching playbook, or maps onto an existing one).
  See `tests/acceptance.md` — the gap this closes.
- Whether to fill the rule-matrix holes (`stable+ux+low`,
  `greenfield+feature+low`, `greenfield+enhancement+*`) with explicit rules,
  or keep the `gsd` default — and separately, whether a "defaulted, no rule
  matched" recommendation should surface more loudly to the user than the
  current one-line `rationale` string.
