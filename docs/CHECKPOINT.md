# Checkpoint — 2026-10-28 (stability and consistency; rokkit 1.12.0 adopted)

**Slice:** stability + consistency (user, 2026-10-28) — one sidebar, identities from one state, the
state/view audit, then `make test-app-e2e` green and `make install` verified live.

**Done (develop, pushed):** three sidebars are one Rail (d4d9039c); Workspaces list query 29.6→1.2 ms
(f5f9a631); identities from one PersonaList (b60ea672, 09703055); audit items 1–15 (f91d884d); "Notify me" in
the relay inbox header (5206abfb); Settings · Dōjō says an empty sync list in words (f500d1dc); e2e specs follow
the app (c13568e0, 280f4db9); rokkit 1.12.0 in app/dōjō/website (738d01ca) — white labels on vermillion by day,
primary/accent text AA, quieter dark sidebar; 96 chips draw tint text with text-on-{role}-soft, guarded by
rokkit-defaults.spec across the three frontends, dōjō K2_NODE gains onSoft (604099b1).

**e2e green:** full suite on a fresh bundle — 136 passed, 0 failed, 22 skipped (conditional on data)
(a75f9aa6: instruments specs declare the sensei MCP they test; log chip text-on-error-soft).
**In flight:** `make install` (release), then the live check below.

**Next:**
1. e2e green → `make install` (one more Keychain prompt) → verify live: Workspaces load, identities overlay
   and Settings agree, the three sidebars, the workspace window opens with content, white primary labels.
2. The Settings screen walk.

**Open:** rokkit main merge (publishes the guides site) is the user's. stash@{0} (abandoned daemon
identity_status tests) can be dropped. #282 waits on the Apple Developer ID. sensei MCP checkpoints 404 in
this long session (stale sensei-mcp; `/mcp` reconnect).

**Known broken:** Keychain tests fail on rebuilt binaries (#282). CI qlty workflows disabled (minutes).
