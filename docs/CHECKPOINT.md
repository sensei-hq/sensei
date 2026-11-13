# Checkpoint — 2026-11-13 (lanes/relay-screen on lane/relay-screen-rebuild, from develop @ d19c71807)

**Slice:** Relay screen rebuild to mockup r1 — one issue at a time, each component verified against the mockup
(desktop + phone × light/dark × kanji/icon) on the bench before integration (user, 2026-11-12). Design:
`docs/design/2026-11-12-relay-screen-rebuild.md`; issues #418–#429 (#429 = the rig).

**Done:**
- #429 rig: mockup renderer, `/gallery` (DEV-only), `gallery:capture` + `gallery:sheet [--embed]`; shell frames
  shot whole on a phone (`2c5305088`).
- #420 complete on the lane: RelayShell on both pages (`afd37d1d0`), schema filters #384 (`9fa889983`), footer
  "not reported" + focusable shell sidebar (`d501e8ebc`), e2e state pins error/no-crew/loaded 3/3 against the real
  daemon (`3e6e74e52`). Covers #383 #384 #385 #394 (commented; close on merge). Evidence:
  `coord/reports/relay-screen/{bench-r1,bench-r2,e2e-r1}/`. Review sheet republished (artifact 3869a0ea v4).

**Remaining (in order):** #421 `@sensei/state` → #422 Now (incl. #386 lanes table) → #423 Queue → #424 Crew →
#425 Task (incl. #388, verify #380) → #426 e2e run. Daemon #418/#419 in parallel.

**Next command:** read #421 (`gh issue view 421`) and the design note's `@sensei/state` plan (§3), then write the
red spec for the shared relay state package.

**Open questions (user):** feedback on the bench sheet (r2). Retire `lane/relay-screen-conformance`? (its r2/r3
commits live only there). Header height: with the info band open, the header bands push the body below the fold
(phone ~1100px; a desktop window shows only "Needs you"); "Completed today" truncates in the stat card.

**Known broken:** none on the lane. A whole-suite e2e run must use explicit spec paths: a bare `relay` filter ran
everything.
