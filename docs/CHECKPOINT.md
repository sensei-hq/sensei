# Checkpoint — 2026-10-15

**Slice:** #238 Claude Code mods. Step 2 (usage at source).

**Done** (pushed to develop, ea4bb6e3..1f1a5abd):
- `marketplace/plugins/sensei/hooks/usage.ts` (mods module): forwards `session.measure` as
  `SessionMeasure` and `turn.complete` as `TurnComplete` to `/hook/event`, and dead-letters
  to `~/.sensei/events.jsonl` on failure. 8 tests; `make test-plugin` is in `test` and `test-fast`.
- `activity.sessions.list_cost_usd`: the /cost list-price total, kept at its max. Written by
  `capture_drain::apply_to_session` on both the live and the drain paths.
- Live-verified: a `claude -p --plugin-dir` turn produced cost 0.4233614, equal to the run's
  `total_cost_usd`.

**Remaining in #238:** step 3 (model by risk class: `agent.spawn` / `turn.step`),
step 4 (unattended drive: `$.clock.every` + `$.prompt.submit`), step 5 (`tool.check` gating).
**Then the earlier queue:** #225 live verification → #225 app per-checkout status → #268 → #254 JPA 8b.

**Next command:** `/sensei:session refocus`, then pick #238 step 3 or #225 live verification.

**Open questions:**
- Does `--resume` keep the session id and reset /cost? If it does, `GREATEST` under-counts
  resumed sessions. Not yet observed.
- Should `check_hooks` WARN when the mods module is missing (no cost capture)?

**Known-broken / pending:**
- The mod reaches installs only at the next `make bump`. The column ships with the next
  release (already reconciled to the live and sensei_test DBs).
- The committed `database/design.dbml` is stale against `dbd dbml` output (~1.7k-line diff).
  This predates the slice and was left untouched.
