# Checkpoint — 2026-10-28 (evening: live-data fixes and the diagrams/sessions walk)

**Slice:** stability + consistency, then the user's 2026-10-28 evening list (dbd workspace).

**Done (develop, pushed):** e2e 136/136 on a fresh bundle; `make install` live. Live-data fixes:
Settings · Dōjō froze on an identity paired with two dōjōs (95a6d95f); settings window has a main
landmark + harness fixes (5e145495); Settings · Libraries filtered by a stale folder snapshot
(d7e4f56b); navigation waits out slow loads + schema gate derives its counts (eb4c5d2b); a failed
settings read keeps the last settings, shows it in the status strip, retries (e6b154d3); bands are
vertical bars with clean dates, Sessions opens on recency 7/30/90 (bf528427); Layers has rokkit's
zoom overlay (2a9c3ba0); Cycles draws each loop (08a0fcdf); Structure caches each level·kinds read
(6dbcc4a1). Website split filed as #346.

**rokkit develop (pushed, NOT released):** daef55a0 etched dark paper-edge (dividers); af4701a2 pinch
zoom proportional to wheel travel; db3ae1f5 cli bundle/build await every folder (the 1.12.1 bump
failure). Next: user runs `bun run bump patch` → bump app/dōjō/website to 1.12.1.

**Open / to ask:** dbd title-bar "scribble" — not reproduced (no graphic near the titlebar on
overview, sessions, structure, layers, cycles); need a screenshot. Bands' minutes are wall-clock
(dbd has 20h–2d sessions) — chart active time instead? Structure's per-kind unplaced count needs a
daemon change before kinds can filter client-side. Live inspect still red: layers crawl blocks
>30 s on large graphs (rokkit layout perf); tool-calls index freshness (recheck after re-index);
glyphs spec waits for a kanji on a workspace overview that may never render one.

**Known broken:** Keychain tests on rebuilt binaries (#282). CI qlty workflows disabled. sensei MCP
checkpoints 404 in this long session (stale sensei-mcp; `/mcp` reconnect).
