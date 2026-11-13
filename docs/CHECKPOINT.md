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

**Remaining (in order):** LaneRow step progress (current step, n of m) benched vs mockup → #419 (draft/start/cancel
run, nudge, catalogue) → #426 run-journey e2e. Gaps outside #418: `task.waiting`; person-message read ladder.

**Next command:** LaneRow step progress: render the mockup's lane rows, bench the row with current_step/steps,
then compose. The installed daemon is this branch's (`make install-debug`, 2026-11-13); relay e2e 8/8 against it;
live `relay.now` view applied (`dbd apply -n relay.now`).

**Open questions (user):** bench sheet feedback; retire `lane/relay-screen-conformance`?; AddItemForm design;
phone header height; merge #417 + `make release`; `font-display` no-op elsewhere in the app.

**Known broken:** none on the lane. `sensei_test` template was stale (rebuilt with `make test-template`). Whole-suite
e2e runs need explicit spec paths.
