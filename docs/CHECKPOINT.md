# Checkpoint — 2026-11-13 (lanes/relay-screen on lane/relay-screen-rebuild, from develop @ d19c71807)

**Slice:** Relay screen rebuild to mockup r1 — one issue at a time, each component verified against the mockup
(desktop + phone × light/dark × kanji/icon) on the bench before integration (user, 2026-11-12). Design:
`docs/design/2026-11-12-relay-screen-rebuild.md`; issues #418–#429 (#429 = the rig).

**Done:**
- #429 rig (`/gallery`, `gallery:capture`, `gallery:sheet --embed`); review sheet = artifact 3869a0ea.
- #420 shell + filters + footer; #421 `@sensei/state` + both pages cut over; #422 Now; #423 Queue (+add-to-triage
  form, not in mockup); #424 Crew; #425 Task view (#380/#381/#388 fixed; relay hooks no longer collide with rokkit).
  Each benched vs mockup (`coord/reports/relay-screen/bench-r1..r7`), relay e2e 8/8 against the real daemon.
- #418 DONE on the lane (commits listed on the issue); live check `coord/reports/relay-screen/daemon-418/`.

- LaneRow step progress (not in the mockup): selector `3ca277909`, component `9193b3fe2`, bench `008575ac0`
  (`bench-r8`; sheet artifact 3869a0ea v10). Approved by the user on desktop and phone (2026-11-13).
- #419 slices 1–6 green, each red-first (commits in `git log`): nudge · codenames · live lanes + presence (daemon +
  state) · run draft/start/cancel (daemon + state; smoke draft→start PASS, `coord/reports/relay-screen/daemon-419/`) ·
  run list/detail (`relay.run_list`; archive schema + daily purge NOT built) · person crew close · item close/
  reorder/retitle + ticketed backlog (`d7c243d38`, clippy fix `b43450572`; GitLab tickets wait for a `gitlab`
  provider). Relay 406/406, app 3604/3604. Live DDL NOT applied; install batched to the end of #419.

**Remaining (in order):** #419 slice 8 (catalogue) + wrap-up (guidance v2 text, live DDL, install, e2e) → #426 run-journey e2e. Gaps outside
#418: `task.waiting`; person-message read ladder. Lane is NOT merged to develop (user's call).

**Next command:** #419 slice 8 — red tests: the AddMember catalogue (CONTRACT §3l): assistants/editors seen on this
machine with launch template or paste line, or first cut the two paste lines with the crew id. DB tests: `TEST_DATABASE_URL=postgresql://localhost:5432/sensei_test_relay_screen`
(rebuild with the scratchpad `lane-template.sh` after DDL changes; other lanes restamp `sensei_test`).

**Open questions (user):** bench sheet feedback; retire `lane/relay-screen-conformance`?; AddItemForm design;
phone header height; merge #417 + `make release`; `font-display` no-op elsewhere in the app.

**Known broken:** none on the lane. `sensei_test` template was stale (rebuilt with `make test-template`). Whole-suite
e2e runs need explicit spec paths.
