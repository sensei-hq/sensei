# Checkpoint — 2026-11-19 (#455 slice 1 — catalogue, session host, AddMember picker)

**Slice:** `add-relay-assistant-catalogue` slice 1 (#455). Branch `lane/relay-455-s1`
(worktree `lanes/relay-open`).

**Done (slice 1 complete: tasks 1.1–1.6, 2.1–2.6, 3.1–3.3, 4.1–4.4):**
- Seeded `sensei.assistant_catalogue` (13 tools) + `assistant_installs` + `activity.sessions.host`.
- `sync_installs` stores this machine's installs; `GET /api/assistants/installs`; boot sync when empty.
- Every transcript adapter declares `host()`; ingest writes it; model attribution by model, not host.
- `GET /relay/catalogue`; relay reads serve `family_name` / `kind` from one `CatalogueNames` helper.
- AddMember picks from the catalogue (pressed lists, error Notice + Retry); preview fixtures captured.
- 1.5: the wizard's assistants stage stores installs on entry; Settings → Assistants shows "detected <when>".
- Review fix 1 (PR #506): only configure / a re-pushing upgrade stamps `configured_at`/`_version`. `16326351`.
- Review fix 2: a failed re-detect after remove/upgrade is a warning; family cards name installed members;
  CLI `assistants_removed` + upgrade warnings; TS `RemoveResult`. Red `02fdde62`, green `5a0c5c66`.
- Review fix 3: every opening of the Crew view reads the catalogue (was once per page). Red `ead334a4`, green
  `5078cf98`, e2e leave-and-return `e1c2784d`. Runs list re-reads on run events; no other one-shot read.
- e2e green: relay* 15/15 (one earlier run had a transient webview `eval` hang; passed alone and on rerun),
  assistants-configure, configure-assistants, settings, observatory-daily.
- Gates: fmt, clippy -D warnings, senseid DB tests serial, `make test-db` 102/102.

**Remaining:** PR re-review + merge to develop. Slice 2 (tasks 5.x) and close-out 6.x are separate.

**Next command:** slice 2 — `openspec/changes/add-relay-assistant-catalogue/tasks.md` §5.1.

**Open questions:** none.

**Known broken:** none from this branch. sensei_e2e needs `dbd reconcile` (x2) + `dbd import` after this DDL.
Preview rig phone captures draw sticky nav chips over every fixture (pre-existing, rig-wide).
