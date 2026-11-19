# Checkpoint — 2026-11-19 (#455 slice 1 — catalogue, session host, AddMember picker)

**Slice:** `add-relay-assistant-catalogue` slice 1 (#455). Branch `lane/relay-455-s1`
(worktree `lanes/relay-open`).

**Done (slice 1 complete: tasks 1.1–1.6, 2.1–2.6, 3.1–3.3, 4.1–4.4):**
- Seeded `sensei.assistant_catalogue` (13 tools) + `assistant_installs` + `activity.sessions.host`.
- `sync_installs` stores this machine's installs; `GET /api/assistants/installs`; boot sync when empty.
- Every transcript adapter declares `host()`; ingest writes it; model attribution by model, not host.
- `GET /relay/catalogue`; relay reads serve `family_name` / `kind` from one `CatalogueNames` helper.
- AddMember picks from the catalogue (pressed-state lists, error Notice with Retry); preview fixtures
  captured across the matrix.
- 1.5: the wizard's assistants stage stores installs on entry; Settings → Assistants says "detected
  <when>" per tool and offers Detect again. The parked wip commit `912af748` is completed by the
  red `test(app)` + green `feat(app)` commits after it.

- Review fix (PR #506): detection no longer stamps `configured_at`/`configured_version`; only configure
  (its successes) and an upgrade that re-pushes the plugin do (`sync_installs(pg, just_configured)`,
  `upgrade_refreshing`). Red `4d10703d`/`74308194`, green `16326351`.
- Review fix 2 (PR #506): a failed install re-detect after remove/upgrade is a warning (remove answers
  `warnings`; upgrade reports gain `warnings`, `ok`/`errors` untouched), so the wizard no longer reverts a landed
  removal; a family card names its installed members ("Claude Code and Claude Desktop"). Same class fixed: the
  CLI read `acps_removed` from `/api/assistants/remove` (daemon sends `assistants_removed`) and dropped upgrade
  warnings; TS `RemoveResult` named the installer's field wrongly. Red `02fdde62`, green `5a0c5c66`;
  configure-assistants nav bound 10s→20s `40d0c0df` (Continue measured 11.4s).
- Review fix 3 (PR #506): the catalogue was read once per page (gated on `!catalogue && !catalogueError`); now
  every opening of the Crew view reads it (`RelayScreen.svelte`, a non-reactive `crewOpen` marks the opening, so
  a crew read while Crew stays open does not re-read). Red `ead334a4`, green `5078cf98`; relay-catalogue e2e
  leaves Crew for Queue, seeds a later Claude session, and sees the new count on return `e1c2784d`. Same class
  checked: the runs list re-reads on run events (`relay-state.svelte.ts:309`); no other one-shot read in the slice.
  All 8 relay* e2e specs 15/15 green on the fix (one earlier run had a transient webview `eval` hang from test 6
  on; the failing specs passed alone, in order, and on the full rerun).
- e2e green: relay-catalogue, assistants-configure (switch names the installed members), configure-assistants,
  every relay*/settings spec; observatory-daily now gates on the rows Today draws.
- Gates: fmt, clippy -D warnings (senseid, sensei-mcp), senseid DB tests serial, `make test-db` 102/102.

**Remaining:** PR re-review + merge to develop. Slice 2 (tasks 5.x) and close-out 6.x are separate.

**Next command:** slice 2 — `openspec/changes/add-relay-assistant-catalogue/tasks.md` §5.1.

**Open questions:** none.

**Known broken:** none from this branch. sensei_e2e needs `dbd reconcile` (twice) + `dbd import` from
`database/` after this DDL: the e2e daemon's converge is gated on the version, which did not change. The preview rig's phone captures draw its sticky nav chips over
every fixture (pre-existing, rig-wide).
