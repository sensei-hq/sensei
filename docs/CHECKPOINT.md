# Checkpoint — 2026-10-26

**Slice:** the user's demo build (sensei + dōjō): libraries → instruments → diagrams, then a shared
diagrams library for app and dōjō with the dōjō diagram sync, then the kanji/icons setting (#133).

**v0.21.0** fully released: `deploy-dojo` and `deploy-worker` approved and green.

**Done today (develop, pushed; daemon installed locally from develop):**
- #341 closed: libraries register from `sensei.library.json` in the real scan; dependencies are discovered
  from their GitHub repository at the pinned ref (`discover_library`, new enum value, reconciled live).
  Live: rokkit 15 packages / 95 pages / 5 skills / 3 agents, dbd 2/50/1/1, kavach 13/15/4/2. #342 filed
  (site `.well-known` discovery).
- Instruments: Playground = discovered servers, tools from each server, schema-built forms, Execute through
  the server (`/api/instruments/mcp-servers/{id}/call`); `/api/mcp/tools` retired; MCP refuses a missing
  required arg; Health no longer draws a server twice (that blanked the grid). `test:inspect -- libraries
  instruments-store` green (3/3).

**Next:** Diagrams — the Code map per `docs/mockups/Sensei/docs/prd-drafts/M17-code-map.md`: L0 modules
(≤ 60, group bands, "+N more", hub lane) → L1 files by role → L2 symbols; lenses on one canvas; what the
daemon, indexer (`module_of`, file roles) and `@rokkit/graph` need. Start by measuring what the current
Structure/World/Atlas views draw on sensei.

**Open (the user's):** #133's two decisions (dōjō default glyph mode; workspace glyphs in icon mode).

**Known:**
- Keychain tests (`gateway_keys`, `dojo_client::session` discard) fail on rebuilt test binaries: test-owned
  items partitioned by an older cdhash (#282). Environmental, not code.
- After a DDL change run `make test-template`; fmt + workspace clippy before every Rust commit.
- Daemon reinstalls revoke keychain approval (once per identity).
