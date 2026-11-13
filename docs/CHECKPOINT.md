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
- #419 slice 1 nudge: red `544a62ef7`, green `88016e584` — `POST /relay/messages` takes `nudge`+`reason`
  (+`waiting_min`), `NUDGE_REASONS` held by the props check, history shows the nudge. Relay tests 386/386.
- #419 slice 2 codenames: red `2cafda34b`, green `ebe37a476` — register mints `colour-star` (unique per crew),
  served by register, now, task view `task.lane`/`lanes[]`; TS types gain it. Relay 392/392, app relay 846/846.
- #419 slice 3 live lanes: red `45892a03c` → green `41dbad77a` (relay.now drops unregistered lanes + closed crews,
  lane `presence`); state red `65a6facaa` → green `550a8600e` (`isNotResponding` reads it everywhere). Relay 394/394,
  app 3603/3603, state 86/86. Live DDL NOT yet applied; install batched to the end of #419.
- #419 slice 4 run lifecycle: red `8ef273e72` → green `5f73e0800` (draft/start/cancel, `run_state` draft, runs.crew_id
  nullable + repo_key/workspace_id; smoke draft→start PASS on a scratch broker,
  `coord/reports/relay-screen/daemon-419/`); state red `d507ce3da` → green `547148d6c`. Relay 399/399, app 3604/3604.
- #419 slice 5 run list/detail: red `31b872091` → green `64e8307dc` (`relay.run_list` view, `GET /relay/runs`,
  `GET /relay/runs/{id}`; archive schema + daily purge NOT built). Relay 401/401. Client methods wait for the screen.

**Remaining (in order):** #419 slices 6–8 (person crew close · item delete/patch + ticketed backlog · catalogue) → #426 run-journey e2e. Gaps outside
#418: `task.waiting`; person-message read ladder. Lane is NOT merged to develop (user's call).

**Next command:** #419 slice 6 — red tests: a person route closes a crew (`POST /relay/crews/{id}/close`), its
lanes leave Now on the stream. DB tests: `TEST_DATABASE_URL=postgresql://localhost:5432/sensei_test_relay_screen`
(rebuild with the scratchpad `lane-template.sh` after DDL changes; other lanes restamp `sensei_test`).

**Open questions (user):** bench sheet feedback; retire `lane/relay-screen-conformance`?; AddItemForm design;
phone header height; merge #417 + `make release`; `font-display` no-op elsewhere in the app.

**Known broken:** none on the lane. `sensei_test` template was stale (rebuilt with `make test-template`). Whole-suite
e2e runs need explicit spec paths.
