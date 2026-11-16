# Change: Phone layout across the app — mobile-first with UnoCSS `sm:`/`md:` (#420)

## Why
At phone width (~430px) the app is unusable in most places. A survey (2026-11-16) found ~40
layouts that break; only 18 app `.svelte` files use any `sm:`/`md:` class. The worst causes are
shared: every app window keeps a fixed 220px sidebar beside the content (~210px left for the
page), and rokkit's ScreenShell is desktop-first (3-column info band and 12-column stats that
collapse only under `max-width` queries; stat labels never wrap — "Completed to…"), so Relay's
header bands take ~1100px before its body. The user's direction (2026-11-16): use UnoCSS `md:`
/ `sm:` variants to adjust layout and wrapping for phone — no separate phone components.

## What Changes
- **Rule:** every layout is mobile-first — the unprefixed classes are the phone layout; `sm:`
  (≥640px) and `md:` (≥768px) adjust upward. No `max-*:` variants, no layout `@media` or
  `<style>` layout blocks in components.
- **Shells:** the shared `Rail` sidebar (observatory, workspace, settings windows) becomes a
  drawer below `md:` (closed by default, opened from the header), static at `md:` and up — the
  same pattern as the dōjō's NavPane. The setup wizard's 260px rail stacks on phone.
- **rokkit ScreenShell / StatCard (fix rokkit, not per screen):** 1 column first with `min-width`
  steps up; the info band starts collapsed on phone; stat cards 2-up on phone; stat labels wrap.
  Released as a rokkit patch; app and dōjō bumped.
- **Screens** (each: mobile-first grid/flex, wrap where text must not be cut, wide tables in
  `overflow-x-auto`): observatory home, instruments, impact, consolidation, insights,
  workspaces, activity logs, learnings, libraries, knowledge sources; workspace impact,
  memories, overview, instruments, about; settings general, metrics, inference, instruments;
  health logs; setup scan/welcome/done; Relay LaneRow (pill no longer wraps under the name),
  RunCard and RelayRunScreen facts; dōjō RelayDetail padding, DojoSignIn, OrgSwitcher width.
- **Guard:** a check that fails on a new unprefixed multi-column layout wider than a phone
  (`grid-cols-N` N≥3, fixed `grid-cols-[NNNpx_…]`, `w-[≥300px]`) in app/dōjō components, with
  today's count as an allowlist that only shrinks (catalogue Order step 2).

## Impact
- Specs: `app-layout` (new capability).
- Code: rokkit `packages/themes` (screen-shell, stat-card) + `packages/ui` (ShellScreen); `app/src`
  (Rail and layouts, ~30 screens/components); `dojo/src` (3 components); a guard script.
- Verification: the `/preview` capture matrix at phone width for Relay; a phone-width e2e pass
  over the main windows (the app's e2e at 430px) — no horizontal scroll on the page body.
