# Checkpoint — 2026-10-26

**Slice:** the user's demo build (sensei + dōjō): libraries → instruments → diagrams (Code map, M17), then
the dōjō embed of the same Code map from a shared package; rokkit changes filed, built and released from
rokkit when the canvas needs them (user, 2026-10-26).

**Done (develop, pushed, daemon installed locally):**
- #341 libraries, #343 manual MCP registration, Instruments through the real servers — closed with evidence.
- Code map backend: `/api/workspaces/{id}/diagrams/codemap?groupBy=layers|folders` (L0),
  `/codemap/module?id=` (L1), `/codemap/file?id=` (L2); `analysis::codemap` is the pure core.
  `module_of` splits container folders; reconciled live. `/workspaces/{id}/graph` repos=0 fixed.

**Next:** `packages/codemap` — CodeMapState (level, path/breadcrumb, groupBy, role filter, lens,
selection), a data port the app and the dōjō each implement, the L0 band canvas, L1 role columns, L2
symbol view, lenses (Structure, Cycles, Matrix from rokkit's DependencyMatrix, Zones from the existing
endpoint), and a list mode under 640px. Wire into the app's workspace Diagrams (retire World/Atlas
entries per M17), then the dōjō Code health tab (needs the graph synced to the dōjō).

**Open (the user's):** #133 — the dōjō default glyph mode (recommend kanji) and workspace glyphs in icon
mode (recommend keep each workspace's inferred kanji).

**Known:** Keychain tests (`dojo_client::session` discard, `gateway_keys`) fail on rebuilt test binaries
(#282, environmental). After a DDL change run `make test-template`; reconcile live with dbd. fmt +
workspace clippy before every Rust commit.
