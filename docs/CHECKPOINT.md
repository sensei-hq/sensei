# Checkpoint — 2026-10-27

**Slice:** the demo build (sensei + dōjō): Code map (M17) from the shared `packages/codemap`, then its
dōjō embed; #133 glyphs (user decisions 2026-10-27: one rule — kanji mode kanji, icon mode icons,
everywhere; default kanji; the preference syncs).

**Done (develop, pushed; daemon installed locally):**
- Code map: daemon L0/L1/L2 endpoints; `module_of` splits container folders; `@sensei/codemap`
  (state, layout, Structure/Cycles/Matrix lenses, list mode under 640px, 16 specs); the app's
  workspace Diagrams opens on it (`test:inspect -- codemap` green: 60 blocks, drill to symbols, back).
- #133: one rule in both apps (no `icon={null}` opt-outs; `KIND_ICONS`); sync via
  `dojo.user_preferences` + `/v1/you/preferences`, the account menu, and the daemon's sync tick
  (`ui.glyphs.set_at`, later choice wins). Local dōjō reconciled; access test 62/62.

**Next:**
1. Code map lenses still missing: Zones (daemon endpoint exists), Wheel, Hidden coupling (needs
   co-change from `commit_files`), Complexity (needs stored complexity); retire World and Atlas (M17).
2. The dōjō Code health tab: a dōjō port for `@sensei/codemap` over the graph it already syncs.
3. A visual look at the Code map: the inspect harness's screenshot is blank (the bundle lacks Screen
   Recording permission) — the user eyeballs it in the app.

**Open (the user's):** none.

**Not live yet:** the glyph sync needs `dojo.user_preferences` in production (next release's
`deploy-dojo`).

**Known:** Keychain tests fail on rebuilt test binaries (#282, environmental). After a DDL change run
`make test-template`; reconcile live with dbd. fmt + workspace clippy before every Rust commit.
