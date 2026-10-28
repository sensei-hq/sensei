# Checkpoint — 2026-10-28 (stability and consistency; e2e back to green)

**Slice:** stability + consistency (user, 2026-10-28) — one sidebar, identities from one state, the
state/view audit, then `make test-app-e2e` green and `make install` verified live.

**Done (develop, pushed):** three sidebars are one Rail (d4d9039c); Workspaces list query 29.6→1.2 ms
(f5f9a631); identities from one PersonaList (b60ea672, 09703055); audit items 1–15 (f91d884d); "Notify me" in
the relay inbox header (5206abfb); Settings · Dōjō says an empty sync list in words, tones from state
(f500d1dc); e2e specs follow the app — data-screen="workspaces", Rail markup, 220px, #343 copy, the bogus
/memories a11y route removed (c13568e0).

**rokkit develop (pushed, NOT released):** 4fd0cfeb primary/accent read as text by day (zen-sumi shu 600,
others 700; 500 by night) and on-primary follows the painted fill → white labels on vermillion by day;
5973a45c zen-sumi List resting items ink-mute in dark mode; 65e80bcf `text-on-{role}-soft`. Minor → 1.12.0.

**Next:**
1. User: `! cd ~/Developer/rokkit && bun run bump minor` (release is blocked for me).
2. Bump app/dōjō/website to rokkit 1.12.0; replace `text-accent` on `bg-accent-soft` (insights filter chip,
   MemoryChip, dojo-artifacts, upgrades pills, DatapointDrilldown) with `text-on-accent-soft`; verify
   `/__uno.css` 200 with no `${`.
3. `make test-app-e2e` → green (a11y was the last red group); then `make install` and verify live.
4. The Settings screen walk.

**Open:** rokkit main merge (publishes the guides site) is the user's. stash@{0} (abandoned daemon
identity_status tests) can be dropped. #282 waits on the Apple Developer ID.

**Known broken:** Keychain tests fail on rebuilt binaries (#282). CI qlty workflows disabled (minutes).
