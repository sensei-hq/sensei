# Checkpoint — 2026-11-12 (lanes/relay-screen on lane/relay-screen-rebuild, from develop @ d19c71807)

**Slice:** Relay screen rebuild to mockup r1 — one issue at a time, each component verified against the mockup
(desktop + phone × light/dark × kanji/icon) on the bench before integration (user, 2026-11-12). Design:
`docs/design/2026-11-12-relay-screen-rebuild.md`; issues #418–#429 (#429 = the rig).

**Done:**
- #429 rig: mockup renderer, `/gallery` (DEV-only), `gallery:capture` + `gallery:sheet [--embed]`. Bench r0.
- #420 part 1: `RelayShell` + `relay-shell.ts` selector on the bench (`8a67b5c9f`); ShellScreen h-full/pinned in
  `packages/ui`; scoped toolbar partial `relay-toolbar.css`.
- #420 part 2: both relay pages on RelayShell; `toShellInput` counts stats from the read (red `47806b5ef`, green
  `afd37d1d0`, pushed). Bench r1: `coord/reports/relay-screen/bench-r1/`; review sheet republished (artifact
  3869a0ea, version 3).

**Remaining (in order):** #420 schema filters via `@rokkit/forms` (FilterSchema ↔ URL narrowing) and the four-state
e2e pins (harvest `relay-conformance.spec.ts` + `app/e2e/lib/psql.ts` from `lane/relay-screen-conformance`), then
close #380 #383 #384 #385 #388 #394, supersede #386 → #421 state package → #422 Now → #423 Queue → #424 Crew →
#425 Task → #426 e2e run. Daemon #418/#419 in parallel.

**Next command:** in `lanes/relay-screen/app`: `git show lane/relay-screen-conformance --stat` to harvest the
conformance spec, then write the red spec for the schema filter band.

**Open questions (user):** feedback on the bench sheet (r1) — which deltas matter first.

**Known broken:** none on the lane. The phone bench frame is fixed-height, so the stats band sits below its crop.
