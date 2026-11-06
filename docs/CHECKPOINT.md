# Checkpoint — 2026-11-06 (coordinator; lanes in ~/Developer/sensei-hq/coord/TRACKER.md)

**Slice:** Relay slice 1 (D90) — `relay` schema (L4) → daemon + MCP tools (L3) → resource lease (L3); Relay screen (L5,
merged); onboarding lines (L0). Billing is low priority.

**Done:**
- Relay screen replaces Intake (2898b4157; L5 r2, reviewed by L4/L6; F2 wordings ratified).
- rokkit 1.15.2 (ScreenShell/Toolbar attribute passthrough, L7) in app + dōjō (2ef07a55b).
- dōjō `@rokkit/app` resolution fix (e1d724a14): dōjō check + 40 test files were red since 4aec5a688.
- dbd 0.27.9 released: a call never resolves to a relation (sensei-hq/dbd#112, PR #113); CI test job pinned to
  ubuntu-24.04 (26.04 lacks libxml2.so.2).
- Gates at 2ef07a55b: app check 0 / 3196 tests; dōjō check 0 / 2279 tests; make test-fast green.

**Remaining:**
- L4 relay schema r2: B1 `current_timestamp` (no `now()` in relay DDL) + `dbd inspect` 0 errors both scopes, S1
  search_path, session-local-day window, policy count line → L6 reviews → L0 rehearses on a clone, converges both local
  DBs twice, merges.
- L3: run.sh refuses the live `sensei` DB (tests first) → relay part 2 (pool TimeZone = the Mac's) → resource lease →
  bump the daemon's dbd-core pin v0.27.8 → v0.27.9.
- L5: relay follow-ups (`coord/reviews/L5/relay-screen-followups.md`); diagrams-shell phase B after L3 approves tests r2.
- L6: sensei-hq/dbd#114 (tests spawning git clear GIT_* env) while no review is pending.
- L0: onboarding lines (slice 1 item 5); dbd develop → main PR.

**Next command:** read `coord/reports/L4/` newest (relay-schema-impl-r2) and run `dbd inspect` + `--scope dojo` in
`lanes/L9/database`.

**Open questions (user):** billing edge — a future-start Pro or ended trial shows "once Pro is renewed".

**Known broken:** none on develop. Production lacks per-row confirm and refuses `hidden_coupling_pairs` until the next
sensei release.
