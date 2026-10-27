# Checkpoint — 2026-10-27

**Slice:** the Code map (M17) — app done; the dōjō's Code health embed next. Decisions (user, 2026-10-27):
the dōjō map is computed by the DAEMON and pushed with the graph sync; complexity comes from LOCAL QLTY.

**Done (develop, pushed; daemon installed locally; live DB reconciled):**
- Code map in the app (`@sensei/codemap`): L0 modules → L1 files by role → L2 symbols; lenses Structure,
  Wheel, Cycles, Matrix, Zones, Hidden coupling (co-change from `commit_files`), Complexity (local qlty,
  `measure_complexity` task, `sensei.file_complexity`). `test:inspect -- codemap` green for every lens.
- World and Atlas retired (routes, state, `/diagrams/world`, `/workspaces/{id}/graph`).
- `module_of`: container folders split one level deeper, files directly in one stay in it (185 modules).
- `crate::qlty` shared by the quality metric and complexity.
- #133 one rule + preference sync (live check after the next release deploys `dojo.user_preferences`).

**Next — the dōjō embed:**
1. Daemon: compute the Code map L0 PER REPOSITORY + branch (today's queries are workspace-scoped; add a
   repository scope) and push it with `dojo_sync`'s graph push.
2. dōjō: a table for the pushed L0 (per repository, branch; RLS = `can_read_repository`), SQL functions for
   L1/L2 over `graph_nodes`/`graph_edges`, `/v1` routes, a port for `@sensei/codemap`, and a Code health tab
   on the dōjō workspace page (repository picker).

**Known:** Keychain tests fail on rebuilt test binaries (#282, environmental). After a DDL change run
`make test-template`; reconcile live with dbd. fmt + workspace clippy before every Rust commit.
