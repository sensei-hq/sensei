---
name: front-door — acceptance
updated: 2026-07-20
---

# front-door — acceptance

> Outcomes / signals. The acceptance criteria for this chunk and the tests that
> prove them. Coverage for front-door lives here.

This is the honest checklist: what's built and verified, what's shipped but
not yet released, and what's still gapped. Don't round up — a gap recorded
here is a gap that gets fixed later, not one that gets papered over.

## Built + verified

- [x] Recommend/preview/confirm round-trip works on both surfaces (CLI/agent
  `/sensei:intake` and the app `/intake` screen) against `POST
  /api/playbook/recommend`.
- [x] The app `/intake` route, the Observatory rail "Intake" anchor (first
  item), and the freeform → recommend → confirm → recorded flow are covered
  end-to-end — `app/e2e/tests/intake.spec.ts`, 4/4 passing (direct
  navigation renders the screen; the rail anchor navigates to `/intake`;
  freeform → recommend → confirm records a run; plus one more covering the
  same round-trip).
- [x] `preview: true` classifies + recommends only — writes no `playbook_run`
  row. `confirm: true` records exactly one row, reusing the already-classified
  axes (no re-classification, no double-insert).
- [x] The recommend response returns the classified axes (`lifecycle`,
  `intent`, `risk`) alongside the playbook, rationale, opening tone, and
  trust info — so both surfaces can show sensei's read of the chunk back to
  the user as a sanity check, not apply it silently.

## Shipped, not released

These are real, working code on `develop` — not vapor — but haven't gone out
in a tagged release yet (on `develop`, post-`v0.6.0`, not released).

- [x] The §9 learning loop: FTR attribution (the analyzer's `LearnPlaybooks`
  global pass joins a confirmed run's session FTR back onto the
  `playbook_run`), bounded reweight (rule `priority` nudged toward a fixed
  target FTR, clamped, deterministic and idempotent), learned-rule proposals
  (`source='learned'`, disabled by default), and the human accept path.
- [x] Auto-select-on-trust: for low-risk chunks, when a playbook's live track
  record for the exact axes combo hits `n >= 10` and `ftr >= 0.8`, the
  recommendation auto-confirms instead of waiting on the human (reversible —
  the human can still override). High-risk chunks never auto-select.
- [x] The nudge hook (activated on `develop`): a non-blocking, once-per-session
  `PreToolUse` hook (`hooks/nudge`, registered in the sensei plugin manifest,
  wired to `POST /hook/nudge`) suggests `/sensei:intake` when substantive
  work starts without a confirmed `playbook_run` for the session. Fail-open —
  a missing session id or DB error yields no nudge, never a block.

## Known gaps / open

These are recorded, not fixed here. Each has a one-line pointer to where the
fix belongs — a separate chunk, not this one.

- [ ] **Intent taxonomy is code-only.** The `intent` axis has no
  "documentation / product-definition" value — only `explore` / `ux` /
  `feature` / `enhancement` / `bug`. A chunk whose actual goal is writing docs
  or defining a product (not code) gets forced into `ux`, the closest
  available bucket. Observed directly: this docs-restructure effort's own
  intake dogfood classified itself `stable/ux/low` — there was no better axis
  value to pick. Fix = a separate chunk: add the intent value (+ decide
  whether it needs a matching playbook, see `decisions.md`).
- [ ] **Rule-matrix holes.** The six seeded rules don't cover every
  `lifecycle × intent × risk` combination; anything uncovered silently falls
  through to the `gsd` default. Concretely uncovered today: `stable+ux+low`,
  `greenfield+feature+low`, and `greenfield+enhancement+*`. The only signal
  the user gets is `rationale: "no rule matched"` — there's no louder
  indicator that the recommendation was a fallback rather than a matched
  rule. Fix = a separate chunk: either add explicit rules for these combos or
  keep the `gsd` default and make "defaulted" more visible (see
  `decisions.md`, Open decisions).
- [ ] **App form is session-less.** The app path passes the app's session id
  if one exists, else `null` — so app-initiated intakes are recorded but not
  FTR-attributed, and don't feed the §9 learning loop. Only the CLI/agent path
  (which runs inside a live coding session) currently trains the rule
  weights. Fix = a separate chunk, contingent on whether app-initiated chunks
  ever get their own FTR signal (no session to score against today).

## How to verify now

Requires the daemon running on `:7744` with a seeded DB.

```bash
curl :7744/api/playbook/guide
curl :7744/api/playbook/recommend -d '{"chunk":"fix the null deref when the token refreshes","preview":true}'
```

The first returns the intake guide (frame + per-axis prompts + playbook
catalog); the second classifies + recommends without writing a row — add
`"confirm":true` (reusing the same axes) to record exactly one `playbook_run`.

For the app surface: `make test-app-e2e` runs the full Playwright suite,
including `app/e2e/tests/intake.spec.ts` (4/4 today).
