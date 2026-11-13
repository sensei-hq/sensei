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

**Remaining (in order):** #418 slice 3 (`/relay/now` lanes gain current_step/steps_done/steps_total) → 4 (lane/task/run
events carry rows; item/decision/message entities; triage/backlog writes emit) → #419 (draft/start/cancel run, nudge,
catalogue) → #426 run-journey e2e.

**Next command:** red test for slice 3 in `crates/senseid/src/api/handlers/relay.rs` (`cargo test --bin senseid relay
-- --test-threads=1`).

**Open questions (user):** bench sheet feedback; retire `lane/relay-screen-conformance`?; AddItemForm design;
phone header height; merge #417 + `make release`; `font-display` no-op elsewhere in the app.

**Known broken:** none on the lane. `sensei_test` template was stale (rebuilt with `make test-template`). Whole-suite
e2e runs need explicit spec paths.
