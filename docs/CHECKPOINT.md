# Checkpoint — 2026-10-30 (Diagrams: review fixes; rokkit zoom/icon pending release)

**Slice:** the user's diagram review (2026-10-30), then #347 attribution, #348 Muse, #349 Freebuff.

**Done (develop, pushed, installed):**
- e1b1d714a — Zones abstractness counts declared interfaces/traits (pain 26 → 15 on sensei).
- Header per the updated mockup: level tag, four toggles (info three columns, stats as cards,
  controls band, side panel), all remembered; side panel and diagram scroll on their own.
- Takeaways as a rokkit List accordion; zone filter as a rokkit Toggle in the controls row.
- 0d828c0f7 — Rail header/footer fixed, nav scrolls; one-row controls, shared first, Find a module
  (filterable rokkit Select by group); matrix sticky headers + gridlines; complexity/cycle tiers as
  borders; Database · ER: name + type filters, schema Select, rokkit Tree with schema colours,
  entity Details | Diagram.
- Live inspect green with the installed app quit (memory: quit-sensei-before-inspect).

**rokkit develop (pushed, NOT released):** d9cbf302 dark divider; 874cd28fe zoom top-left by
default + `position`/`zoomPosition`, DiagramFrame zoom slot, view glyph split-view.
**Next after the user's rokkit release:** bump app/dōjō/website; ZoomFrame → `<ZoomControl
position="top-left">` on its canvas (drop the overlay wrapper).

**Waiting on the user:** Communities rule (recommend dominant community, small ones folded);
design MCP connection (`claude mcp add … claude_design`, /design-login) for the two design links.

**Later:** dbd ObjectView/OverviewView → @rokkit/graph/schema; enums/triggers/sequences/modules in
the schema payload; Changelog after dbd release 1 (user, 2026-10-30).

**Known broken:** Keychain test on rebuilt binaries (#282).
