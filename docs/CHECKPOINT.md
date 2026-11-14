# Checkpoint — 2026-11-14 (lanes/relay-screen on lane/relay-screen-rebuild, from develop @ d19c71807)

**Slice:** Relay screen rebuild to mockup r1, each component benched against the mockup (desktop + phone ×
light/dark × kanji/icon). Design `docs/design/2026-11-12-relay-screen-rebuild.md`; issues #418–#429 (#429 = rig).
User ruling 2026-11-14: one shell — fix rokkit, never patch per screen; reuse rokkit components over local copies.

**Done (2026-11-14, red-first):**
- rokkit `fix/zen-sumi-shell-parity` → **PR sensei-hq/rokkit#180, awaiting the user's review** (not merged, not
  published): font utilities resolve (`font-display`/`font-ui` emitted NOTHING; sans/mono read system stacks);
  stat cards = mockup (display font 24, mute pressed fill, edge frame — user ruling, not 3:1; toned icon; accent
  tone; 1-line description); toolbar `kind` + `count` badge; zen-sumi title-bar 36px framed buttons; filter row (#178).
- Nudge end to end: `now` task `last_nudge {at, by}` (`44c729251`); WaitList Nudge {party} → `RelayState.nudge`,
  "Nudged just now · by you", Sensei's beside a long wait (`63ef0f1b7`). Two stale SQL pins fixed (`4d6c6a2a7`).
- `/preview` replaces `/gallery` (`b4fead1c4`): index, `/preview/<Component>` with fixture tabs + view toggles,
  `/preview/all`; wholly dev-only (verified absent from `build/`). Rig: `preview:capture|sheet|scroll`.
- Scroll test (`9eb7cd209`): RelayShell fixtures overflow inside the app's `<main>` chain; `preview:scroll` green at
  wide + phone; mutation-checked (a stray wrapper fails it).

**Remaining (in order):** user reviews rokkit#180 → merge + publish → bump app `@rokkit/*`, delete
`relay-toolbar.css`, `relay-filters.css`, sensei `toolbar-item.js` helpers; Working stat tone `accent`; relay
toolbar via `kind`/`count` · rokkit-first pass (Pill/Tag, lists → rokkit List, TaskHistory → rokkit Timeline,
StatCard fixture, toolbar separator) — plan pending the user · app font tokens → `font-display`/`font-ui` · #419
wrap-up (guidance v2, live DDL incl. now.ddl `last_nudge`, install, e2e) → #426.

**Next command:** `gh pr view 180 -R sensei-hq/rokkit` (review state), then the rokkit-first plan once agreed.

**Open questions (user):** →| Relay mark needs a non-Solar glyph (amends the 2026-10-27 Solar-only rule)?;
rokkit#180 review; Pill vs Tag; retire `lane/relay-screen-conformance`?; merge #417 + `make release`.

**Known broken:** CONTRACT §15 auto-nudge for non-review waits not built (`task.waiting` gap). Dev Vite binds IPv6
`localhost` only (rig uses `localhost`). Lane test DB: re-apply a changed view by hand (self-apply skips).
