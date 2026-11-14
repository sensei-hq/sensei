# Checkpoint — 2026-11-14 (lanes/relay-screen on lane/relay-screen-rebuild, develop merged in @ d747d0a87)

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

- rokkit #180 + #181 merged, **v1.15.4 published**; app + dōjō bumped (`3cd4a8cac`); relay toolbar = rokkit kind/
  count, both relay CSS overrides + `@sensei/ui` ToolbarItem deleted (`ac4640baf`); Working accent (`035b8aeac`);
  relay Pill/Tag → rokkit Pill (`942488387`). Glyphs: #406 = repurpose `@sensei/glyphs` (comments added).
- Consolidation after the power loss (`0cf5bc480`): develop (#284, #359, #415, #435, zizmor) merged INTO this lane,
  not the lane into develop — release PR #417 is open and this slice is unfinished. `shot()` gained `unavailable`
  in `e2e/lib/shot.ts`; ratchet 1,197 (measured 1,195). `lane/relay-screen-conformance` retired (superseded, §5).

**Remaining (in order):** shared-component catalogue `docs/spec/2026-11-14-shared-component-catalogue.md` —
awaiting the user's agreement · TaskHistory → rokkit Timeline · lists → rokkit List (no-button rows) · StatCard
fixtures · #419 wrap-up (guidance v2, live DDL incl. now.ddl `last_nudge`, install, e2e) → #426.

**Next command:** the Timeline migration (red-first), once the catalogue is agreed.

**Open questions (user):** the catalogue spec; host tag needs a Pill `title`; merge #417 + `make release`
(then this lane → develop when finished).

**Known broken:** CONTRACT §15 auto-nudge for non-review waits not built (`task.waiting` gap). Dev Vite binds IPv6
`localhost` only (rig uses `localhost`). Lane test DB: re-apply a changed view by hand (self-apply skips).
