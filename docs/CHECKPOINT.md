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

- e2e green: relay-catalogue, assistants-configure (switch now "Enable Claude Code"), configure-assistants,
  every relay*/settings spec; observatory-daily now gates on the rows Today draws.
- Gates: fmt, clippy -D warnings (senseid, sensei-mcp), senseid DB tests serial, `make test-db` 102/102.

**Remaining:** PR review + merge to develop. Slice 2 (tasks 5.x) and close-out 6.x are separate.

**Next command:** slice 2 — `openspec/changes/add-relay-assistant-catalogue/tasks.md` §5.1.

**Open questions:** none.

**Known broken:** none from this branch. sensei_e2e needs `dbd reconcile` (twice) + `dbd import` from
`database/` after this DDL: the e2e daemon's converge is gated on the version, which did not change. The preview rig's phone captures draw its sticky nav chips over
every fixture (pre-existing, rig-wide).
