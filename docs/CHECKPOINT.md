# Checkpoint — 2026-11-13 (lanes/relay-screen on lane/relay-screen-rebuild, from develop @ d19c71807)

**Slice:** Relay screen rebuild to mockup r1 — one issue at a time, each component verified against the mockup
(desktop + phone × light/dark × kanji/icon) on the bench before integration (user, 2026-11-12). Design:
`docs/design/2026-11-12-relay-screen-rebuild.md`; issues #418–#429 (#429 = the rig).

**Done:**
- #429 rig (`/gallery`, `gallery:capture`, `gallery:sheet --embed`); review sheet = artifact 3869a0ea.
- #420 shell + filters + footer; #421 `@sensei/state` + both pages cut over; #422 Now; #423 Queue (+add-to-triage
  form, not in mockup); #424 Crew; #425 Task view (#380/#381/#388 fixed; relay hooks no longer collide with rokkit).
  Each benched vs mockup (`coord/reports/relay-screen/bench-r1..r7`), relay e2e 8/8 against the real daemon.
- #418 slice 1/4: `GET /api/relay/events?crew_id=` drops events naming another crew (crew-less task/relay ops pass;
  bad id 400) (`0f11b8feb`); the task route opens its crew's stream (`d72406159`). senseid relay 376/376.

- #418 slice 2/4: a checkpoint's `steps[]` arrive as step events, then the task event (`8280d0422`).

- #418 slice 3/4: `relay.now` lane tasks carry current_step/steps_done/steps_total (`2d34546f1`).
- #418 slice 4a: lane events carry the lane's Now row (`b15ba05da`; `relay/rows.rs`, `relay_lane_row`); the app
  restates re-added rows, strips op/reason/lane_id, reads Now for an unseen crew (`5dba05a34`).

**Remaining (in order):** #418 4b run rows → 4c `decision` entity (ask/answer) → 4d `message` entity → 4e `item`
entity (triage/backlog/sprint writes emit) → 4f task rows → 4g app stops re-reading after writes → LaneRow step
progress on the bench → #419 → #426. Live check needs the new daemon installed (Keychain #283 caveat).

**Next command:** red test: run events carry the run's Now row (`crates/senseid/src/api/handlers/relay.rs`;
`cargo test --bin senseid relay -- --test-threads=1`).

**Open questions (user):** bench sheet feedback; retire `lane/relay-screen-conformance`?; AddItemForm design;
phone header height; merge #417 + `make release`; `font-display` no-op elsewhere in the app.

**Known broken:** none on the lane. `sensei_test` template was stale (rebuilt with `make test-template`). Whole-suite
e2e runs need explicit spec paths.
