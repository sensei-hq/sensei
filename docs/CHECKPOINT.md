# Checkpoint — 2026-11-12 (lanes/relay-screen on lane/relay-screen-rebuild, from develop @ d19c71807)

**Slice:** Relay screen rebuild to mockup r1 — one issue at a time, each component verified against the mockup
(desktop + phone × light/dark × kanji/icon) on the bench before integration (user, 2026-11-12). Design:
`docs/design/2026-11-12-relay-screen-rebuild.md`; issues #418–#429 (#429 = the rig).

**Done:**
- #429 rig: `app/e2e/mockup/render.mjs` (mockup per state, crops), `/gallery` (DEV-only, `src/lib/relay/gallery.ts`
  catalogue), `bun run gallery:capture` + `gallery:sheet [--embed]` (matrix + review sheet), boot-gate exemption
  red/green (`edc310174` → `42b509508`), sheet as a review page (`cad344c42`). Bench r0 (today's 10 components, 21
  fixtures): `coord/reports/relay-screen/bench-r0/index.html`; published page for the user's review.

**Remaining (in order):** #420 Relay on the shell (ShellScreen height fix in `packages/ui`, toolbar view icon buttons
with badges, stats band, schema filters, info band in four states — each through the bench first) → #421 state
package → #422 Now → #423 Queue → #424 Crew → #425 Task → #426 the e2e run. Daemon #418/#419 can run in parallel.

**Next command:** in `lanes/relay-screen/app`: write the red spec for `src/lib/relay/relay-shell.ts` (state →
ShellScreen props: toolbar views + badges, stats, info slots, footer, context) and a `RelayShell` bench fixture, then
`bun run gallery:capture && bun run gallery:sheet --embed`.

**Open questions (user):** feedback on bench r0 (the published page) — which deltas matter first.

**Known broken:** none on the lane. The gallery's QueueRow/DecisionRow look differs from the mockup by design today —
that is the work.
