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

- #421 first cut: `packages/state` (`@sensei/state`) — base class, EventManager, results, relay types/client/fixtures,
  six entity states + composite RelayState over a RelaySource (SSE source; realtime seam + canned double), `toStats`,
  RelayTaskState; both relay pages cut over (`f193776e4`), old state/crew-stream retired. Relay e2e 8/8 incl.
  live-stream no-refresh specs. View row selectors land with #422–#425.

- #422 part 1: `toLaneGroups`/`toCompletedRows` (package) + SectionHeading, LaneList/LaneRow, CompletedList/CompletedRow
  benched vs mockup (bench-r3) and composed into Now (`c67c235cf`); Pill hook `data-relay-pill` + paper tint fix.

**Remaining (in order):** #422 rest (DecisionCard, WaitList/WaitRow + Nudge, triage line + Unassigned row,
`toNeedsYou`/`toWaitRows`, stat filters drive lists) → #423 Queue → #424 Crew → #425 Task (incl. #388,
verify #380) → #426 e2e run. Daemon #418/#419 in parallel (row-carrying events; #421's apply is ready for them).

**Next command:** render the mockup's Needs you (`node app/e2e/mockup/render.mjs … --state '{"view":"now","load":"loaded","pending":true}' --shot --crop 'section[aria-label="Needs you"]'`),
then red spec for `toWaitRows`/`toNeedsYou` and DecisionCard/WaitRow on the bench.

**Open questions (user):** feedback on the bench sheet (r2). Retire `lane/relay-screen-conformance`? (its r2/r3
commits live only there). Header height: with the info band open, the header bands push the body below the fold
(phone ~1100px; a desktop window shows only "Needs you"); "Completed today" truncates in the stat card.

**Known broken:** none on the lane. Bare hooks rokkit's theme also styles: [data-field] [data-select]
[data-progress] [data-timeline] (CreateSprintForm, QueueRow, SprintBand, RelayTimeline) — fix with #423–#425. A whole-suite e2e run must use explicit spec paths: a bare `relay` filter ran
everything.
