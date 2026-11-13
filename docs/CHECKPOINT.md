# Checkpoint — 2026-11-13 (lanes/relay-screen on lane/relay-screen-rebuild, from develop @ d19c71807)

**Slice:** Relay screen rebuild to mockup r1 — one issue at a time, each component verified against the mockup
(desktop + phone × light/dark × kanji/icon) on the bench before integration (user, 2026-11-12). Design:
`docs/design/2026-11-12-relay-screen-rebuild.md`; issues #418–#429 (#429 = the rig).

**Done:**
- #429 rig (`/gallery`, `gallery:capture`, `gallery:sheet --embed`); review sheet = artifact 3869a0ea.
- #420 shell + filters + footer; #421 `@sensei/state` + both pages cut over; #422 Now; #423 Queue (+add-to-triage
  form, not in mockup); #424 Crew; #425 Task view (#380/#381/#388 fixed; relay hooks no longer collide with rokkit).
  Each benched vs mockup (`coord/reports/relay-screen/bench-r1..r7`), relay e2e 8/8 against the real daemon.
- #418 DONE on the lane (comment on the issue lists each commit): crew filter, checkpoint step events, Now lane
  step counters, lane/run/task rows, `decision`/`message`/`item` entities; the app applies rows in place and reads
  nothing back after its writes (`863742b5d`). Live check on a scratch broker: `coord/reports/relay-screen/daemon-418/`.

- LaneRow step progress (not in the mockup): selector `3ca277909`, component `9193b3fe2`, bench `008575ac0`
  (`bench-r8`; sheet artifact 3869a0ea v10). Awaiting the user's call on the design.

**Remaining (in order):** #419 (draft/start/cancel run, nudge, catalogue) → #426 run-journey e2e. Gaps outside
#418: `task.waiting`; person-message read ladder. Lane is NOT merged to develop (user's call).

**Next command:** #419 — red tests for `POST /relay/runs/{id}/start|cancel` and the nudge route in
`crates/senseid/src/api/handlers/relay.rs`. Installed: daemon = this lane; /Applications app = v0.21.1 (diagrams skew,
backlog item) — `make install-app` from this lane would match them.

**Open questions (user):** bench sheet feedback; retire `lane/relay-screen-conformance`?; AddItemForm design;
phone header height; merge #417 + `make release`; `font-display` no-op elsewhere in the app.

**Known broken:** none on the lane. `sensei_test` template was stale (rebuilt with `make test-template`). Whole-suite
e2e runs need explicit spec paths.
